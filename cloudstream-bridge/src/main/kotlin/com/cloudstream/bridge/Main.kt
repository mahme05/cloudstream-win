package com.cloudstream.bridge

import kotlinx.coroutines.runBlocking
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.PrintStream

// ─────────────────────────────────────────────────────────────────────────────
// PROTOCOL
//
// Rust spawns this JAR once and keeps it alive for the whole app session.
// Communication is newline-delimited JSON over stdin / stdout.
//
//  Rust → JVM (one JSON object per line):
//    { "id":"<uuid>", "action":"<action>", "pluginId":"...",
//      "pluginUrl":"...", "arg":"..." }
//
//  JVM → Rust (one JSON object per line):
//    { "id":"<uuid>", "ok":true,  "result":"<json-string>" }
//    { "id":"<uuid>", "ok":false, "error":"<message>" }
//
//  Startup handshake (before entering the loop):
//    JVM prints:  {"ready":true}
//    Rust blocks until it reads this line before sending any requests.
//
// ACTIONS
//   ping               → "pong"                         (health-check)
//   loadPlugin         → WirePluginMeta JSON            (download .cs3 from pluginUrl)
//   loadPluginFromFile → WirePluginMeta JSON            (load .cs3 from arg=filePath)
//   removePlugin       → "true"                         (unregister pluginId)
//   listPlugins        → JSON array of WirePluginMeta
//   search             → JSON array of WireSearchResult
//   getEpisodes        → JSON array of WireEpisode
//   getStreams         → JSON array of WireStreamSource
// ─────────────────────────────────────────────────────────────────────────────

@Serializable
data class BridgeRequest(
    val id:        String,
    val action:    String,
    val pluginId:  String = "",
    val pluginUrl: String = "",
    val arg:       String = "",
)

@Serializable
data class BridgeResponse(
    val id:     String,
    val ok:     Boolean,
    val result: String? = null,
    val error:  String? = null,
)

val json = Json {
    ignoreUnknownKeys = true
    encodeDefaults    = true
    isLenient         = true
}

fun main() {
    // Route all System.err to stderr so it never corrupts our stdout protocol
    System.setErr(PrintStream(System.err, true, "UTF-8"))

    val registry = PluginRegistry()
    val executor = PluginExecutor(registry)
    val reader   = BufferedReader(InputStreamReader(System.`in`, "UTF-8"))
    val out      = System.out // keep a direct reference — don't use println

    // ── Handshake ─────────────────────────────────────────────────────────────
    out.println("""{"ready":true}""")
    out.flush()
    System.err.println("[bridge] Ready. Waiting for requests…")

    // ── Main request loop ─────────────────────────────────────────────────────
    while (true) {
        val line = reader.readLine() ?: break   // null = stdin closed, exit cleanly
        if (line.isBlank()) continue

        val response: BridgeResponse = try {
            val req = json.decodeFromString<BridgeRequest>(line)
            handleRequest(req, registry, executor)
        } catch (e: Exception) {
            System.err.println("[bridge] Failed to parse request: ${e.message}")
            BridgeResponse(id = "?", ok = false, error = "Parse error: ${e.message}")
        }

        out.println(json.encodeToString(response))
        out.flush()
    }

    System.err.println("[bridge] stdin closed — exiting.")
}

fun handleRequest(
    req:      BridgeRequest,
    registry: PluginRegistry,
    executor: PluginExecutor,
): BridgeResponse {
    System.err.println("[bridge] action=${req.action} pluginId=${req.pluginId}")

    return try {
        val result = runBlocking {
            when (req.action) {
                "ping"               -> "pong"
                "loadPlugin"         -> executor.loadPluginFromUrl(req.pluginUrl)
                "loadPluginFromFile" -> executor.loadPluginFromFile(req.arg)
                "removePlugin"       -> { registry.remove(req.pluginId); "true" }
                "listPlugins"        -> executor.listPlugins()
                "search"             -> executor.search(req.pluginId, req.arg)
                "getEpisodes"        -> executor.getEpisodes(req.pluginId, req.arg)
                "getStreams"         -> executor.getStreams(req.pluginId, req.arg)
                else -> throw IllegalArgumentException("Unknown action: ${req.action}")
            }
        }
        BridgeResponse(id = req.id, ok = true, result = result)
    } catch (e: Exception) {
        System.err.println("[bridge] Error in ${req.action}: ${e.message}")
        e.printStackTrace(System.err)
        BridgeResponse(id = req.id, ok = false, error = e.message ?: "Unknown error")
    }
}
