// plugin_runtime/js_engine.rs
//
// Pure-Rust JS execution using the Boa engine.
// Used for the existing .js plugin format — completely unchanged.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub fn execute_js(
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

    // ── __http_fetch(url, method, headersJson, body) -> responseJson ──────────
    let c = client.clone();
    let fetch_fn = unsafe { NativeFunction::from_closure(move |_this, args, _ctx| {
        let url    = args.get(0).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
        let method = args.get(1).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_else(|| "GET".into());
        let hdrs   = args.get(2).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped()).unwrap_or_else(|| "{}".into());
        let body   = args.get(3).and_then(|v| v.as_string()).map(|s| s.to_std_string_escaped());
        let headers: HashMap<String, String> = serde_json::from_str(&hdrs).unwrap_or_default();
        let cc = c.clone();

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

    // ── console.log ───────────────────────────────────────────────────────────
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

    // ── fetch() polyfill ──────────────────────────────────────────────────────
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

    // ── load plugin source ────────────────────────────────────────────────────
    ctx.eval(Source::from_bytes(source.as_bytes()))
        .map_err(|e| anyhow!("plugin load: {:?}", e))?;

    // ── call the function, capture result via JS globals ─────────────────────
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

    ctx.run_jobs();

    let err = ctx.eval(Source::from_bytes(b"__error"))
        .map_err(|e| anyhow!("read __error: {:?}", e))?;
    if !err.is_null_or_undefined() {
        if let Some(s) = err.as_string() {
            return Err(anyhow!("plugin '{}' threw: {}", func_name, s.to_std_string_escaped()));
        }
    }

    ctx.eval(Source::from_bytes(b"__result"))
        .map_err(|e| anyhow!("read __result: {:?}", e))?
        .as_string()
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| anyhow!("'{}' returned null — check plugin logs", func_name))
}
