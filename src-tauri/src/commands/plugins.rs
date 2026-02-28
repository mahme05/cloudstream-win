// commands/plugins.rs

use tauri::State;
use serde::Deserialize;
use crate::AppState;
use crate::plugin_runtime::{PluginInfo, SearchResult, Episode, StreamSource};

// ── List ──────────────────────────────────────────────────────────────────────

/// Returns all installed plugins (both JS and native).
/// React: await invoke("list_plugins")
#[tauri::command]
pub async fn list_plugins(
    state: State<'_, AppState>,
) -> Result<Vec<PluginInfo>, String> {
    Ok(state.plugin_manager.list_plugins())
}

// ── Install JS plugin ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPluginPayload {
    pub js_path: Option<String>,  // local .js file path
    pub source:  Option<String>,  // raw JS source (for URL-downloaded plugins)
}

/// Install a .js plugin from a local file path or raw JS source.
/// Parses the @plugin-info header automatically.
/// React: await invoke("install_plugin", { payload: { jsPath: "C:\\...\\plugin.js" } })
#[tauri::command]
pub async fn install_plugin(
    state:   State<'_, AppState>,
    payload: InstallPluginPayload,
) -> Result<PluginInfo, String> {
    let source = if let Some(path) = &payload.js_path {
        std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read plugin file: {}", e))?
    } else if let Some(src) = payload.source {
        src
    } else {
        return Err("Must provide either jsPath or source".to_string());
    };

    let info = crate::plugin_runtime::PluginManager::parse_plugin_info(&source)
        .map_err(|e| format!("Invalid plugin (bad @plugin-info header): {}", e))?;

    let info_clone = info.clone();
    state.plugin_manager
        .load_plugin_from_source(source, info)
        .map_err(|e| e.to_string())?;

    Ok(info_clone)
}

// ── Install native CloudStream plugin ─────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallNativePayload {
    /// Direct URL to a .cs3 file (e.g. from a CloudStream repo's plugin list)
    pub plugin_url:  Option<String>,
    /// Local file path to a .cs3 that was already downloaded
    pub plugin_path: Option<String>,
}

/// Install a native CloudStream extension (.cs3).
/// The JVM bridge downloads (or loads) the file and returns its metadata.
///
/// React:
///   // From a repo URL:
///   await invoke("install_native_plugin", { payload: { pluginUrl: "https://..." } })
///
///   // From a local file (e.g. after the user picks one with the file dialog):
///   await invoke("install_native_plugin", { payload: { pluginPath: "C:\\...\\GogoAnime.cs3" } })
#[tauri::command]
pub async fn install_native_plugin(
    state:   State<'_, AppState>,
    payload: InstallNativePayload,
) -> Result<PluginInfo, String> {
    if let Some(url) = payload.plugin_url {
        state.plugin_manager
            .install_native_from_url(&url)
            .await
            .map_err(|e| format!("Failed to install native plugin from URL: {}", e))
    } else if let Some(path) = payload.plugin_path {
        state.plugin_manager
            .install_native_from_file(&path)
            .await
            .map_err(|e| format!("Failed to install native plugin from file: {}", e))
    } else {
        Err("Must provide either pluginUrl or pluginPath".to_string())
    }
}

// ── Fetch URL (CORS proxy for plugin installs) ────────────────────────────────

/// Fetch a URL and return its body as text.
/// Used when the React frontend needs to download a plugin file but is blocked
/// by CORS restrictions inside the WebView.
/// React: await invoke("fetch_url", { url: "https://..." })
#[tauri::command]
pub async fn fetch_url(url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| e.to_string())?;

    let text = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(text)
}

// ── Remove ────────────────────────────────────────────────────────────────────

/// Uninstall a plugin by ID.
/// For native plugins this also tells the JVM bridge to unload it.
/// React: await invoke("remove_plugin", { pluginId: "gogoanime" })
#[tauri::command]
pub async fn remove_plugin(
    state:     State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    // Tell the JVM bridge to drop the plugin if it was native
    if let Some(ref bridge) = state.jvm_bridge {
        // Ignore errors — the plugin might have been a JS plugin
        let _ = bridge.call("removePlugin", &plugin_id, "", "").await;
    }
    state.plugin_manager.remove_plugin(&plugin_id);
    Ok(())
}

// ── Search / Episodes / Streams ───────────────────────────────────────────────

/// Search for content using a specific plugin.
/// React: await invoke("search_content", { pluginId: "gogoanime", query: "naruto" })
#[tauri::command]
pub async fn search_content(
    state:     State<'_, AppState>,
    plugin_id: String,
    query:     String,
) -> Result<Vec<SearchResult>, String> {
    state.plugin_manager
        .search(&plugin_id, &query)
        .await
        .map_err(|e| {
            log::error!("[search] plugin={} error={}", plugin_id, e);
            e.to_string()
        })
}

/// Get episode list for a show / anime.
/// For movies, returns a single-item list.
/// React: await invoke("get_episodes", { pluginId: "gogoanime", showId: "https://..." })
#[tauri::command]
pub async fn get_episodes(
    state:     State<'_, AppState>,
    plugin_id: String,
    show_id:   String,
) -> Result<Vec<Episode>, String> {
    state.plugin_manager
        .get_episodes(&plugin_id, &show_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get playable stream URLs for a movie or episode.
/// React: await invoke("get_streams", { pluginId: "gogoanime", mediaId: "https://..." })
#[tauri::command]
pub async fn get_streams(
    state:     State<'_, AppState>,
    plugin_id: String,
    media_id:  String,
) -> Result<Vec<StreamSource>, String> {
    state.plugin_manager
        .get_streams(&plugin_id, &media_id)
        .await
        .map_err(|e| e.to_string())
}
