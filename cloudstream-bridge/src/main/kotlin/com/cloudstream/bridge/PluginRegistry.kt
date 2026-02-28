package com.cloudstream.bridge

import java.util.concurrent.ConcurrentHashMap

// ─────────────────────────────────────────────────────────────────────────────
// PluginRegistry — thread-safe store of loaded CloudStream providers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Metadata about a loaded plugin — mirrors the PluginInfo struct on the Rust side.
 * Field names are camelCase to match the JSON the Rust side expects.
 */
data class PluginMeta(
    val id:             String,
    val name:           String,
    val version:        String,
    val description:    String,
    val author:         String,
    val iconUrl:        String?,
    val supportedTypes: List<String>,
    val isNative:       Boolean,
)

/**
 * A loaded plugin: its metadata + the actual provider instance that can
 * search / load / extract.
 */
data class LoadedPlugin(
    val meta:     PluginMeta,
    val provider: CloudstreamProvider,
)

class PluginRegistry {
    private val plugins = ConcurrentHashMap<String, LoadedPlugin>()

    fun register(plugin: LoadedPlugin) {
        plugins[plugin.meta.id] = plugin
        System.err.println("[registry] Registered: ${plugin.meta.id} (${plugin.meta.name})")
    }

    fun get(id: String): LoadedPlugin =
        plugins[id] ?: throw IllegalStateException(
            "Plugin '$id' not found. Loaded: ${plugins.keys.joinToString()}"
        )

    fun remove(id: String) {
        plugins.remove(id)
        System.err.println("[registry] Removed: $id")
    }

    fun listAll(): List<PluginMeta> = plugins.values.map { it.meta }
}
