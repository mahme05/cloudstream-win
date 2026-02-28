// commands/plugins.rs
// IPC handlers for plugin operations.
//
// #[tauri::command] makes a Rust function callable from React via:
//   import { invoke } from "@tauri-apps/api/core"
//   const results = await invoke("search_content", { pluginId: "...", query: "..." })
//
// State<AppState> gives you access to the database and plugin manager.
// The return type Result<T, String> means:
//   - Ok(value) -> sent to React as the resolved value
//   - Err(string) -> sent to React as a rejected error

use tauri::State;
use serde::{Deserialize, Serialize};
use crate::AppState;
use crate::plugin_runtime::{PluginInfo, SearchResult, Episode, StreamSource};

/// List all currently loaded plugins
/// React call: await invoke("list_plugins")
#[tauri::command]
pub async fn list_plugins(
    state: State<'_, AppState>,
) -> Result<Vec<PluginInfo>, String> {
    Ok(state.plugin_manager.list_plugins())
}

/// Install a plugin from a .wasm file path
/// React call: await invoke("install_plugin", { wasmPath: "C:/...", info: {...} })
#[tauri::command]
pub async fn install_plugin(
    state: State<'_, AppState>,
    wasm_path: String,
    info: PluginInfo,
) -> Result<(), String> {
    state.plugin_manager
        .load_plugin(&wasm_path, info)
        .map_err(|e| e.to_string())
}

/// Search for content using a specific plugin
/// React call: await invoke("search_content", { pluginId: "myPlugin", query: "naruto" })
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

/// Get episode list for a show
/// React call: await invoke("get_episodes", { pluginId: "...", showId: "..." })
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

/// Get stream URLs for a piece of media
/// React call: await invoke("get_streams", { pluginId: "...", mediaId: "..." })
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
