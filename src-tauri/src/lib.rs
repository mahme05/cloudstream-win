// lib.rs
//
// App entry point wired by Tauri.
//
// AppState holds:
//   db             — SQLite database (bookmarks, history, downloads)
//   plugin_manager — manages both JS and native CloudStream plugins
//   jvm_bridge     — the long-running Kotlin bridge process
//                    (stored separately so commands can send messages to it
//                     without going through plugin_manager for non-plugin ops)

use std::{path::PathBuf, sync::Arc};
use tauri::Manager;

pub mod commands;
pub mod db;
pub mod player;
pub mod plugin_runtime;
pub mod sidecar;

pub struct AppState {
    pub db:             db::Database,
    pub plugin_manager: plugin_runtime::PluginManager,
    /// Direct access to the JVM bridge for commands that need it
    /// (e.g. install_native_plugin, list_cs_repos, etc.)
    pub jvm_bridge: Option<Arc<sidecar::JvmBridge>>,
}

// AppState is Send + Sync automatically:
//   db             — Arc<SqlitePool>          : Send + Sync
//   plugin_manager — Arc<RwLock<_>>          : Send + Sync
//   jvm_bridge     — Option<Arc<JvmBridge>>  : Send + Sync
// No unsafe impls needed.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // ── Database ──────────────────────────────────────────────────────
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Could not resolve app data directory");
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("cloudstream.db");

            let db = tauri::async_runtime::block_on(async {
                db::Database::new(&db_path.to_string_lossy())
                    .await
                    .expect("Failed to initialise database")
            });

            // ── JVM bridge ────────────────────────────────────────────────────
            // The bridge JAR lives next to the app binary in the resources dir.
            // In dev mode (cargo tauri dev) it is built into src-tauri/resources/.
            // In production bundles Tauri copies it there automatically.
            let resource_dir = app
                .path()
                .resource_dir()
                .expect("Could not resolve resource directory");

            // In dev mode, resource_dir() resolves to target/debug/ where Tauri
            // copies a stub. Always use the actual src-tauri/resources/ folder
            // (known at compile time via CARGO_MANIFEST_DIR) so we get the real
            // fat JAR built by Gradle. In production the resource_dir is correct.
            let resources_dir = if cfg!(dev) {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("resources")
            } else {
                resource_dir.clone()
            };

            let jar_path = resources_dir.join("cloudstream-bridge.jar");
            log::info!("[startup] JAR path: {}", jar_path.display());

            // Optional bundled JRE — placed at resources/jre/ by the build script.
            // If absent, the system java on PATH is used instead.
            let jre_path: Option<PathBuf> = {
                let p = resources_dir.join("jre");
                if p.exists() { Some(p) } else { None }
            };

            let jvm_bridge = match sidecar::JvmBridge::spawn(&jar_path, jre_path.as_ref()) {
                Ok(bridge) => {
                    log::info!("[startup] JVM bridge launched successfully");
                    Some(bridge)
                }
                Err(e) => {
                    // Non-fatal — JS plugins still work without the JVM.
                    // Native plugins will return an error when invoked.
                    log::warn!(
                        "[startup] JVM bridge failed to start (native plugins unavailable): {}",
                        e
                    );
                    None
                }
            };

            // ── PluginManager — share the bridge handle ───────────────────────
            let plugin_manager = plugin_runtime::PluginManager::new();
            if let Some(ref bridge) = jvm_bridge {
                *plugin_manager.jvm.write().unwrap() = Some(bridge.clone());
            }

            app.manage(AppState { db, plugin_manager, jvm_bridge });
            // Download manager — tracks cancellation tokens for in-progress downloads
            app.manage(commands::downloads::DownloadManager::new());
            log::info!("[startup] CloudStream Win ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ── Plugin commands ───────────────────────────────────────────────
            commands::plugins::list_plugins,
            commands::plugins::install_plugin,
            commands::plugins::install_native_plugin,
            commands::plugins::fetch_url,
            commands::plugins::remove_plugin,
            commands::plugins::search_content,
            commands::plugins::get_episodes,
            commands::plugins::get_streams,
            // ── Repo browser ──────────────────────────────────────────────────
            commands::repos::fetch_repo,
            commands::repos::fetch_repos,
            // ── Bookmarks ─────────────────────────────────────────────────────
            commands::bookmarks::get_bookmarks,
            commands::bookmarks::add_bookmark,
            commands::bookmarks::remove_bookmark,
            // ── Downloads ─────────────────────────────────────────────────────
            commands::downloads::get_downloads,
            commands::downloads::start_download,
            commands::downloads::cancel_download,
            // ── Streaming / watch history ─────────────────────────────────────
            commands::streaming::play_stream,
            commands::streaming::get_watch_history,
            commands::streaming::update_watch_progress,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application error");
}
