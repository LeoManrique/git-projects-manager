use crate::auth::Authed;
use crate::error::ApiError;
use crate::state::AppState;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncCard {
    pub name_with_owner: String,
    pub column: String,
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

    // Per-card LWW: only overwrite when incoming.updated_at is strictly newer.
    {
        let mut stmt = tx.prepare(
            "INSERT INTO manifest_cards (sub, name_with_owner, column_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(sub, name_with_owner) DO UPDATE SET
                column_id  = excluded.column_id,
                updated_at = excluded.updated_at
             WHERE excluded.updated_at > manifest_cards.updated_at",
        )?;
        for card in &body.cards {
            stmt.execute(rusqlite::params![
                authed.sub,
                card.name_with_owner,
                card.column,
                card.created_at,
                card.updated_at,
            ])?;
        }
    }

    let merged: Vec<SyncCard> = {
        let mut stmt = tx.prepare(
            "SELECT name_with_owner, column_id, created_at, updated_at
             FROM manifest_cards WHERE sub = ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![authed.sub], |r| {
            Ok(SyncCard {
                name_with_owner: r.get(0)?,
                column: r.get(1)?,
                created_at: r.get(2)?,
                updated_at: r.get(3)?,
            })
        })?;
        rows.collect::<Result<_, _>>()?
    };

    tx.commit()?;
    Ok(Json(SyncResponse { cards: merged }))
}
