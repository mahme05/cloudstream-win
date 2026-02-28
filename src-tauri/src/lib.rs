// lib.rs
// This is the "root" of your library. It declares all submodules
// and wires together Tauri, the database, plugin runtime, and IPC commands.

use tauri::Manager;

// Declare our modules (Rust needs explicit module declarations)
pub mod commands;   // IPC handlers (called from React via invoke())
pub mod db;         // Database models and queries
pub mod plugin_runtime; // WASM plugin loader and executor
pub mod player;     // libmpv video player wrapper

// AppState holds shared data that all commands can access.
// Think of it like a global store, but safe and thread-friendly.
pub struct AppState {
    pub db: db::Database,
    pub plugin_manager: plugin_runtime::PluginManager,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging so you can see debug output in the terminal
    env_logger::init();

    tauri::Builder::default()
        // Register Tauri plugins
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        // Setup runs once when the app starts
        .setup(|app| {
            // Get the path to app data directory (where we'll store the DB)
            let app_data_dir = app.path().app_data_dir()
                .expect("Could not find app data directory");
            
            // Create the directory if it doesn't exist
            std::fs::create_dir_all(&app_data_dir)?;
            
            let db_path = app_data_dir.join("cloudstream.db");
            
            // Initialize database (blocking here is fine in setup)
            let db = tauri::async_runtime::block_on(async {
                db::Database::new(&db_path.to_string_lossy())
                    .await
                    .expect("Failed to initialize database")
            });
            
            // Initialize plugin manager
            let plugin_manager = plugin_runtime::PluginManager::new();
            
            // Register shared state so all commands can access it
            app.manage(AppState { db, plugin_manager });
            
            log::info!("CloudStream Win started!");
            Ok(())
        })
        // Register all IPC command handlers
        // These are the functions React can call via invoke("command_name")
        .invoke_handler(tauri::generate_handler![
            // Plugin commands
            commands::plugins::list_plugins,
            commands::plugins::install_plugin,
            commands::plugins::search_content,
            commands::plugins::get_episodes,
            commands::plugins::get_streams,
            // Bookmark commands
            commands::bookmarks::get_bookmarks,
            commands::bookmarks::add_bookmark,
            commands::bookmarks::remove_bookmark,
            // Download commands  
            commands::downloads::get_downloads,
            commands::downloads::start_download,
            commands::downloads::cancel_download,
            // Streaming / player commands
            commands::streaming::play_stream,
            commands::streaming::get_watch_history,
            commands::streaming::update_watch_progress,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
