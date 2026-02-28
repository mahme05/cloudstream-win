// commands/downloads.rs
// IPC handlers for download management.
// Downloads run as background tokio tasks. Cancellation is handled via
// CancellationToken stored in a global map keyed by download ID.

use tauri::{AppHandle, Emitter, State};
use serde::Deserialize;
use chrono::Utc;
use uuid::Uuid;
use std::{collections::HashMap, sync::Mutex};
use tokio_util::sync::CancellationToken;
use crate::{AppState, db::Download};

// ── Global cancellation token registry ───────────────────────────────────────
// Wrapped in a Mutex<HashMap> so start_download and cancel_download
// can safely share it across async tasks.

type CancelMap = Mutex<HashMap<String, CancellationToken>>;

// Stored as Tauri state, initialised in lib.rs setup.
pub struct DownloadManager(pub CancelMap);

impl DownloadManager {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_downloads(
    state: State<'_, AppState>,
) -> Result<Vec<Download>, String> {
    state.db.get_downloads().await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadPayload {
    pub media_id:      String,
    pub plugin_id:     String,
    pub title:         String,
    pub episode_title: Option<String>,
    pub url:           String,
    pub save_path:     String,
}

/// Start a download. Progress events are emitted as "download-progress".
/// React: await invoke("start_download", { url, savePath, title, ... })
#[tauri::command]
pub async fn start_download(
    app:     AppHandle,
    state:   State<'_, AppState>,
    manager: State<'_, DownloadManager>,
    payload: StartDownloadPayload,
) -> Result<String, String> {
    let download = Download {
        id:            Uuid::new_v4().to_string(),
        media_id:      payload.media_id,
        plugin_id:     payload.plugin_id,
        title:         payload.title,
        episode_title: payload.episode_title,
        url:           payload.url.clone(),
        save_path:     payload.save_path.clone(),
        status:        "downloading".to_string(),
        progress:      0.0,
        created_at:    Utc::now(),
    };

    state.db.add_download(&download).await.map_err(|e| e.to_string())?;

    let download_id  = download.id.clone();
    let cancel_token = CancellationToken::new();

    // Register the token so cancel_download() can reach it
    manager.0.lock().unwrap().insert(download_id.clone(), cancel_token.clone());

    let url        = payload.url.clone();
    let save_path  = payload.save_path.clone();
    let id_clone   = download_id.clone();
    let app_clone  = app.clone();

    tokio::spawn(async move {
        let result = download_file(
            &app_clone,
            &id_clone,
            &url,
            &save_path,
            cancel_token,
        )
        .await;

        match result {
            Ok(())               => { /* completion event already emitted inside */ }
            Err(DownloadError::Cancelled) => {
                log::info!("[download] {} cancelled", id_clone);
                let _ = app_clone.emit("download-cancelled", serde_json::json!({ "id": id_clone }));
            }
            Err(DownloadError::Failed(e)) => {
                log::error!("[download] {} failed: {}", id_clone, e);
                let _ = app_clone.emit("download-failed", serde_json::json!({
                    "id":    id_clone,
                    "error": e,
                }));
            }
        }
    });

    Ok(download_id)
}

/// Cancel an in-progress download by ID.
/// React: await invoke("cancel_download", { downloadId: "..." })
#[tauri::command]
pub async fn cancel_download(
    state:   State<'_, AppState>,
    manager: State<'_, DownloadManager>,
    download_id: String,
) -> Result<(), String> {
    // Signal the running task to stop
    if let Some(token) = manager.0.lock().unwrap().remove(&download_id) {
        token.cancel();
    }
    // Update DB status
    state.db
        .update_download_progress(&download_id, 0.0, "cancelled")
        .await
        .map_err(|e| e.to_string())
}

// ── Download implementation ───────────────────────────────────────────────────

enum DownloadError {
    Cancelled,
    Failed(String),
}

async fn download_file(
    app:          &AppHandle,
    download_id:  &str,
    url:          &str,
    save_path:    &str,
    cancel_token: CancellationToken,
) -> Result<(), DownloadError> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let client   = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| DownloadError::Failed(e.to_string()))?;

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    let mut file = tokio::fs::File::create(save_path)
        .await
        .map_err(|e| DownloadError::Failed(e.to_string()))?;

    loop {
        tokio::select! {
            // Cancellation has priority — check it before reading the next chunk
            _ = cancel_token.cancelled() => {
                // Best-effort cleanup of the partial file
                drop(file);
                let _ = tokio::fs::remove_file(save_path).await;
                return Err(DownloadError::Cancelled);
            }

            chunk = stream.next() => {
                match chunk {
                    None => break, // stream finished
                    Some(Err(e)) => return Err(DownloadError::Failed(e.to_string())),
                    Some(Ok(bytes)) => {
                        file.write_all(&bytes)
                            .await
                            .map_err(|e| DownloadError::Failed(e.to_string()))?;

                        downloaded += bytes.len() as u64;

                        if total_size > 0 {
                            let progress = downloaded as f64 / total_size as f64;
                            let _ = app.emit("download-progress", serde_json::json!({
                                "id":         download_id,
                                "progress":   progress,
                                "downloaded": downloaded,
                                "total":      total_size,
                            }));
                        }
                    }
                }
            }
        }
    }

    file.flush()
        .await
        .map_err(|e| DownloadError::Failed(e.to_string()))?;

    let _ = app.emit("download-complete", serde_json::json!({ "id": download_id }));
    Ok(())
}
