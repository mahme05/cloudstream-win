// plugin_runtime/mod.rs
// JavaScript plugin execution engine using QuickJS (via rquickjs).
//
// Each plugin is a .js file that exports three async functions:
//   search(query)       -> JSON string of SearchResult[]
//   getEpisodes(id)     -> JSON string of Episode[]
//   getStreams(id)       -> JSON string of StreamSource[]
//
// Plugins make HTTP requests through a sandboxed fetch() that routes
// all network calls through our reqwest client.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ─────────────────────────────────────────────
// SHARED DATA TYPES
// Mirror the TypeScript types in src/types/index.ts
// ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub poster_url: Option<String>,
    pub media_type: String,
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
    pub quality: String,
    pub format: String,
    pub subtitles: Vec<SubtitleTrack>,
    pub headers: HashMap<String, String>,
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
    pub supported_types: Vec<String>,
}

// ─────────────────────────────────────────────
// LOADED PLUGIN
// Stores the plugin metadata and its JS source code.
// ─────────────────────────────────────────────

#[derive(Clone)]
struct LoadedPlugin {
    info: PluginInfo,
    source: String, // The raw JS source code
}

// ─────────────────────────────────────────────
// PLUGIN MANAGER
// ─────────────────────────────────────────────

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    http_client: reqwest::Client,
}

