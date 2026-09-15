use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::{str::FromStr, time::Duration};

/// A single task. Rendered by `templates/tasks/row.html`.
#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub done: bool,
}

/// Database-backed state. Handlers and templates only see these methods,
/// so the storage engine can change without touching them.
#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
}

impl AppState {
    /// Open (creating if missing) the SQLite database at `url`
    /// (e.g. `sqlite://tasks.db`) and apply pending migrations.
    pub async fn connect(url: &str) -> sqlx::Result<Self> {
        let opts = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;
        Self::from_pool(pool).await
    }

    /// Fresh in-memory database, isolated per call. Used by tests.
    pub async fn in_memory() -> sqlx::Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        // Every connection to `:memory:` is a separate database, so keep exactly one.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(opts)
            .await?;
        Self::from_pool(pool).await
    }

    async fn from_pool(pool: SqlitePool) -> sqlx::Result<Self> {
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Insert demo rows if the table is empty. Returns how many were added.
    pub async fn seed_if_empty(&self) -> sqlx::Result<u64> {
        if self.count().await? > 0 {
            return Ok(0);
        }
        self.create("Read the htmx docs").await?;
        self.create("Write an Askama template").await?;
        let id = self.create("Ship it").await?;
        self.toggle(id).await?;
        Ok(3)
    }

    /// Insert a task and return its id. Title is trimmed; callers validate emptiness.
    pub async fn create(&self, title: &str) -> sqlx::Result<i64> {
        let result = sqlx::query("INSERT INTO tasks (title) VALUES (?)")
            .bind(title.trim())
            .execute(&self.pool)
            .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<Task>> {
        sqlx::query_as::<_, Task>("SELECT id, title, done FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Flip `done`. Returns the updated task, or None if it does not exist.
    pub async fn toggle(&self, id: i64) -> sqlx::Result<Option<Task>> {
        sqlx::query_as::<_, Task>(
            "UPDATE tasks SET done = NOT done WHERE id = ? RETURNING id, title, done",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Remove a task. Returns true if something was removed.
    pub async fn delete(&self, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Tasks whose title contains `query` (case-insensitive). Empty query returns all.
    pub async fn search(&self, query: &str) -> sqlx::Result<Vec<Task>> {
        let pattern = format!("%{}%", escape_like(query.trim()));
        sqlx::query_as::<_, Task>(
            "SELECT id, title, done FROM tasks \
             WHERE title LIKE ? ESCAPE '\\' \
             ORDER BY id",
        )
        .bind(pattern)
        .fetch_all(&self.pool)
        .await
    }

    /// Number of tasks not yet done.
    pub async fn remaining(&self) -> sqlx::Result<usize> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE done = 0")
            .fetch_one(&self.pool)
            .await?;
        Ok(n as usize)
    }

    async fn count(&self) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&self.pool)
            .await
    }
}

/// Escape LIKE metacharacters so user input matches literally.
fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_toggle_delete_roundtrip() {
        let s = AppState::in_memory().await.unwrap();
        let id = s.create("  hello  ").await.unwrap();
        assert_eq!(s.get(id).await.unwrap().unwrap().title, "hello");
        assert_eq!(s.remaining().await.unwrap(), 1);
        assert!(s.toggle(id).await.unwrap().unwrap().done);
        assert_eq!(s.remaining().await.unwrap(), 0);
        assert!(s.delete(id).await.unwrap());
        assert!(!s.delete(id).await.unwrap());
        assert!(s.get(id).await.unwrap().is_none());
        assert!(s.toggle(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn search_is_case_insensitive_and_escapes_wildcards() {
        let s = AppState::in_memory().await.unwrap();
        s.create("Buy Milk").await.unwrap();
        s.create("Walk dog").await.unwrap();
        s.create("100% done").await.unwrap();
        assert_eq!(s.search("milk").await.unwrap().len(), 1);
        assert_eq!(s.search("").await.unwrap().len(), 3);
        assert_eq!(s.search("zzz").await.unwrap().len(), 0);
        assert_eq!(
            s.search("%").await.unwrap().len(),
            1,
            "% must match literally"
        );
        assert_eq!(
            s.search("_").await.unwrap().len(),
            0,
            "_ must match literally"
        );
    }

    #[tokio::test]
    async fn in_memory_databases_are_isolated() {
        let a = AppState::in_memory().await.unwrap();
        let b = AppState::in_memory().await.unwrap();
        a.create("only in a").await.unwrap();
        assert_eq!(a.search("").await.unwrap().len(), 1);
        assert_eq!(b.search("").await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn seed_only_runs_once() {
        let s = AppState::in_memory().await.unwrap();
        assert_eq!(s.seed_if_empty().await.unwrap(), 3);
        assert_eq!(s.seed_if_empty().await.unwrap(), 0);
        assert_eq!(s.remaining().await.unwrap(), 2);
    }
}
