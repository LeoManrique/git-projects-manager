use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init(database_url: &str) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(database_url).with_init(|c| {
        c.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
    });
    let pool = Pool::builder().max_size(8).build(manager)?;
    migrate(&pool)?;
    Ok(pool)
}

/// Bring the database to the current schema, whether it is brand new or was
/// created by an earlier release. Every step is safe to repeat, so this runs
/// on each start.
fn migrate(pool: &DbPool) -> Result<()> {
    let conn = pool.get()?;
    create_tables(&conn)?;
    add_notes_column(&conn)?;
    Ok(())
}

/// The schema a fresh database gets. `IF NOT EXISTS` leaves an existing
/// table untouched, so a column added after the first release needs its own
/// step below.
fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            sub        TEXT PRIMARY KEY,
            email      TEXT,
            name       TEXT,
            created_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS sessions (
            token      TEXT PRIMARY KEY,
            sub        TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_sessions_sub ON sessions(sub);
         CREATE TABLE IF NOT EXISTS manifest_cards (
            sub             TEXT NOT NULL,
            name_with_owner TEXT NOT NULL,
            column_id       TEXT NOT NULL,
            notes           TEXT,
            created_at      INTEGER NOT NULL,
            updated_at      INTEGER NOT NULL,
            PRIMARY KEY (sub, name_with_owner)
         );
         CREATE INDEX IF NOT EXISTS idx_manifest_sub ON manifest_cards(sub);",
    )
}

/// Per-card notes arrived after the first deployment, so a database from
/// before then lacks the column. There is no `ADD COLUMN IF NOT EXISTS`, so
/// look before altering.
fn add_notes_column(conn: &Connection) -> rusqlite::Result<()> {
    if column_exists(conn, "manifest_cards", "notes")? {
        return Ok(());
    }
    conn.execute_batch("ALTER TABLE manifest_cards ADD COLUMN notes TEXT")
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let matches: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
        rusqlite::params![table, column],
        |row| row.get(0),
    )?;
    Ok(matches > 0)
}

#[cfg(test)]
mod tests {
    use super::column_exists;
    use crate::test_support::{REPO, TempDb, USER};
    use rusqlite::Connection;

    fn has_notes_column(db: &TempDb) -> bool {
        let conn = Connection::open(db.path()).unwrap();
        column_exists(&conn, "manifest_cards", "notes").unwrap()
    }

    #[test]
    fn a_fresh_database_gets_the_notes_column() {
        let db = TempDb::new("fresh");

        let _pool = db.pool();

        assert!(has_notes_column(&db));
    }

    #[test]
    fn a_database_from_before_notes_gains_the_column() {
        let db = TempDb::new("legacy").with_pre_notes_schema();

        let pool = db.pool();

        assert!(has_notes_column(&db));
        let notes: Option<String> = pool
            .get()
            .unwrap()
            .query_row(
                "SELECT notes FROM manifest_cards WHERE sub = ?1 AND name_with_owner = ?2",
                rusqlite::params![USER, REPO],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(notes, None, "an existing card survives with no notes");
    }

    #[test]
    fn migrating_twice_is_harmless() {
        let db = TempDb::new("twice");
        let _first = db.pool();

        let _second = db.pool();

        assert!(has_notes_column(&db));
    }
}
