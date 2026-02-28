// plugin_runtime/mod.rs
// JavaScript plugin execution using Boa — a pure-Rust JS engine.
// No system dependencies, works on Windows out of the box.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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

#[derive(Clone)]
struct LoadedPlugin {
    info: PluginInfo,
    source: String,
}

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

    pub fn parse_plugin_info(source: &str) -> Result<PluginInfo> {
        let line = source
            .lines()
            .find(|l| l.contains("@plugin-info"))
            .ok_or_else(|| anyhow!("Missing @plugin-info header"))?;
        let start = line.find('{').ok_or_else(|| anyhow!("@plugin-info missing JSON"))?;
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
                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }

    pub fn load_plugin_from_source(&self, source: String, info: PluginInfo) -> Result<()> {
        log::info!("Loading plugin: {}", info.id);
        self.plugins.write().unwrap().insert(info.id.clone(), LoadedPlugin { info, source });
        Ok(())
    }

    pub fn load_plugin(&self, js_path: &str, info: PluginInfo) -> Result<()> {
        let source = std::fs::read_to_string(js_path)
            .map_err(|e| anyhow!("Cannot read '{}': {}", js_path, e))?;
        self.load_plugin_from_source(source, info)
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.read().unwrap().values().map(|p| p.info.clone()).collect()
    }

    pub fn remove_plugin(&self, id: &str) {
        self.plugins.write().unwrap().remove(id);
    }

    pub async fn search(&self, plugin_id: &str, query: &str) -> Result<Vec<SearchResult>> {
        let json = self.call(plugin_id, "search", query).await?;
        serde_json::from_str(&json).map_err(|e| anyhow!("search() bad JSON: {}", e))
    }

    pub async fn get_episodes(&self, plugin_id: &str, show_id: &str) -> Result<Vec<Episode>> {
        let json = self.call(plugin_id, "get_episodes", show_id).await?;
        serde_json::from_str(&json).map_err(|e| anyhow!("get_episodes() bad JSON: {}", e))
    }

    pub async fn get_streams(&self, plugin_id: &str, media_id: &str) -> Result<Vec<StreamSource>> {
        let json = self.call(plugin_id, "get_streams", media_id).await?;
        serde_json::from_str(&json).map_err(|e| anyhow!("get_streams() bad JSON: {}", e))
    }

    async fn call(&self, plugin_id: &str, func: &str, arg: &str) -> Result<String> {
        let plugin = {
            let g = self.plugins.read().unwrap();
            g.get(plugin_id).cloned()
                .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?
        };
        let client = self.http_client.clone();
        let source = plugin.source.clone();
        let func   = func.to_string();
        let arg    = arg.to_string();
        tokio::task::spawn_blocking(move || execute_js(&source, &func, &arg, client))
            .await
            .map_err(|e| anyhow!("JS thread panic: {}", e))?
    }
}

impl Default for PluginManager {
    fn default() -> Self { Self::new() }
}

