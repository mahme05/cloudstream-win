// db/mod.rs
// Everything database-related lives here.
// We use SQLite (a file-based database, no server needed) via sqlx.
// 
// Think of sqlx like an ORM but you write plain SQL — 
// it just maps results to Rust structs for you.

use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

// ─────────────────────────────────────────────
// DATA MODELS
// These are Rust structs that map to database rows.
// #[derive(Serialize, Deserialize)] lets them be converted to/from JSON automatically.
// ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Bookmark {
    pub id: String,
    pub media_id: String,       // ID from the plugin (e.g. "tt1234567")
    pub plugin_id: String,      // Which plugin this came from
    pub title: String,
    pub poster_url: Option<String>,
    pub media_type: String,     // "movie" | "show" | "anime"
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct WatchHistory {
    pub id: String,
    pub media_id: String,
    pub plugin_id: String,
    pub episode_id: Option<String>,
    pub title: String,
    pub episode_title: Option<String>,
    pub progress_seconds: i64,  // How far into the episode (in seconds)
    pub duration_seconds: i64,  // Total duration
    pub watched_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Download {
    pub id: String,
    pub media_id: String,
    pub plugin_id: String,
    pub title: String,
    pub episode_title: Option<String>,
    pub url: String,
    pub save_path: String,
    pub status: String,         // "pending" | "downloading" | "done" | "failed"
    pub progress: f64,          // 0.0 to 1.0
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────
// DATABASE WRAPPER
// Wraps the connection pool and exposes clean methods.
// ─────────────────────────────────────────────

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create (or open) the SQLite database at the given path.
    /// Runs migrations to create tables if they don't exist.
    pub async fn new(path: &str) -> Result<Self> {
        // The connection string format for SQLite
        // On Windows, backslashes in paths break the sqlite:// URI — use forward slashes
        // Windows paths like C:\foo\bar must become sqlite:///C:/foo/bar
        // (three slashes = absolute path in the sqlite:// URI scheme)
        let normalized = path.replace('\\', "/");
        let connection_string = format!("sqlite:///{}?mode=rwc", normalized);
        
        let pool = SqlitePool::connect(&connection_string).await?;
        
        let db = Self { pool };
        db.run_migrations().await?;
        
        Ok(db)
    }
    
    /// Create all tables if they don't already exist.
    /// This is a simple migration system — for a real app you'd use sqlx migrate files.
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY,
                media_id TEXT NOT NULL,
                plugin_id TEXT NOT NULL,
                title TEXT NOT NULL,
                poster_url TEXT,
                media_type TEXT NOT NULL DEFAULT 'movie',
                created_at TEXT NOT NULL,
                UNIQUE(media_id, plugin_id)
            )
        "#).execute(&self.pool).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS watch_history (
                id TEXT PRIMARY KEY,
                media_id TEXT NOT NULL,
                plugin_id TEXT NOT NULL,
                episode_id TEXT,
                title TEXT NOT NULL,
                episode_title TEXT,
                progress_seconds INTEGER NOT NULL DEFAULT 0,
                duration_seconds INTEGER NOT NULL DEFAULT 0,
                watched_at TEXT NOT NULL
            )
        "#).execute(&self.pool).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS downloads (
                id TEXT PRIMARY KEY,
                media_id TEXT NOT NULL,
                plugin_id TEXT NOT NULL,
                title TEXT NOT NULL,
                episode_title TEXT,
                url TEXT NOT NULL,
                save_path TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                progress REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL
            )
        "#).execute(&self.pool).await?;

        log::info!("Database migrations complete");
        Ok(())
    }

    // ─── BOOKMARK METHODS ───
    
    pub async fn get_bookmarks(&self) -> Result<Vec<Bookmark>> {
        let bookmarks = sqlx::query_as::<_, Bookmark>(
            "SELECT * FROM bookmarks ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(bookmarks)
    }
    
    pub async fn add_bookmark(&self, bookmark: &Bookmark) -> Result<()> {
        sqlx::query(r#"
            INSERT OR IGNORE INTO bookmarks 
            (id, media_id, plugin_id, title, poster_url, media_type, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&bookmark.id)
        .bind(&bookmark.media_id)
        .bind(&bookmark.plugin_id)
        .bind(&bookmark.title)
        .bind(&bookmark.poster_url)
        .bind(&bookmark.media_type)
        .bind(bookmark.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    pub async fn remove_bookmark(&self, media_id: &str, plugin_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM bookmarks WHERE media_id = ? AND plugin_id = ?")
            .bind(media_id)
            .bind(plugin_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ─── WATCH HISTORY METHODS ───
    
    pub async fn get_watch_history(&self, limit: i64) -> Result<Vec<WatchHistory>> {
        let history = sqlx::query_as::<_, WatchHistory>(
            "SELECT * FROM watch_history ORDER BY watched_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(history)
    }
    
    pub async fn upsert_watch_progress(&self, history: &WatchHistory) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO watch_history 
            (id, media_id, plugin_id, episode_id, title, episode_title, 
             progress_seconds, duration_seconds, watched_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                progress_seconds = excluded.progress_seconds,
                watched_at = excluded.watched_at
        "#)
        .bind(&history.id)
        .bind(&history.media_id)
        .bind(&history.plugin_id)
        .bind(&history.episode_id)
        .bind(&history.title)
        .bind(&history.episode_title)
        .bind(history.progress_seconds)
        .bind(history.duration_seconds)
        .bind(history.watched_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ─── DOWNLOAD METHODS ───
    
    pub async fn get_downloads(&self) -> Result<Vec<Download>> {
        let downloads = sqlx::query_as::<_, Download>(
            "SELECT * FROM downloads ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(downloads)
    }
    
    pub async fn add_download(&self, download: &Download) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO downloads 
            (id, media_id, plugin_id, title, episode_title, url, save_path, status, progress, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&download.id)
        .bind(&download.media_id)
        .bind(&download.plugin_id)
        .bind(&download.title)
        .bind(&download.episode_title)
        .bind(&download.url)
        .bind(&download.save_path)
        .bind(&download.status)
        .bind(download.progress)
        .bind(download.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    pub async fn update_download_progress(&self, id: &str, progress: f64, status: &str) -> Result<()> {
        sqlx::query("UPDATE downloads SET progress = ?, status = ? WHERE id = ?")
            .bind(progress)
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
