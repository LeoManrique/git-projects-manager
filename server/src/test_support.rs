//! Helpers shared by the unit tests. The server is a binary crate, so its
//! tests live next to the code they check and meet here.

use crate::db::{self, DbPool};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const USER: &str = "google-sub-1";
pub const REPO: &str = "leo/some-repo";

/// A throwaway directory holding one database file. It is removed on drop,
/// so a failing test leaves nothing behind.
pub struct TempDb {
    dir: PathBuf,
}

impl TempDb {
    /// A fresh directory. `tag` names it for a human, the process id keeps
    /// concurrent `cargo test` runs apart, and a counter keeps the tests of
    /// this process apart even when two of them reuse a tag.
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("gpm-sync-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    /// Write `manifest_cards` the way the first release created it, before
    /// the `notes` column existed, holding one card of `USER` in "doing".
    pub fn with_pre_notes_schema(self) -> Self {
        let conn = Connection::open(self.path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE manifest_cards (
                sub             TEXT NOT NULL,
                name_with_owner TEXT NOT NULL,
                column_id       TEXT NOT NULL,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL,
                PRIMARY KEY (sub, name_with_owner)
             )",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO manifest_cards VALUES (?1, ?2, 'doing', 1, 2)",
            rusqlite::params![USER, REPO],
        )
        .unwrap();
        self
    }

    pub fn path(&self) -> String {
        self.dir.join("sync.db").to_string_lossy().into_owned()
    }

    /// Open the pool the way `main` does, migrations included.
    pub fn pool(&self) -> DbPool {
        db::init(&self.path()).unwrap()
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
