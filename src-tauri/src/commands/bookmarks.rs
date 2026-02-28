// commands/bookmarks.rs
// IPC handlers for bookmark operations.

use tauri::State;
use serde::Deserialize;
use chrono::Utc;
use uuid::Uuid;
use crate::AppState;
use crate::db::Bookmark;

#[tauri::command]
pub async fn get_bookmarks(
    state: State<'_, AppState>,
) -> Result<Vec<Bookmark>, String> {
    state.db.get_bookmarks().await.map_err(|e| e.to_string())
}

// Payload sent from React when adding a bookmark
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddBookmarkPayload {
    pub media_id: String,
    pub plugin_id: String,
    pub title: String,
    pub poster_url: Option<String>,
    pub media_type: String,
}

/// Add a bookmark
/// React call: await invoke("add_bookmark", { mediaId: "...", pluginId: "...", title: "...", ... })
#[tauri::command]
pub async fn add_bookmark(
    state: State<'_, AppState>,
    payload: AddBookmarkPayload,
) -> Result<Bookmark, String> {
    let bookmark = Bookmark {
        id: Uuid::new_v4().to_string(),
        media_id: payload.media_id,
        plugin_id: payload.plugin_id,
        title: payload.title,
        poster_url: payload.poster_url,
        media_type: payload.media_type,
        created_at: Utc::now(),
    };
    
    state.db.add_bookmark(&bookmark).await.map_err(|e| e.to_string())?;
    Ok(bookmark)
}

/// Remove a bookmark
/// React call: await invoke("remove_bookmark", { mediaId: "...", pluginId: "..." })
#[tauri::command]
pub async fn remove_bookmark(
    state: State<'_, AppState>,
    media_id: String,
    plugin_id: String,
) -> Result<(), String> {
    state.db
        .remove_bookmark(&media_id, &plugin_id)
        .await
        .map_err(|e| e.to_string())
}
