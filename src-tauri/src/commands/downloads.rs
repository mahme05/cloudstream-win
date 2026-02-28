// commands/downloads.rs
// IPC handlers for download management.
// Downloads run as background tokio tasks and emit progress events to the frontend.

use tauri::{State, Emitter, AppHandle};
use serde::Deserialize;
use chrono::Utc;
use uuid::Uuid;
use crate::AppState;
use crate::db::Download;

#[tauri::command]
pub async fn get_downloads(
    state: State<'_, AppState>,
) -> Result<Vec<Download>, String> {
    state.db.get_downloads().await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct StartDownloadPayload {
    pub media_id: String,
    pub plugin_id: String,
    pub title: String,
    pub episode_title: Option<String>,
    pub url: String,
    pub save_path: String,
}

/// Start a download
/// React call: await invoke("start_download", { url: "...", savePath: "C:/...", title: "..." })
/// Progress events are emitted as: listen("download-progress", (event) => { ... })
#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: StartDownloadPayload,
) -> Result<String, String> {
    let download = Download {
        id: Uuid::new_v4().to_string(),
        media_id: payload.media_id,
        plugin_id: payload.plugin_id,
        title: payload.title,
        episode_title: payload.episode_title,
        url: payload.url.clone(),
        save_path: payload.save_path.clone(),
        status: "downloading".to_string(),
        progress: 0.0,
        created_at: Utc::now(),
    };
    
    state.db.add_download(&download).await.map_err(|e| e.to_string())?;
    
    let download_id = download.id.clone();
    let save_path = payload.save_path.clone();
    let url = payload.url.clone();
    
    // Spawn a background task for downloading
    // This doesn't block the UI — it runs concurrently
    tokio::spawn(async move {
        if let Err(e) = download_file(&app, &download_id, &url, &save_path).await {
            log::error!("Download failed: {}", e);
            // Emit failure event to frontend
            let _ = app.emit("download-failed", serde_json::json!({
                "id": download_id,
                "error": e.to_string()
            }));
        }
    });
    
    Ok(download.id)
}

/// Cancel an in-progress download
#[tauri::command]
pub async fn cancel_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<(), String> {
    state.db
        .update_download_progress(&download_id, 0.0, "cancelled")
        .await
        .map_err(|e| e.to_string())
}

/// Background download task using reqwest with streaming
async fn download_file(
    app: &AppHandle,
    download_id: &str,
    url: &str,
    save_path: &str,
) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    
    let mut file = tokio::fs::File::create(save_path).await?;
    let mut stream = response.bytes_stream();
    
    // Stream bytes to disk while emitting progress events
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        
        if total_size > 0 {
            let progress = downloaded as f64 / total_size as f64;
            
            // Emit progress event — React listens with: listen("download-progress", ...)
            let _ = app.emit("download-progress", serde_json::json!({
                "id": download_id,
                "progress": progress,
                "downloaded": downloaded,
                "total": total_size
            }));
        }
    }
    
    // Emit completion event
    let _ = app.emit("download-complete", serde_json::json!({
        "id": download_id
    }));
    
    Ok(())
}
