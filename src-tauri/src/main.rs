// main.rs
// This is the entry point of your Rust application.
// Tauri starts here, loads the webview (your React app), and sets up IPC.
// Keep this file minimal — all logic lives in lib.rs and submodules.

// Prevents a terminal window from appearing on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cloudstream_win_lib::run();
}
