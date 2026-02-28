// commands/plugins.rs

use tauri::State;
use serde::Deserialize;
use crate::AppState;
use crate::plugin_runtime::{PluginInfo, SearchResult, Episode, StreamSource};

#[tauri::command]
pub async fn list_plugins(
    state: State<'_, AppState>,
) -> Result<Vec<PluginInfo>, String> {
    Ok(state.plugin_manager.list_plugins())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPluginPayload {
    pub js_path: Option<String>,  // Local file path (sent as jsPath from frontend)
    pub source: Option<String>,   // Raw JS source (for URL-installed plugins)
}

/// Install a plugin from a .js file path or raw JS source.
/// Automatically parses the @plugin-info header to get metadata.
/// React call: await invoke("install_plugin", { payload: { jsPath: "..." } })
#[tauri::command]
pub async fn install_plugin(
    state: State<'_, AppState>,
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

    // Auto-parse plugin info from the @plugin-info header
    let info = crate::plugin_runtime::PluginManager::parse_plugin_info(&source)
        .map_err(|e| format!("Invalid plugin: {}", e))?;

    let info_clone = info.clone();
    state.plugin_manager
        .load_plugin_from_source(source, info)
        .map_err(|e| e.to_string())?;

    Ok(info_clone)
}

/// Fetch a URL and return body as text (used for URL-based plugin installs,
/// which can't use browser fetch due to CORS restrictions in the WebView)
#[tauri::command]
pub async fn fetch_url(url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| e.to_string())?;
    let text = client.get(&url)
        .send().await.map_err(|e| format!("Request failed: {}", e))?
        .text().await.map_err(|e| format!("Failed to read body: {}", e))?;
    Ok(text)
}

/// Remove an installed plugin
#[tauri::command]
pub async fn remove_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    state.plugin_manager.remove_plugin(&plugin_id);
    Ok(())
}

#[tauri::command]
pub async fn search_content(
    state: State<'_, AppState>,
    plugin_id: String,
    query: String,
) -> Result<Vec<SearchResult>, String> {
    state.plugin_manager
        .search(&plugin_id, &query)
        .await
        .map_err(|e| {
            log::error!("Search error for plugin {}: {}", plugin_id, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn get_episodes(
    state: State<'_, AppState>,
    plugin_id: String,
    show_id: String,
) -> Result<Vec<Episode>, String> {
    state.plugin_manager
        .get_episodes(&plugin_id, &show_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_streams(
    state: State<'_, AppState>,
    plugin_id: String,
    media_id: String,
) -> Result<Vec<StreamSource>, String> {
    state.plugin_manager
        .get_streams(&plugin_id, &media_id)
        .await
        .map_err(|e| e.to_string())
}
