use crate::auth::Authed;
use crate::error::ApiError;
use crate::state::AppState;
use axum::{Json, extract::State};
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};

/// The wire shape of a card. It mirrors `KanbanCard` in the core crate.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncCard {
    pub name_with_owner: String,
    pub column: String,
    /// Missing from cards sent by a client built before notes existed, and
    /// left out of the response when there are none, exactly like the core.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRequest {
    pub cards: Vec<SyncCard>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub cards: Vec<SyncCard>,
}

pub async fn sync(
    State(state): State<AppState>,
    authed: Authed,
    Json(body): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, ApiError> {
    let mut conn = state.db.get()?;
    let tx = conn.transaction()?;
    store_newer(&tx, &authed.sub, &body.cards)?;
    let cards = load_all(&tx, &authed.sub)?;
    tx.commit()?;
    Ok(Json(SyncResponse { cards }))
}

/// Per-card last-writer-wins: only when the incoming `updated_at` is strictly
/// newer are the stored column, notes and `updated_at` replaced (absent notes
/// clear the stored ones); `created_at` keeps its first value. A card this
/// user has never synced is inserted as it comes.
fn store_newer(tx: &Transaction<'_>, sub: &str, cards: &[SyncCard]) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare(
        "INSERT INTO manifest_cards (sub, name_with_owner, column_id, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(sub, name_with_owner) DO UPDATE SET
            column_id  = excluded.column_id,
            notes      = excluded.notes,
            updated_at = excluded.updated_at
         WHERE excluded.updated_at > manifest_cards.updated_at",
    )?;
    for card in cards {
        stmt.execute(rusqlite::params![
            sub,
            card.name_with_owner,
            card.column,
            card.notes,
            card.created_at,
            card.updated_at,
        ])?;
    }
    Ok(())
}

/// Every card stored for this user: the board the client merges into its own.
fn load_all(tx: &Transaction<'_>, sub: &str) -> rusqlite::Result<Vec<SyncCard>> {
    let mut stmt = tx.prepare(
        "SELECT name_with_owner, column_id, notes, created_at, updated_at
         FROM manifest_cards WHERE sub = ?1",
    )?;
    let rows = stmt.query_map(rusqlite::params![sub], |row| {
        Ok(SyncCard {
            name_with_owner: row.get(0)?,
            column: row.get(1)?,
            notes: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::{SyncCard, SyncRequest, sync};
    use crate::auth::Authed;
    use crate::google::GoogleVerifier;
    use crate::state::AppState;
    use crate::test_support::{REPO, TempDb, USER};
    use axum::{Json, extract::State};

    fn card(notes: Option<&str>, updated_at: i64) -> SyncCard {
        SyncCard {
            name_with_owner: REPO.to_string(),
            column: "backlog".to_string(),
            notes: notes.map(str::to_string),
            created_at: 0,
            updated_at,
        }
    }

    fn state_for(db: &TempDb) -> AppState {
        AppState {
            db: db.pool(),
            google: GoogleVerifier::new("test-client".to_string()),
            session_ttl_secs: 0,
        }
    }

    /// One sync round trip for `USER` with a single card, returning the
    /// card the server holds afterwards.
    async fn sync_one(state: &AppState, card: SyncCard) -> SyncCard {
        let Json(response) = sync(
            State(state.clone()),
            Authed {
                sub: USER.to_string(),
            },
            Json(SyncRequest { cards: vec![card] }),
        )
        .await
        .unwrap();
        response
            .cards
            .into_iter()
            .find(|stored| stored.name_with_owner == REPO)
            .unwrap()
    }

    #[tokio::test]
    async fn notes_are_stored_and_echoed_back() {
        let db = TempDb::new("store");
        let state = state_for(&db);

        let stored = sync_one(&state, card(Some("Ask for feedback"), 10)).await;

        assert_eq!(stored.notes.as_deref(), Some("Ask for feedback"));
    }

    #[tokio::test]
    async fn a_newer_card_replaces_the_notes() {
        let db = TempDb::new("newer");
        let state = state_for(&db);
        sync_one(&state, card(Some("old"), 10)).await;
        let newer = SyncCard {
            created_at: 99,
            ..card(Some("new"), 20)
        };

        let stored = sync_one(&state, newer).await;

        assert_eq!(stored.notes.as_deref(), Some("new"));
        assert_eq!(stored.created_at, 0, "created_at keeps its first value");
    }

    #[tokio::test]
    async fn a_card_with_the_same_updated_at_changes_nothing() {
        let db = TempDb::new("same");
        let state = state_for(&db);
        sync_one(&state, card(Some("first"), 10)).await;

        let stored = sync_one(&state, card(Some("second"), 10)).await;

        assert_eq!(stored.notes.as_deref(), Some("first"));
    }

    #[tokio::test]
    async fn a_card_migrated_from_before_notes_accepts_them() {
        let db = TempDb::new("migrated").with_pre_notes_schema();
        let state = state_for(&db);

        let stored = sync_one(&state, card(Some("added after the upgrade"), 10)).await;

        assert_eq!(stored.notes.as_deref(), Some("added after the upgrade"));
        assert_eq!(
            stored.created_at, 1,
            "the migrated row was updated in place, not replaced"
        );
    }

    #[tokio::test]
    async fn an_older_card_leaves_the_notes_alone() {
        let db = TempDb::new("older");
        let state = state_for(&db);
        sync_one(&state, card(Some("kept"), 20)).await;

        let stored = sync_one(&state, card(None, 10)).await;

        assert_eq!(stored.notes.as_deref(), Some("kept"));
        assert_eq!(stored.updated_at, 20);
    }

    #[tokio::test]
    async fn a_newer_card_without_notes_clears_them() {
        // Whole-card last-writer-wins, the same trade-off the core accepts.
        let db = TempDb::new("clear");
        let state = state_for(&db);
        sync_one(&state, card(Some("gone"), 10)).await;

        let stored = sync_one(&state, card(None, 20)).await;

        assert_eq!(stored.notes, None);
    }

    #[test]
    fn a_request_without_the_notes_key_still_parses() {
        let from_an_old_client = r#"{
            "nameWithOwner": "leo/some-repo",
            "column": "backlog",
            "createdAt": 1,
            "updatedAt": 2
        }"#;

        let parsed: SyncCard = serde_json::from_str(from_an_old_client).unwrap();

        assert_eq!(parsed.notes, None);
    }

    #[test]
    fn a_card_without_notes_is_sent_without_the_key() {
        let json = serde_json::to_string(&card(None, 2)).unwrap();

        assert!(!json.contains("notes"), "unexpected notes key in {json}");
    }
}
