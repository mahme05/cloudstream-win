// lib.rs
use tauri::Manager;

pub mod commands;
pub mod db;
pub mod plugin_runtime;
pub mod player;

pub struct AppState {
    pub db: db::Database,
    pub plugin_manager: plugin_runtime::PluginManager,
}

#[cfg_attr(desktop, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()
                .expect("Could not find app data directory");
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("cloudstream.db");

            let db = tauri::async_runtime::block_on(async {
                db::Database::new(&db_path.to_string_lossy())
                    .await
                    .expect("Failed to initialize database")
            });

            let plugin_manager = plugin_runtime::PluginManager::new();
            app.manage(AppState { db, plugin_manager });
            log::info!("CloudStream Win started!");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // JS Plugin commands
            commands::plugins::list_plugins,
            commands::plugins::install_plugin_file,
            commands::plugins::install_plugin_url,
            commands::plugins::remove_plugin,
            commands::plugins::search_content,
            commands::plugins::get_episodes,
            commands::plugins::get_streams,
            // Repo browser (CloudStream-compatible repo.json)
            commands::repos::fetch_repo,
            commands::repos::fetch_repos,
            // Bookmark commands
            commands::bookmarks::get_bookmarks,
            commands::bookmarks::add_bookmark,
            commands::bookmarks::remove_bookmark,
            // Download commands
            commands::downloads::get_downloads,
            commands::downloads::start_download,
            commands::downloads::cancel_download,
            // Streaming commands
            commands::streaming::play_stream,
            commands::streaming::get_watch_history,
            commands::streaming::update_watch_progress,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
