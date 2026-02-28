// sidecar/mod.rs
//
// Long-running JVM daemon bridge.
//
// ┌─────────────────────────────────────────────────────────────────┐
// │  ARCHITECTURE                                                   │
// │                                                                 │
// │  Tauri (Rust)          cloudstream-bridge.jar (Kotlin/JVM)      │
// │  ─────────────         ──────────────────────────────────────   │
// │  JvmBridge::spawn() ─► java -jar cloudstream-bridge.jar         │
// │                          │  stdin  ◄── newline-JSON requests    │
// │                          │  stdout ──► newline-JSON responses   │
// │                          │  stderr ──► inherited (Tauri logs)   │
// │                                                                 │
// │  Multiple async callers can be in-flight at once.               │
// │  Each request gets a UUID; responses are matched back by ID.    │
// │  The JVM process stays alive for the entire app session.        │
// └─────────────────────────────────────────────────────────────────┘
//
// PROTOCOL  (one JSON object per line, UTF-8, LF-terminated)
//
//   Rust → JVM:
//     { "id":"<uuid>", "action":"<action>",
//       "pluginId":"<id>", "pluginUrl":"<url>", "arg":"<string>" }
//
//   JVM → Rust (success):
//     { "id":"<uuid>", "ok":true,  "result":"<json-string>" }
//
//   JVM → Rust (error):
//     { "id":"<uuid>", "ok":false, "error":"<message>" }
//
// ACTIONS
//   loadPlugin         — download a .cs3 from pluginUrl, register it
//   loadPluginFromFile — load a .cs3 from a local file path in arg
//   removePlugin       — unregister pluginId
//   listPlugins        — returns JSON array of WirePluginMeta
//   search             — pluginId + arg=query   → JSON SearchResult[]
//   getEpisodes        — pluginId + arg=showUrl  → JSON Episode[]
//   getStreams          — pluginId + arg=mediaData→ JSON StreamSource[]
//   ping               — health-check, returns "pong"

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::oneshot;
use uuid::Uuid;

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct BridgeRequest<'a> {
    id:         String,
    action:     &'a str,
    #[serde(rename = "pluginId")]
    plugin_id:  &'a str,
    #[serde(rename = "pluginUrl")]
    plugin_url: &'a str,
    arg:        &'a str,
}

#[derive(Deserialize, Debug)]
struct BridgeResponse {
    id:     String,
    ok:     bool,
    result: Option<String>,
    error:  Option<String>,
}

// ── JvmBridge ────────────────────────────────────────────────────────────────

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<BridgeResponse>>>>;

pub struct JvmBridge {
    stdin:   Arc<Mutex<Box<dyn Write + Send>>>,
    pending: PendingMap,
}

impl JvmBridge {
    // ── Spawn ─────────────────────────────────────────────────────────────────

    /// Spawn the JVM bridge process and start the background reader thread.
    ///
    /// `jar_path`  — absolute path to cloudstream-bridge.jar
    /// `jre_path`  — optional path to a bundled JRE bin/java.exe;
    ///               if None we fall back to "java" on the system PATH.
    pub fn spawn(jar_path: &PathBuf, jre_path: Option<&PathBuf>) -> Result<Arc<Self>> {
        if !jar_path.exists() {
            return Err(anyhow!(
                "Bridge JAR not found at '{}'.\n\
                 Build it with:  cd cloudstream-bridge && ./gradlew deployToTauri",
                jar_path.display()
            ));
        }

        // Prefer the bundled JRE; fall back to system java
        let java_exe = jre_path
            .map(|p| p.join("bin").join("java.exe"))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "java".to_string());

        log::info!("[jvm_bridge] Spawning {} -jar {}", java_exe, jar_path.display());

        let mut child = std::process::Command::new(&java_exe)
            .args([
                "-Xss8m",     // larger thread stacks for Kotlin coroutines
                "-Xmx256m",   // cap heap — the bridge is just a relay process
                "-jar",
            ])
            .arg(jar_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())   // bridge logs appear in Tauri console
            .spawn()
            .map_err(|e| anyhow!(
                "Failed to launch JVM ('{}') — is Java 11+ installed or bundled?\n{}",
                java_exe, e
            ))?;

        let stdin_raw = child.stdin.take().expect("stdin pipe was not created");
        let stdout    = child.stdout.take().expect("stdout pipe was not created");

        // ── Wait for the "ready" handshake ────────────────────────────────────
        // The bridge prints  {"ready":true}  as its very first stdout line.
        let mut reader = BufReader::new(stdout);
        let mut handshake = String::new();
        reader
            .read_line(&mut handshake)
            .map_err(|e| anyhow!("JVM bridge did not send handshake: {}", e))?;

