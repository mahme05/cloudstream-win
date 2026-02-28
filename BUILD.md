# BUILD.md — CloudStream Win: JVM Sidecar Setup

This document explains how to build and run the full stack.

---

## Architecture

```
cloudstream-win (Tauri app)
│
├── src/                        React frontend
├── src-tauri/                  Rust backend
│   ├── src/
│   │   ├── sidecar/mod.rs      ← spawns & talks to the JVM bridge
│   │   ├── plugin_runtime/     ← routes calls to JS (Boa) or JVM bridge
│   │   └── commands/           ← Tauri IPC commands
│   └── resources/
│       ├── cloudstream-bridge.jar   ← built by Gradle (step 1)
│       └── jre/                     ← optional bundled JRE (step 2)
│
└── cloudstream-bridge/         Kotlin/JVM bridge project
    └── src/main/kotlin/…       Main.kt, PluginExecutor.kt, etc.
```

---

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Java JDK | 17+ | For building the bridge JAR |
| Gradle | 8+ | Bundled via `./gradlew` wrapper |
| Rust | 1.75+ | `rustup update stable` |
| Node.js | 18+ | For the React frontend |
| Tauri CLI | 2.x | `npm install -g @tauri-apps/cli` |

---

## Step 1 — Build the Kotlin bridge JAR

```bash
cd cloudstream-bridge

# On Windows (PowerShell):
.\gradlew.bat deployToTauri

# On any platform:
./gradlew deployToTauri
```

This builds `cloudstream-bridge.jar` and copies it to:
```
src-tauri/resources/cloudstream-bridge.jar
```

You must run this **before** `cargo tauri dev` or `cargo tauri build`.

---

## Step 2 — Bundle a JRE (optional but recommended for distribution)

Without this step, end users need Java 17+ installed on their PC.
With this step, Java is bundled inside the app — no user action required.

```bash
# Create a minimal JRE using jlink (~35 MB vs ~300 MB full JDK)
jlink \
  --add-modules java.base,java.net.http,java.logging,java.sql,java.desktop \
  --strip-debug \
  --no-header-files \
  --no-man-pages \
  --compress=2 \
  --output src-tauri/resources/jre
```

Tauri bundles the `resources/jre/` directory automatically (configured in `tauri.conf.json`).

The Rust code in `lib.rs` checks for `resources/jre/` first; if absent it falls back to
the `java` command on the system PATH.

---

## Step 3 — Run in development

```bash
# From the repo root:
npm install
npm run tauri dev
```

The Rust setup code in `lib.rs` spawns the bridge JAR automatically when the app starts.
You will see this in the console:

```
[startup] JVM bridge launched successfully
[bridge] Ready. Waiting for requests…
```

---

## Step 4 — Production build

```bash
npm run tauri build
```

The output installer (`.msi` / `.exe`) in `src-tauri/target/release/bundle/` includes:
- The Tauri app
- `cloudstream-bridge.jar`
- The bundled JRE (if you ran step 2)

---

## Adding a Gradle wrapper (if missing)

If `cloudstream-bridge/` has no `gradlew` yet:

```bash
cd cloudstream-bridge
gradle wrapper --gradle-version 8.5
```

---

## How native plugins (.cs3) work at runtime

1. User browses a CloudStream repo in the app and clicks "Install"
2. React calls `invoke("install_native_plugin", { pluginUrl: "https://..." })`
3. Rust sends `{ "action": "loadPlugin", "pluginUrl": "..." }` to the JVM bridge via stdin pipe
4. The bridge downloads the `.cs3` (which is a renamed JAR), loads it with `URLClassLoader`,
   finds the `CloudstreamProvider` class, and registers it in `PluginRegistry`
5. The bridge replies with the plugin metadata JSON
6. Rust stores the plugin ID in `PluginManager` with `PluginKind::Native`
7. Future `search_content` / `get_episodes` / `get_streams` calls route through the bridge

---

## Troubleshooting

| Problem | Fix |
|---|---|
| `Bridge JAR not found` | Run `./gradlew deployToTauri` first |
| `Failed to launch JVM` | Install Java 17+ or run the `jlink` step |
| `Plugin 'x' not found` | The plugin wasn't installed in this session (plugins are in-memory only — persistence coming soon) |
| Bridge crashes on startup | Check stderr output; usually a missing class in the JAR |
| Search returns empty | Enable `[bridge]` log output — the Kotlin side logs to stderr which appears in the Tauri console |
