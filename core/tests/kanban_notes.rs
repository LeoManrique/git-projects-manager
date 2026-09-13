//! Per-card notes in the kanban store: the free text a card carries so the
//! user remembers what to do next. Notes must survive the repo
//! reconciliation that runs on every refresh, follow one normalization rule
//! (trimmed, blank means none), and stay out of the JSON when absent so a
//! `kanban_v2.json` written before notes existed still loads.

use gpm_core::infrastructure::kanban_store::KanbanManager;
use std::path::{Path, PathBuf};
use std::time::Duration;

const REPO: &str = "leo/some-repo";

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("gpm-kanban-notes-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// A store with one card in the backlog, the way a first refresh leaves it.
fn manager_with_one_card(root: &Path) -> KanbanManager {
    let manager = KanbanManager::new(root);
    manager.sync_with_repos(vec![REPO.to_string()]).unwrap();
    manager
}

fn notes_of(manager: &KanbanManager) -> Option<String> {
    manager.load().unwrap().cards[REPO].notes.clone()
}

#[test]
fn update_notes_persists_and_bumps_updated_at() {
    let root = temp_root("persist");
    let manager = manager_with_one_card(&root);
    let before = manager.load().unwrap().cards[REPO].updated_at;
    std::thread::sleep(Duration::from_millis(2));

    let state = manager
        .update_notes(REPO, Some("Ask for feedback".to_string()))
        .unwrap();

    let card = &state.cards[REPO];
    assert_eq!(card.notes.as_deref(), Some("Ask for feedback"));
    assert!(
        card.updated_at > before,
        "updated_at must move so the edit wins the cloud merge"
    );
    // A fresh manager reads the file back, so the notes really hit the disk.
    assert_eq!(
        notes_of(&KanbanManager::new(&root)).as_deref(),
        Some("Ask for feedback")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn update_notes_trims_surrounding_whitespace() {
    let root = temp_root("trim");
    let manager = manager_with_one_card(&root);

    manager
        .update_notes(REPO, Some("  keep the middle\n".to_string()))
        .unwrap();

    assert_eq!(notes_of(&manager).as_deref(), Some("keep the middle"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn update_notes_clears_when_the_text_is_blank() {
    let root = temp_root("clear");
    let manager = manager_with_one_card(&root);
    manager
        .update_notes(REPO, Some("something".to_string()))
        .unwrap();

    manager
        .update_notes(REPO, Some("   \n ".to_string()))
        .unwrap();

    assert_eq!(notes_of(&manager), None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn update_notes_ignores_an_unknown_card() {
    let root = temp_root("unknown");
    let manager = manager_with_one_card(&root);

    let state = manager
        .update_notes("nobody/nothing", Some("lost".to_string()))
        .unwrap();

    assert_eq!(state.cards.len(), 1);
    assert!(!state.cards.contains_key("nobody/nothing"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn sync_with_repos_keeps_notes_of_existing_cards() {
    let root = temp_root("reconcile");
    let manager = manager_with_one_card(&root);
    manager
        .update_notes(REPO, Some("still here".to_string()))
        .unwrap();

    let state = manager
        .sync_with_repos(vec![REPO.to_string(), "leo/new-repo".to_string()])
        .unwrap();

    assert_eq!(state.cards[REPO].notes.as_deref(), Some("still here"));
    assert_eq!(
        state.cards["leo/new-repo"].notes, None,
        "a new card starts without notes"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_card_without_notes_serializes_without_the_key() {
    let root = temp_root("no-key");
    manager_with_one_card(&root);

    let content = std::fs::read_to_string(root.join("kanban_v2.json")).unwrap();

    assert!(
        !content.contains("\"notes\""),
        "unexpected notes key in:\n{content}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn state_without_notes_field_still_loads() {
    let root = temp_root("legacy");
    let written_before_notes_existed = r#"{
  "version": 2,
  "cards": {
    "leo/some-repo": {
      "nameWithOwner": "leo/some-repo",
      "column": "done",
      "createdAt": 1,
      "updatedAt": 2
    }
  }
}"#;
    std::fs::write(root.join("kanban_v2.json"), written_before_notes_existed).unwrap();

    let state = KanbanManager::new(&root).load().unwrap();

    let card = &state.cards[REPO];
    assert_eq!(card.column, "done");
    assert_eq!(card.notes, None);
    let _ = std::fs::remove_dir_all(&root);
}
