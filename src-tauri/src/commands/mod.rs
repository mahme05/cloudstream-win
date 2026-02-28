// commands/mod.rs
// IPC command modules.
// Each file here handles a category of commands callable from React via:
//   invoke("command_name", { arg1: value, arg2: value })

pub mod plugins;
pub mod bookmarks;
pub mod downloads;
pub mod streaming;
pub mod repos;
