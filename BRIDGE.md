# CloudStream Win — JVM Sidecar Bridge

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  CloudStream Win                                                     │
│                                                                     │
│  ┌──────────────┐    invoke()    ┌───────────────────────────────┐  │
│  │  React       │ ─────────────► │  Rust (Tauri)                 │  │
│  │  Frontend    │ ◄───────────── │  commands/plugins.rs          │  │
│  └──────────────┘    JSON        │  plugin_runtime/mod.rs        │  │
│                                  │        │                       │  │
│                                  │   JS plugins?                 │  │
│                                  │   └─► Boa engine (in-process) │  │
│                                  │                               │  │
│                                  │   .cs3 plugins?               │  │
│                                  │   └─► sidecar/mod.rs          │  │
│                                  └───────────┬───────────────────┘  │
│                                              │ stdin/stdout pipe     │
│                                              │ newline-JSON          │
│                                  ┌───────────▼───────────────────┐  │
│                                  │  cloudstream-bridge.jar (JVM) │  │
│                                  │  • Loads .cs3 plugins via     │  │
│                                  │    URLClassLoader             │  │
│                                  │  • Runs OkHttp, Jsoup,        │  │
│                                  │    Kotlin coroutines          │  │
│                                  │  • 100% CloudStream compat.   │  │
│                                  └───────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## First-Time Setup

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| JDK | 11+ | Building + running the bridge JAR |
| Rust | latest stable | Tauri backend |
| Node.js | 18+ | React frontend |

### Build everything

```powershell
# From the repo root — builds the JAR and bundles a minimal JRE
powershell -ExecutionPolicy Bypass -File scripts/build-all.ps1
```

Then start dev mode:

```powershell
cargo tauri dev
```

Or build the production installer:

```powershell
cargo tauri build
```

---

## How Each Part is Built

### 1. Kotlin Bridge JAR

```powershell
cd cloudstream-bridge
.\gradlew.bat deployToTauri
```

This compiles the Kotlin code into a fat JAR and copies it to:
```
src-tauri/resources/cloudstream-bridge.jar
```

Tauri's `bundle.resources` config (in `tauri.conf.json`) then includes it in the installer.

### 2. Bundled JRE (for distribution)

```powershell
powershell -ExecutionPolicy Bypass -File scripts/bundle-jre.ps1
```

Uses `jlink` to create a **~35 MB** minimal JRE containing only the modules the bridge needs. Output goes to `src-tauri/resources/jre/`.

End users **do not need Java installed** — the JRE is bundled inside the app.

During development, the system `java` on your PATH is used as a fallback if `jre/` doesn't exist yet.

---

## Protocol Reference

The bridge communicates over **newline-delimited JSON** on stdin/stdout.

### Rust → JVM (request)

```json
{
  "id":        "550e8400-e29b-41d4-a716-446655440000",
  "action":    "search",
  "pluginId":  "gogoanime",
  "pluginUrl": "",
  "arg":       "naruto"
}
```

### JVM → Rust (success)

```json
{ "id": "550e8400-...", "ok": true, "result": "[{...}, {...}]" }
```

### JVM → Rust (error)

```json
{ "id": "550e8400-...", "ok": false, "error": "Plugin not found: gogoanime" }
```

### Actions

| Action | pluginId | pluginUrl | arg | Returns |
|--------|----------|-----------|-----|---------|
| `ping` | — | — | — | `"pong"` |
| `loadPlugin` | — | `.cs3` URL | — | `WirePluginMeta` JSON |
| `loadPluginFromFile` | — | — | local path | `WirePluginMeta` JSON |
| `removePlugin` | plugin id | — | — | `"true"` |
| `listPlugins` | — | — | — | `WirePluginMeta[]` JSON |
| `search` | plugin id | — | query string | `WireSearchResult[]` JSON |
| `getEpisodes` | plugin id | — | show URL | `WireEpisode[]` JSON |
| `getStreams` | plugin id | — | media data | `WireStreamSource[]` JSON |

---

## React Usage Examples

### Install a .cs3 plugin from a CloudStream repo

```typescript
// From a repo URL
const info = await invoke<PluginInfo>("install_native_plugin", {
  payload: { pluginUrl: "https://raw.githubusercontent.com/recloudstream/cs3-repo/builds/GogoAnime.cs3" }
});

// From a local file (after user picks it with the file dialog)
const info = await invoke<PluginInfo>("install_native_plugin", {
  payload: { pluginPath: "C:\\Users\\...\\GogoAnime.cs3" }
});
```

### Search with a native plugin

```typescript
const results = await invoke<SearchResult[]>("search_content", {
  pluginId: "gogoanime",
  query: "naruto"
});
```

### Install a JS plugin (unchanged from before)

```typescript
const info = await invoke<PluginInfo>("install_plugin", {
  payload: { jsPath: "C:\\...\\my-plugin.js" }
});
```

---

## File Map

```
cloudstream-bridge/               ← Kotlin bridge source
  src/main/kotlin/com/cloudstream/bridge/
    Main.kt                       ← Entry point, request loop
    PluginRegistry.kt             ← Thread-safe plugin store
    PluginExecutor.kt             ← load / search / getEpisodes / getStreams
    CloudstreamProvider.kt        ← Interface + data types (mirrors MainAPI)
  build.gradle.kts                ← Fat JAR build + auto-deploy task

src-tauri/
  resources/
    cloudstream-bridge.jar        ← Built by Gradle (git-ignored)
    jre/                          ← Built by bundle-jre.ps1 (git-ignored)
  src/
    sidecar/mod.rs                ← Rust: spawns JVM, manages pipe, async calls
    plugin_runtime/
      mod.rs                      ← Routes JS vs native, shared types
      js_engine.rs                ← Boa JS execution (unchanged)
    commands/plugins.rs           ← Tauri IPC commands (list/install/search/etc.)
    lib.rs                        ← App startup, AppState, bridge init

scripts/
  build-all.ps1                   ← One-shot: build JAR + bundle JRE
  bundle-jre.ps1                  ← jlink: creates ~35MB minimal JRE
```

---

## .gitignore additions

Add these if not already present:

```gitignore
# Built artifacts — regenerated by scripts/build-all.ps1
src-tauri/resources/cloudstream-bridge.jar
src-tauri/resources/jre/

# Gradle
cloudstream-bridge/.gradle/
cloudstream-bridge/build/
```