fn execute_js(
    source: &str,
    func_name: &str,
    arg: &str,
    http_client: reqwest::Client,
) -> Result<String> {
    use boa_engine::{
        js_string, native_function::NativeFunction, object::ObjectInitializer,
        property::Attribute, Context, JsError, JsString, JsValue, Source,
    };

    let mut ctx = Context::default();
    let client  = Arc::new(http_client);

    // ── __http_fetch(url, method, headersJson, body) -> responseJson ──
    let c = client.clone();
    let fetch_fn = unsafe { NativeFunction::from_closure(move |_this, args, _ctx| {
        let url    = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let method = args.get(1).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_else(|| "GET".into());
        let hdrs   = args.get(2).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_else(|| "{}".into());
        let body   = args.get(3).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped());
        let headers: HashMap<String, String> = serde_json::from_str(&hdrs).unwrap_or_default();
        let cc = c.clone();

        // spawn_blocking threads have no tokio handle — build a fresh one-shot runtime
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| JsError::from_opaque(JsValue::from(JsString::from(e.to_string()))))
            .and_then(|rt| {
                rt.block_on(async move {
                    let mut req = match method.to_uppercase().as_str() {
                        "POST"   => cc.post(&url),
                        "PUT"    => cc.put(&url),
                        "DELETE" => cc.delete(&url),
                        _        => cc.get(&url),
                    };
                    for (k, v) in &headers { req = req.header(k, v); }
                    if let Some(b) = body { req = req.body(b); }
                    let res    = req.send().await.map_err(|e| e.to_string())?;
                    let status = res.status().as_u16();
                    let text   = res.text().await.map_err(|e| e.to_string())?;
                    Ok::<serde_json::Value, String>(serde_json::json!({
                        "status": status, "ok": status >= 200 && status < 300, "body": text
                    }))
                })
                .map_err(|e: String| JsError::from_opaque(JsValue::from(JsString::from(e))))
            });

        match result {
            Ok(j)  => Ok(JsValue::from(JsString::from(j.to_string()))),
            Err(e) => Err(e),
        }
    }) };
    ctx.register_global_callable(js_string!("__http_fetch"), 4, fetch_fn)
        .map_err(|e| anyhow!("register fetch: {:?}", e))?;

    // ── console.log ──
    let log_fn = NativeFunction::from_copy_closure(|_this, args, _ctx| {
        let msg: Vec<String> = args.iter().map(|v| v.display().to_string()).collect();
        log::info!("[plugin] {}", msg.join(" "));
        Ok(JsValue::undefined())
    });
    let console = ObjectInitializer::new(&mut ctx)
        .function(log_fn, js_string!("log"), 0)
        .build();
    ctx.register_global_property(js_string!("console"), console, Attribute::all())
        .map_err(|e| anyhow!("register console: {:?}", e))?;

    // ── fetch() polyfill ──
    ctx.eval(Source::from_bytes(r#"
        async function fetch(url, opts) {
            var method  = (opts && opts.method)  || 'GET';
            var body    = (opts && opts.body != null) ? String(opts.body) : null;
            var headers = (opts && opts.headers) || {};
            var raw     = __http_fetch(url, method, JSON.stringify(headers), body);
            var parsed  = JSON.parse(raw);
            return {
                ok: parsed.ok, status: parsed.status,
                text: async function() { return parsed.body; },
                json: async function() { return JSON.parse(parsed.body); }
            };
        }
    "#)).map_err(|e| anyhow!("polyfill: {:?}", e))?;

    // ── load plugin source ──
    ctx.eval(Source::from_bytes(source.as_bytes()))
        .map_err(|e| anyhow!("plugin load: {:?}", e))?;

    // ── call function, capture result in JS globals ──
    // Using globals avoids needing access to Boa's private Promise internals.
    let arg_json = serde_json::to_string(arg)
        .unwrap_or_else(|_| format!("\"{}\"", arg));

    let call = format!(
        "var __result = null; var __error = null; \
         (async function() {{ \
             try {{ var r = await {func}({arg}); \
             __result = (typeof r === 'string') ? r : JSON.stringify(r); \
             }} catch(e) {{ __error = String(e); }} \
         }})();",
        func = func_name,
        arg  = arg_json,
    );

    ctx.eval(Source::from_bytes(call.as_bytes()))
        .map_err(|e| anyhow!("call script: {:?}", e))?;

    // Flush microtask queue — resolves the async IIFE
    ctx.run_jobs();

    // Check for JS-side error
    let err = ctx.eval(Source::from_bytes(b"__error"))
        .map_err(|e| anyhow!("read __error: {:?}", e))?;
    if !err.is_null_or_undefined() {
        if let Some(s) = err.as_string() {
            return Err(anyhow!("plugin '{}' threw: {}", func_name, s.to_std_string_escaped()));
        }
    }

    // Read result
    ctx.eval(Source::from_bytes(b"__result"))
        .map_err(|e| anyhow!("read __result: {:?}", e))?
        .as_string()
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| anyhow!("'{}' returned null — check plugin logs", func_name))
}