impl PluginManager {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        }
    }

    /// Load a plugin from a .js file path.
    /// Parses the @plugin-info header comment to extract metadata.
    pub fn load_plugin(&self, js_path: &str, info: PluginInfo) -> Result<()> {
        log::info!("Loading JS plugin: {} from {}", info.id, js_path);

        let source = std::fs::read_to_string(js_path)
            .map_err(|e| anyhow!("Failed to read plugin file '{}': {}", js_path, e))?;

        let plugin_id = info.id.clone();
        let loaded = LoadedPlugin { info, source };
        self.plugins.write().unwrap().insert(plugin_id, loaded);
        Ok(())
    }

    /// Load a plugin from raw JS source (e.g. downloaded from URL).
    pub fn load_plugin_from_source(&self, source: String, info: PluginInfo) -> Result<()> {
        log::info!("Loading JS plugin from source: {}", info.id);
        let plugin_id = info.id.clone();
        let loaded = LoadedPlugin { info, source };
        self.plugins.write().unwrap().insert(plugin_id, loaded);
        Ok(())
    }

    /// Parse the @plugin-info JSON header from a JS file.
    pub fn parse_plugin_info(source: &str) -> Result<PluginInfo> {
        // Header format: // @plugin-info {...json...}
        let line = source
            .lines()
            .find(|l| l.contains("@plugin-info"))
            .ok_or_else(|| anyhow!("Plugin missing @plugin-info header"))?;

        let json_start = line.find('{')
            .ok_or_else(|| anyhow!("@plugin-info missing JSON object"))?;
        let json_str = &line[json_start..];

        // The JSON has is_builtin which we don't need in PluginInfo, so use a raw Value first
        let v: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Invalid @plugin-info JSON: {}", e))?;

        Ok(PluginInfo {
            id: v["id"].as_str().unwrap_or("unknown").to_string(),
            name: v["name"].as_str().unwrap_or("Unknown").to_string(),
            version: v["version"].as_str().unwrap_or("1.0.0").to_string(),
            description: v["description"].as_str().unwrap_or("").to_string(),
            author: v["author"].as_str().unwrap_or("").to_string(),
            icon_url: v["icon_url"].as_str().map(|s| s.to_string()),
            supported_types: v["supported_types"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.read().unwrap()
            .values()
            .map(|p| p.info.clone())
            .collect()
    }

    pub fn remove_plugin(&self, plugin_id: &str) {
        self.plugins.write().unwrap().remove(plugin_id);
    }

    // ─── PUBLIC API ───

    pub async fn search(&self, plugin_id: &str, query: &str) -> Result<Vec<SearchResult>> {
        let json = self.call_js_function(plugin_id, "search", query).await?;
        Ok(serde_json::from_str(&json)
            .map_err(|e| anyhow!("search() returned invalid JSON: {}", e))?)
    }

    pub async fn get_episodes(&self, plugin_id: &str, show_id: &str) -> Result<Vec<Episode>> {
        let json = self.call_js_function(plugin_id, "get_episodes", show_id).await?;
        Ok(serde_json::from_str(&json)
            .map_err(|e| anyhow!("get_episodes() returned invalid JSON: {}", e))?)
    }

    pub async fn get_streams(&self, plugin_id: &str, media_id: &str) -> Result<Vec<StreamSource>> {
        let json = self.call_js_function(plugin_id, "get_streams", media_id).await?;
        Ok(serde_json::from_str(&json)
            .map_err(|e| anyhow!("get_streams() returned invalid JSON: {}", e))?)
    }

    // ─── JS EXECUTION ───

    /// Execute a JS function from a plugin and return its string result.
    /// All HTTP requests from the plugin are intercepted and routed through reqwest.
    async fn call_js_function(
        &self,
        plugin_id: &str,
        function_name: &str,
        arg: &str,
    ) -> Result<String> {
        let plugin = {
            let plugins = self.plugins.read().unwrap();
            plugins.get(plugin_id)
                .cloned()
                .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?
        };

        let client = self.http_client.clone();
        let source = plugin.source.clone();
        let arg = arg.to_string();
        let function_name = function_name.to_string();

        // Run JS in a blocking thread — QuickJS is not async-native
        tokio::task::spawn_blocking(move || {
            run_js_plugin(&source, &function_name, &arg, client)
        })
        .await
        .map_err(|e| anyhow!("JS thread panicked: {}", e))?
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────
// JS EXECUTION ENGINE
// Runs plugin JS code in QuickJS with a sandboxed fetch() implementation.
// ─────────────────────────────────────────────

fn run_js_plugin(
    source: &str,
    function_name: &str,
    arg: &str,
    http_client: reqwest::Client,
) -> Result<String> {
    use rquickjs::{Context, Runtime, Function, Value, Object};

    let rt = Runtime::new().map_err(|e| anyhow!("QuickJS runtime error: {}", e))?;
    let ctx = Context::full(&rt).map_err(|e| anyhow!("QuickJS context error: {}", e))?;

    // Shared HTTP client for the fetch() implementation
    let client = Arc::new(http_client);

    ctx.with(|ctx| -> Result<String> {
        // ── Inject fetch() ──
        // Plugins call: const res = await fetch(url, options)
        // We intercept this and do a real HTTP request via reqwest.
        let client_clone = client.clone();
        let fetch_fn = Function::new(ctx.clone(), move |url: String, opts: rquickjs::Opt<Object>| {
            let client = client_clone.clone();

            // Parse options
            let method = opts.as_ref()
                .and_then(|o| o.get::<_, String>("method").ok())
                .unwrap_or_else(|| "GET".to_string());

            let body = opts.as_ref()
                .and_then(|o| o.get::<_, String>("body").ok());

            let headers_map: HashMap<String, String> = opts.as_ref()
                .and_then(|o| o.get::<_, Object>("headers").ok())
                .map(|hdrs| {
                    hdrs.props::<String, String>()
                        .filter_map(|r| r.ok())
                        .collect()
                })
                .unwrap_or_default();

            // Execute the request synchronously (we're already in a blocking thread)
            let rt_handle = tokio::runtime::Handle::try_current()
                .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());

            let response_text = rt_handle.block_on(async move {
                let mut req = match method.to_uppercase().as_str() {
                    "POST" => client.post(&url),
                    "PUT"  => client.put(&url),
                    _ =>      client.get(&url),
                };

                for (k, v) in &headers_map {
                    req = req.header(k, v);
                }

                if let Some(b) = body {
                    req = req.body(b);
                }

                let res = req.send().await.map_err(|e| e.to_string())?;
                let status = res.status().as_u16();
                let text = res.text().await.map_err(|e| e.to_string())?;
                Ok::<(u16, String), String>((status, text))
            });

            match response_text {
                Ok((status, text)) => {
                    // Return a Response-like object: { ok, status, text(), json() }
                    Ok(format!(
                        r#"{{
                            "ok": {},
                            "status": {},
                            "_body": {}
                        }}"#,
                        status >= 200 && status < 300,
                        status,
                        serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".into())
                    ))
                }
                Err(e) => Err(rquickjs::Error::Exception),
            }
        }).map_err(|e| anyhow!("Failed to create fetch fn: {:?}", e))?;

        ctx.globals().set("__fetch_impl", fetch_fn)
            .map_err(|e| anyhow!("Failed to set fetch: {:?}", e))?;

        // ── Inject console.log ──
        let log_fn = Function::new(ctx.clone(), |msg: String| {
            log::info!("[plugin] {}", msg);
        }).map_err(|e| anyhow!("Failed to create log fn: {:?}", e))?;
        ctx.globals().set("__console_log", log_fn)
            .map_err(|e| anyhow!("Failed to set console.log: {:?}", e))?;

        // ── Polyfills for fetch and console ──
        let polyfill = r#"
            const fetch = async (url, opts) => {
                const raw = __fetch_impl(url, opts);
                const parsed = JSON.parse(raw);
                return {
                    ok: parsed.ok,
                    status: parsed.status,
                    text: async () => parsed._body,
                    json: async () => JSON.parse(parsed._body),
                };
            };
            const console = { log: (...a) => __console_log(a.map(String).join(' ')) };
        "#;

        ctx.eval::<(), _>(polyfill)
            .map_err(|e| anyhow!("Polyfill error: {:?}", e))?;

        // ── Load plugin source ──
        ctx.eval::<(), _>(source.as_bytes())
            .map_err(|e| anyhow!("Plugin load error: {:?}", e))?;

        // ── Call the function ──
        // We wrap in an async IIFE and use a Promise resolver pattern
        let call_script = format!(
            r#"
            (async () => {{
                const result = await {}({});
                return typeof result === 'string' ? result : JSON.stringify(result);
            }})()
            "#,
            function_name,
            serde_json::to_string(arg).unwrap_or_else(|_| "\"\"".into())
        );

        // Evaluate and resolve the promise
        let promise: rquickjs::Promise = ctx.eval(call_script.as_bytes())
            .map_err(|e| anyhow!("Function call error: {:?}", e))?;

        rt.run_executor();

        let result: String = promise.finish()
            .map_err(|e| anyhow!("Promise rejected: {:?}", e))?;

        Ok(result)
    })
}
