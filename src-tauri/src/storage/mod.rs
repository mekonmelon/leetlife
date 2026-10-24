//! SQLite persistence via SQLx. `Db` owns the pool; `repo` holds every query so
//! the engine never writes SQL inline. All repo functions take
//! `&mut SqliteConnection`, so the same code runs inside a transaction
//! (`&mut *tx`) or on a pooled connection (`&mut *conn`).
pub mod repo;
pub mod seed;

use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Sqlite, Transaction};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Opens (creating if needed) the database file with WAL + NORMAL sync:
    /// durable enough for a local game, and much faster than FULL.
    pub async fn open(path: &Path) -> Result<Db> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new().max_connections(4).min_connections(1).connect_with(opts).await?;
        Ok(Db { pool })
    }

    /// Private in-memory database (tests, benches). Single connection so the
    /// memory DB is never dropped mid-run.
    pub async fn in_memory() -> Result<Db> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        // One connection, never reaped: a `sqlite::memory:` database only exists
        // for as long as its connection does.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(opts)
            .await?;
        Ok(Db { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn conn(&self) -> Result<sqlx::pool::PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }

    pub async fn begin(&self) -> Result<Transaction<'static, Sqlite>> {
        Ok(self.pool.begin().await?)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
