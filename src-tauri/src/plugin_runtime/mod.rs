// plugin_runtime/mod.rs
//
// Unified plugin manager — two plugin kinds, one API.
//
//  Kind 1 — JS plugins  (.js)
//    Written in JavaScript, executed locally via the Boa engine.
//    Zero external dependencies, works out of the box.
//    Backward-compatible with all existing plugins in /plugins/*.js
//
//  Kind 2 — Native plugins  (.cs3 / CloudStream extensions)
//    Real CloudStream Kotlin extensions downloaded from any CS repo.
//    Delegated entirely to the long-running JVM sidecar (sidecar/mod.rs).
//    The Kotlin code runs unmodified — OkHttp, Jsoup, coroutines all work.
//
// Both kinds are addressed through the same search/get_episodes/get_streams
// interface, so the React frontend doesn't need to know the difference.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

mod js_engine;
// native_ffi removed — Kotlin/Native FFI approach abandoned in favour of JVM sidecar
pub use js_engine::execute_js;

// ── Public result types (shared by both plugin kinds) ─────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub id:          String,
    pub title:       String,
    pub poster_url:  Option<String>,
    pub media_type:  String,
    pub year:        Option<i32>,
    pub rating:      Option<f32>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Episode {
    pub id:             String,
    pub title:          String,
    pub season:         Option<i32>,
    pub episode_number: i32,
    pub thumbnail_url:  Option<String>,
    pub description:    Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StreamSource {
    pub url:       String,
    pub quality:   String,
    pub format:    String,
    pub subtitles: Vec<SubtitleTrack>,
    pub headers:   HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubtitleTrack {
    pub url:      String,
    pub language: String,
    pub label:    String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub id:              String,
    pub name:            String,
    pub version:         String,
    pub description:     String,
    pub author:          String,
    pub icon_url:        Option<String>,
    pub supported_types: Vec<String>,
    #[serde(default)]
    pub is_builtin: bool,
    /// true  = CloudStream .cs3 extension running inside the JVM sidecar
    /// false = local .js plugin running in Boa
    #[serde(default)]
    pub is_native: bool,
}

// ── Internal storage ──────────────────────────────────────────────────────────

#[derive(Clone)]
enum PluginKind {
    /// Source is stored in RAM; executed by Boa on demand
    Js { source: String },
    /// Loaded into the JVM sidecar; identified by its plugin_id string
    Native,
}

#[derive(Clone)]
struct LoadedPlugin {
    info: PluginInfo,
    kind: PluginKind,
}

// ── PluginManager ─────────────────────────────────────────────────────────────

pub struct PluginManager {
    plugins:     Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    http_client: reqwest::Client,
    /// Shared handle to the long-running JVM bridge.
    /// None until the first native plugin is installed.
    pub jvm: Arc<RwLock<Option<Arc<crate::sidecar::JvmBridge>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            plugins:     Arc::new(RwLock::new(HashMap::new())),
            http_client,
            jvm:         Arc::new(RwLock::new(None)),
        }
    }

    // ── JVM bridge access ─────────────────────────────────────────────────────

    /// Returns a reference to the running bridge.
    /// Caller must have already stored one via AppState.
    fn require_jvm(&self) -> Result<Arc<crate::sidecar::JvmBridge>> {
        self.jvm
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow!("JVM bridge is not running. \
                This is a bug — it should have been started at app launch."))
    }

    // ── JS plugin API ─────────────────────────────────────────────────────────

    pub fn parse_plugin_info(source: &str) -> Result<PluginInfo> {
        let line = source
            .lines()
            .find(|l| l.contains("@plugin-info"))
            .ok_or_else(|| anyhow!("Missing @plugin-info header"))?;
        let start = line.find('{')
            .ok_or_else(|| anyhow!("@plugin-info header has no JSON object"))?;
        let v: serde_json::Value = serde_json::from_str(&line[start..])
            .map_err(|e| anyhow!("Bad @plugin-info JSON: {}", e))?;
        Ok(PluginInfo {
            id:              v["id"].as_str().unwrap_or("unknown").to_string(),
            name:            v["name"].as_str().unwrap_or("Unknown").to_string(),
            version:         v["version"].as_str().unwrap_or("1.0.0").to_string(),
            description:     v["description"].as_str().unwrap_or("").to_string(),
            author:          v["author"].as_str().unwrap_or("").to_string(),
            icon_url:        v["icon_url"].as_str().map(|s| s.to_string()),
            supported_types: v["supported_types"].as_array()
                .map(|a| a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect())
                .unwrap_or_default(),
            is_builtin: v["is_builtin"].as_bool().unwrap_or(false),
            is_native:  false,
        })
    }

    pub fn load_plugin_from_source(&self, source: String, info: PluginInfo) -> Result<()> {
        log::info!("[plugin_manager] Registering JS plugin: {}", info.id);
        self.plugins.write().unwrap().insert(
            info.id.clone(),
            LoadedPlugin { info, kind: PluginKind::Js { source } },
        );
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_plugin(&self, js_path: &str, info: PluginInfo) -> Result<()> {
        let source = std::fs::read_to_string(js_path)
            .map_err(|e| anyhow!("Cannot read plugin file '{}': {}", js_path, e))?;
        self.load_plugin_from_source(source, info)
    }

    // ── Native plugin API ─────────────────────────────────────────────────────

    /// Download and register a CloudStream .cs3 plugin from a URL.
    /// The JVM bridge downloads the file, loads it, and returns its metadata.
    pub async fn install_native_from_url(&self, plugin_url: &str) -> Result<PluginInfo> {
        log::info!("[plugin_manager] Installing native plugin from: {}", plugin_url);
        let bridge    = self.require_jvm()?;
        let meta_json = bridge.call("loadPlugin", "", plugin_url, "").await?;
        let info      = Self::parse_native_meta(&meta_json)?;
        self.register_native(info.clone());
        Ok(info)
    }

    /// Register a native plugin that was already loaded into the JVM from a
    /// local file path (used when restoring persisted plugins on startup).
    pub async fn install_native_from_file(&self, file_path: &str) -> Result<PluginInfo> {
        log::info!("[plugin_manager] Installing native plugin from file: {}", file_path);
        let bridge    = self.require_jvm()?;
        let meta_json = bridge.call("loadPluginFromFile", "", "", file_path).await?;
        let info      = Self::parse_native_meta(&meta_json)?;
        self.register_native(info.clone());
        Ok(info)
    }

    fn register_native(&self, info: PluginInfo) {
        self.plugins.write().unwrap().insert(
            info.id.clone(),
            LoadedPlugin { info, kind: PluginKind::Native },
        );
    }

    fn parse_native_meta(json: &str) -> Result<PluginInfo> {
        let v: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| anyhow!("Bad plugin meta JSON from JVM: {}", e))?;
        Ok(PluginInfo {
            id:              v["id"].as_str().unwrap_or("unknown").to_string(),
            name:            v["name"].as_str().unwrap_or("Unknown").to_string(),
            version:         v["version"].as_str().unwrap_or("1.0.0").to_string(),
            description:     v["description"].as_str().unwrap_or("").to_string(),
            author:          v["author"].as_str().unwrap_or("CloudStream").to_string(),
            icon_url:        v["iconUrl"].as_str().map(|s| s.to_string()),
            supported_types: v["supportedTypes"].as_array()
                .map(|a| a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect())
                .unwrap_or_default(),
            is_builtin: false,
            is_native:  true,
        })
    }

    // ── Common API ────────────────────────────────────────────────────────────

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.read().unwrap().values().map(|p| p.info.clone()).collect()
    }

    pub fn remove_plugin(&self, id: &str) {
        self.plugins.write().unwrap().remove(id);
    }

    pub async fn search(&self, plugin_id: &str, query: &str) -> Result<Vec<SearchResult>> {
        let json = self.dispatch(plugin_id, "search", query).await?;
        serde_json::from_str(&json).map_err(|e| anyhow!("search() bad JSON: {}", e))
    }

    pub async fn get_episodes(&self, plugin_id: &str, show_id: &str) -> Result<Vec<Episode>> {
        let json = self.dispatch(plugin_id, "getEpisodes", show_id).await?;
        serde_json::from_str(&json).map_err(|e| anyhow!("get_episodes() bad JSON: {}", e))
    }

    pub async fn get_streams(&self, plugin_id: &str, media_id: &str) -> Result<Vec<StreamSource>> {
        let json = self.dispatch(plugin_id, "getStreams", media_id).await?;
        serde_json::from_str(&json).map_err(|e| anyhow!("get_streams() bad JSON: {}", e))
    }

    async fn dispatch(&self, plugin_id: &str, func: &str, arg: &str) -> Result<String> {
        let plugin = {
            let g = self.plugins.read().unwrap();
            g.get(plugin_id).cloned()
                .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?
        };

        match plugin.kind {
            // ── JS path: run Boa in a blocking thread ─────────────────────────
            PluginKind::Js { source } => {
                let client = self.http_client.clone();
                let func   = func.to_string();
                let arg    = arg.to_string();
                tokio::task::spawn_blocking(move || {
                    execute_js(&source, &func, &arg, client)
                })
                .await
                .map_err(|e| anyhow!("JS thread panic: {}", e))?
            }

            // ── Native path: delegate to JVM bridge ───────────────────────────
            PluginKind::Native => {
                let bridge = self.require_jvm()?;
                bridge.call(func, plugin_id, "", arg).await
            }
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self { Self::new() }
}
