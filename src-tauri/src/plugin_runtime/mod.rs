// plugin_runtime/mod.rs
// This is the heart of the plugin system.
// 
// Plugins are .wasm files that we load and execute safely.
// "Safely" means they can't access your disk or network — 
// all requests go through OUR Rust code first.
//
// Think of it like a browser tab: the website (plugin) can ask
// the browser (our app) to do things, but can't break out of the sandbox.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasmtime::*;

// ─────────────────────────────────────────────
// SHARED DATA TYPES
// These are the types plugins return, converted from JSON.
// Both the plugin system and IPC commands use these.
// ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub id: String,             // Unique ID for this media (used in get_episodes/get_streams)
    pub title: String,
    pub poster_url: Option<String>,
    pub media_type: String,     // "movie" | "show" | "anime"
    pub year: Option<i32>,
    pub rating: Option<f32>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Episode {
    pub id: String,
    pub title: String,
    pub season: Option<i32>,
    pub episode_number: i32,
    pub thumbnail_url: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StreamSource {
    pub url: String,
    pub quality: String,        // "1080p", "720p", "480p", etc.
    pub format: String,         // "hls", "mp4", "dash"
    pub subtitles: Vec<SubtitleTrack>,
    pub headers: HashMap<String, String>, // Custom headers (Referer, etc.)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubtitleTrack {
    pub url: String,
    pub language: String,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub supported_types: Vec<String>, // ["movies", "shows", "anime"]
}

// ─────────────────────────────────────────────
// PLUGIN MANAGER
// Manages all loaded plugins and handles calling into them.
// ─────────────────────────────────────────────

pub struct PluginManager {
    // Loaded plugins: plugin_id -> Plugin
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    // Wasmtime engine (shared, expensive to create, so we make one)
    engine: Engine,
    // HTTP client — plugins use this to make web requests through us
    http_client: reqwest::Client,
}

struct LoadedPlugin {
    info: PluginInfo,
    // The compiled WASM module
    module: Module,
}

impl PluginManager {
    pub fn new() -> Self {
        // Configure the Wasmtime engine
        // This is the core of the WASM sandbox
        let mut config = Config::new();
        config.wasm_component_model(false); // Using classic WASM for now, simpler
        
        let engine = Engine::new(&config)
            .expect("Failed to create Wasmtime engine");
        
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            engine,
            http_client,
        }
    }
    
    /// Load a plugin from a .wasm file path
    pub fn load_plugin(&self, wasm_path: &str, info: PluginInfo) -> Result<()> {
        log::info!("Loading plugin: {} from {}", info.id, wasm_path);
        
        // Read and compile the .wasm file
        // compile() is expensive but we only do it once per plugin
        let wasm_bytes = std::fs::read(wasm_path)?;
        let module = Module::new(&self.engine, &wasm_bytes)?;
        
        let plugin_id = info.id.clone();
        let loaded = LoadedPlugin { info, module };
        
        self.plugins.write().unwrap().insert(plugin_id, loaded);
        Ok(())
    }
    
    /// Get list of all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.read().unwrap()
            .values()
            .map(|p| p.info.clone())
            .collect()
    }
    
    /// Call the plugin's search() function
    /// 
    /// How this works:
    /// 1. We serialize the query to JSON
    /// 2. We call the WASM function with that JSON
    /// 3. The WASM returns JSON back
    /// 4. We deserialize it into Vec<SearchResult>
    pub async fn search(&self, plugin_id: &str, query: &str) -> Result<Vec<SearchResult>> {
        let result_json = self.call_plugin_function(plugin_id, "search", query).await?;
        let results: Vec<SearchResult> = serde_json::from_str(&result_json)?;
        Ok(results)
    }
    
    /// Call the plugin's get_episodes() function
    pub async fn get_episodes(&self, plugin_id: &str, show_id: &str) -> Result<Vec<Episode>> {
        let result_json = self.call_plugin_function(plugin_id, "get_episodes", show_id).await?;
        let episodes: Vec<Episode> = serde_json::from_str(&result_json)?;
        Ok(episodes)
    }
    
    /// Call the plugin's get_streams() function
    pub async fn get_streams(&self, plugin_id: &str, media_id: &str) -> Result<Vec<StreamSource>> {
        let result_json = self.call_plugin_function(plugin_id, "get_streams", media_id).await?;
        let streams: Vec<StreamSource> = serde_json::from_str(&result_json)?;
        Ok(streams)
    }
    
    /// Low-level: instantiate a WASM module and call a function.
    /// This is the core of plugin execution.
    async fn call_plugin_function(
        &self, 
        plugin_id: &str, 
        function_name: &str,
        input: &str,
    ) -> Result<String> {
        let module = {
            let plugins = self.plugins.read().unwrap();
            let plugin = plugins.get(plugin_id)
                .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?;
            plugin.module.clone()
        };
        
        // Create a fresh Store for each call (provides isolation)
        // Store holds the WASM instance's memory and state
        let mut store = Store::new(&self.engine, ());
        
        // Linker defines what host functions the WASM can call
        // We only expose an http_fetch function — nothing else
        let mut linker = Linker::new(&self.engine);
        
        // Register the HTTP fetch function that plugins can call
        // This is how plugins make web requests through our client
        let http_client = self.http_client.clone();
        
        // Note: In a full implementation, you'd use wasmtime's Func::wrap here
        // to register host functions the WASM can call.
        // For simplicity, this scaffold shows the structure.
        
        // Instantiate the WASM module
        let instance = linker.instantiate(&mut store, &module)?;
        
        // Get the function we want to call from the WASM module
        let func = instance.get_typed_func::<(i32, i32), (i32, i32)>(&mut store, function_name)
            .map_err(|e| anyhow!("Plugin '{}' missing function '{}': {}", plugin_id, function_name, e))?;
        
        // Write input string into WASM memory, call function, read output
        // This is the low-level string passing protocol
        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("Plugin has no exported memory"))?;
        
        let input_bytes = input.as_bytes();
        let input_len = input_bytes.len() as i32;
        
        // Allocate space in WASM memory for the input
        // (In a real implementation you'd call the WASM's malloc or alloc function)
        let input_ptr = 0i32; // simplified
        memory.write(&mut store, input_ptr as usize, input_bytes)?;
        
        // Call the WASM function with (pointer, length)
        let (out_ptr, out_len) = func.call(&mut store, (input_ptr, input_len))?;
        
        // Read the output string from WASM memory
        let mut output_bytes = vec![0u8; out_len as usize];
        memory.read(&store, out_ptr as usize, &mut output_bytes)?;
        
        let output = String::from_utf8(output_bytes)?;
        Ok(output)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
