// commands/streaming.rs
// IPC handlers for streaming and watch history.

use tauri::State;
use serde::Deserialize;
use chrono::Utc;
use uuid::Uuid;
use crate::AppState;
use crate::db::WatchHistory;

/// Open the video player with a stream URL
/// This triggers the mpv player in the player module
/// React call: await invoke("play_stream", { url: "...", title: "..." })
#[tauri::command]
pub async fn play_stream(
    url: String,
    title: String,
    headers: Option<std::collections::HashMap<String, String>>,
) -> Result<(), String> {
    log::info!("Playing stream: {} ({})", title, url);
    // TODO: Wire up to the mpv player
    // crate::player::play(&url, &title, headers).await.map_err(|e| e.to_string())
    Ok(())
}

/// Get watch history
/// React call: await invoke("get_watch_history", { limit: 20 })
#[tauri::command]
pub async fn get_watch_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<WatchHistory>, String> {
    state.db
        .get_watch_history(limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct UpdateProgressPayload {
    pub media_id: String,
    pub plugin_id: String,
    pub episode_id: Option<String>,
    pub title: String,
    pub episode_title: Option<String>,
    pub progress_seconds: i64,
    pub duration_seconds: i64,
}

/// Update watch progress (called periodically while watching)
/// React call: await invoke("update_watch_progress", { mediaId: "...", ... })
#[tauri::command]
pub async fn update_watch_progress(
    state: State<'_, AppState>,
    payload: UpdateProgressPayload,
) -> Result<(), String> {
    let history = WatchHistory {
        id: format!("{}-{}", payload.media_id, payload.episode_id.as_deref().unwrap_or("movie")),
        media_id: payload.media_id,
        plugin_id: payload.plugin_id,
        episode_id: payload.episode_id,
        title: payload.title,
        episode_title: payload.episode_title,
        progress_seconds: payload.progress_seconds,
        duration_seconds: payload.duration_seconds,
        watched_at: Utc::now(),
    };
    
    state.db
        .upsert_watch_progress(&history)
        .await
        .map_err(|e| e.to_string())
}
