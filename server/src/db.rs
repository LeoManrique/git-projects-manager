use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

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

fn migrate(pool: &DbPool) -> Result<()> {
    let conn = pool.get()?;
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
            created_at      INTEGER NOT NULL,
            updated_at      INTEGER NOT NULL,
            PRIMARY KEY (sub, name_with_owner)
         );
         CREATE INDEX IF NOT EXISTS idx_manifest_sub ON manifest_cards(sub);",
    )?;
    Ok(())
}