        if !handshake.contains("ready") {
            return Err(anyhow!(
                "JVM bridge sent unexpected handshake line: {}",
                handshake.trim()
            ));
        }
        log::info!("[jvm_bridge] JVM bridge ready ✓");

        // ── Build the shared bridge struct ────────────────────────────────────
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let bridge = Arc::new(JvmBridge {
            stdin:   Arc::new(Mutex::new(Box::new(stdin_raw))),
            pending: pending.clone(),
        });

        // ── Background reader thread ──────────────────────────────────────────
        // Reads response lines from the JVM and delivers them to the waiting
        // oneshot channels. The thread owns `child` so the process isn't
        // orphaned if the bridge handle is dropped.
        let pending_clone = pending.clone();
        std::thread::Builder::new()
            .name("jvm-bridge-reader".into())
            .spawn(move || {
                let _child_owner = child; // keep the child process alive

                for line_result in reader.lines() {
                    match line_result {
                        Err(e) => {
                            log::error!("[jvm_bridge] Pipe read error: {}", e);
                            break;
                        }
                        Ok(line) if line.is_empty() => continue,
                        Ok(line) => {
                            match serde_json::from_str::<BridgeResponse>(&line) {
                                Err(e) => {
                                    log::warn!("[jvm_bridge] Malformed JSON: {} | raw: {}", e, line);
                                }
                                Ok(resp) => {
                                    let tx = pending_clone.lock().unwrap().remove(&resp.id);
                                    if let Some(sender) = tx {
                                        let _ = sender.send(resp);
                                    } else {
                                        log::warn!("[jvm_bridge] Orphan response id={}", resp.id);
                                    }
                                }
                            }
                        }
                    }
                }

                log::error!(
                    "[jvm_bridge] Reader thread exiting — \
                     JVM process has likely crashed or been killed."
                );
                // Notify every pending caller that the bridge has died.
                // Dropping the sender makes the receiver's await return Err,
                // which the 60s timeout in call() will surface as a clean error
                // rather than a silent hang.
                let mut map = pending_clone.lock().unwrap();
                map.clear(); // dropping Senders notifies all waiting Receivers
            })
            .map_err(|e| anyhow!("Failed to spawn reader thread: {}", e))?;

        Ok(bridge)
    }

    // ── call ─────────────────────────────────────────────────────────────────

    /// Send one request to the JVM and await its response.
    ///
    /// This is safe to call from multiple async tasks concurrently.
    /// Each call gets a unique UUID; responses are matched by that ID.
    pub async fn call(
        &self,
        action:     &str,
        plugin_id:  &str,
        plugin_url: &str,
        arg:        &str,
    ) -> Result<String> {
        let id       = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<BridgeResponse>();

        // Register the pending waiter BEFORE writing to stdin to avoid any
        // possible race where the JVM replies before we register.
        self.pending.lock().unwrap().insert(id.clone(), tx);

        let req  = BridgeRequest { id: id.clone(), action, plugin_id, plugin_url, arg };
        let line = serde_json::to_string(&req).expect("request serialisation infallible") + "\n";

        {
            let mut stdin = self.stdin.lock().unwrap();
            if let Err(e) = stdin.write_all(line.as_bytes()) {
                self.pending.lock().unwrap().remove(&id);
                return Err(anyhow!("JVM stdin write failed (process dead?): {}", e));
            }
            // flush() ensures the line is pushed through the OS pipe buffer
            // immediately — without this, small writes can be held in the
            // kernel buffer and the JVM will never see them.
            if let Err(e) = stdin.flush() {
                self.pending.lock().unwrap().remove(&id);
                return Err(anyhow!("JVM stdin flush failed: {}", e));
            }
        }

        // Wait for the matching response, with a generous timeout
        let resp = tokio::time::timeout(Duration::from_secs(60), rx)
            .await
            .map_err(|_| {
                self.pending.lock().unwrap().remove(&id);
                anyhow!("JVM call '{}' timed out after 60s", action)
            })?
            .map_err(|_| anyhow!("JVM bridge response channel was dropped unexpectedly"))?;

        if resp.ok {
            resp.result
                .ok_or_else(|| anyhow!("JVM returned ok=true but result field was null"))
        } else {
            Err(anyhow!(
                "JVM bridge error in action='{}': {}",
                action,
                resp.error.unwrap_or_else(|| "<no error message>".into())
            ))
        }
    }

    // ── ping ──────────────────────────────────────────────────────────────────

    /// Check that the bridge is alive. Returns Ok(()) on success.
    pub async fn ping(&self) -> Result<()> {
        self.call("ping", "", "", "").await?;
        Ok(())
    }
}
