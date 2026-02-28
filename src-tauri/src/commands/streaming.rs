// commands/streaming.rs
// Opens stream URLs in the user's installed media player (VLC, MPV, MPC-HC, etc.)
// This is simpler and more reliable than embedding a player.

use tauri::State;
use serde::Deserialize;
use chrono::Utc;
use crate::AppState;
use crate::db::WatchHistory;

/// Open a stream URL in the best available external player.
/// Tries VLC → MPV → MPC-HC → Windows default in that order.
#[tauri::command]
pub async fn play_stream(
    url: String,
    title: String,
    headers: Option<std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    log::info!("Opening stream: {}", url);

    // Build header args for players that support them
    let referer = headers
        .as_ref()
        .and_then(|h| h.get("Referer").or_else(|| h.get("referer")))
        .cloned();

    // Try each player in order of preference
    if let Ok(player) = try_vlc(&url, &referer) {
        return Ok(player);
    }
    if let Ok(player) = try_mpv(&url, &referer) {
        return Ok(player);
    }
    if let Ok(player) = try_mpc(&url) {
        return Ok(player);
    }

    // Last resort: open with Windows default handler
    // This works for MP4 links but not HLS (.m3u8)
    open_with_default(&url).map_err(|e| e.to_string())?;
    Ok("default".to_string())
}

/// Try to open with VLC
fn try_vlc(url: &str, referer: &Option<String>) -> Result<String, ()> {
    let vlc_paths = [
        r"C:\Program Files\VideoLAN\VLC\vlc.exe",
        r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
    ];

    for path in &vlc_paths {
        if std::path::Path::new(path).exists() {
            let mut args = vec![
                url.to_string(),
                "--play-and-exit".to_string(),
                "--no-video-title-show".to_string(),
            ];
            if let Some(ref r) = referer {
                args.push(format!("--http-referrer={}", r));
            }
            std::process::Command::new(path)
                .args(&args)
                .spawn()
                .map_err(|_| ())?;
            return Ok("vlc".to_string());
        }
    }
    Err(())
}

/// Try to open with MPV
fn try_mpv(url: &str, referer: &Option<String>) -> Result<String, ()> {
    let mpv_paths = [
        r"C:\Program Files\mpv\mpv.exe",
        r"C:\Program Files (x86)\mpv\mpv.exe",
        // Also try if mpv is on PATH
        "mpv",
    ];

    for path in &mpv_paths {
        let mut cmd = std::process::Command::new(path);
        cmd.arg(url);
        if let Some(ref r) = referer {
            cmd.arg(format!("--referrer={}", r));
        }
        if cmd.spawn().is_ok() {
            return Ok("mpv".to_string());
        }
    }
    Err(())
}

/// Try to open with MPC-HC
fn try_mpc(url: &str) -> Result<String, ()> {
    let mpc_paths = [
        r"C:\Program Files\MPC-HC\mpc-hc64.exe",
        r"C:\Program Files (x86)\MPC-HC\mpc-hc.exe",
        r"C:\Program Files\MPC-BE x64\mpc-be64.exe",
    ];

    for path in &mpc_paths {
        if std::path::Path::new(path).exists() {
            std::process::Command::new(path)
                .arg(url)
                .spawn()
                .map_err(|_| ())?;
            return Ok("mpc".to_string());
        }
    }
    Err(())
}

/// Open with Windows default handler (works for MP4, not HLS)
fn open_with_default(url: &str) -> anyhow::Result<()> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()?;
    Ok(())
}

// ── WATCH HISTORY ──────────────────────────────────────────

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
