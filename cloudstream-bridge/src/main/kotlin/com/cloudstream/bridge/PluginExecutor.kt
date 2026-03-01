package com.cloudstream.bridge

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.net.URLClassLoader
import java.util.concurrent.TimeUnit
import java.util.jar.JarFile

// ─────────────────────────────────────────────────────────────────────────────
// Wire types — camelCase JSON, mirrors Rust structs exactly
// ─────────────────────────────────────────────────────────────────────────────

@Serializable data class WirePluginMeta(
    val id:             String,
    val name:           String,
    val version:        String,
    val description:    String,
    val author:         String,
    val iconUrl:        String?,
    val supportedTypes: List<String>,
    val isNative:       Boolean,
)

@Serializable data class WireSearchResult(
    val id:          String,
    val title:       String,
    val posterUrl:   String?,
    val mediaType:   String,
    val year:        Int?,
    val rating:      Float?,
    val description: String?,
)

@Serializable data class WireEpisode(
    val id:            String,
    val title:         String,
    val season:        Int?,
    val episodeNumber: Int,
    val thumbnailUrl:  String?,
    val description:   String?,
)

@Serializable data class WireStreamSource(
    val url:       String,
    val quality:   String,
    val format:    String,
    val subtitles: List<WireSubtitle>,
    val headers:   Map<String, String>,
)

@Serializable data class WireSubtitle(
    val url:      String,
    val language: String,
    val label:    String,
)

// ─────────────────────────────────────────────────────────────────────────────
// PluginExecutor
// ─────────────────────────────────────────────────────────────────────────────

class PluginExecutor(private val registry: PluginRegistry) {

    private val http = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .followRedirects(true)
        .build()

    // ── loadPluginFromUrl ─────────────────────────────────────────────────────
    // BUG FIX: OkHttp's execute() is blocking. Wrap in Dispatchers.IO so we
    // don't block the coroutine dispatcher thread that runs the main loop.

    suspend fun loadPluginFromUrl(url: String): String = withContext(Dispatchers.IO) {
        System.err.println("[executor] Downloading plugin: $url")

        val req  = Request.Builder()
            .url(url)
            .header("User-Agent", "CloudStream/4.0")
            .build()

        val bytes = http.newCall(req).execute().use { resp ->
            if (!resp.isSuccessful)
                throw RuntimeException("HTTP ${resp.code} downloading plugin from $url")
            resp.body?.bytes() ?: throw RuntimeException("Empty response body from $url")
        }

            // Derive a clean name from the URL before creating the temp file
        val pluginName = url.substringAfterLast("/").removeSuffix(".cs3")
        val tmp = File.createTempFile("cs_plugin_", ".cs3").also { it.deleteOnExit() }
        tmp.writeBytes(bytes)

        loadFromFile(tmp, sourceUrl = url, nameHint = pluginName)
    }

    // ── loadPluginFromFile ────────────────────────────────────────────────────

    suspend fun loadPluginFromFile(filePath: String): String = withContext(Dispatchers.IO) {
        val file = File(filePath)
        if (!file.exists()) throw IllegalArgumentException("File not found: $filePath")
        val nameHint = file.nameWithoutExtension
        loadFromFile(file, sourceUrl = filePath, nameHint = nameHint)
    }

    // ── Core loader ───────────────────────────────────────────────────────────
    // Must be called from Dispatchers.IO — URLClassLoader and JarFile do disk I/O.

    private fun loadFromFile(file: File, sourceUrl: String, nameHint: String = file.nameWithoutExtension): String {
        System.err.println("[executor] Loading plugin: $nameHint")

        val loader   = URLClassLoader(arrayOf(file.toURI().toURL()), javaClass.classLoader)
        val provider = tryLoadProvider(loader, file) ?: StubProvider(nameHint)

        val meta = PluginMeta(
            id             = provider.name.lowercase().replace(Regex("[^a-z0-9]"), "_"),
            name           = provider.name,
            version        = provider.version.toString(),
            description    = provider.description ?: "Loaded from ${file.name}",
            author         = provider.authors.joinToString().ifBlank { "CloudStream" },
            iconUrl        = provider.iconUrl,
            supportedTypes = provider.supportedTypes.map { it.toWireString() }.distinct(),
            isNative       = true,
        )

        registry.register(LoadedPlugin(meta, provider))

        return json.encodeToString(WirePluginMeta(
            id             = meta.id,
            name           = meta.name,
            version        = meta.version,
            description    = meta.description,
            author         = meta.author,
            iconUrl        = meta.iconUrl,
            supportedTypes = meta.supportedTypes,
            isNative       = true,
        ))
    }

    private fun tryLoadProvider(loader: URLClassLoader, file: File): CloudstreamProvider? {
        return try {
            // Strategy 1 — check JAR manifest for Plugin-Class or Main-Class
            JarFile(file).use { jar ->
                val attrs = jar.manifest?.mainAttributes
                val mainClass = attrs?.getValue("Plugin-Class")
                    ?: attrs?.getValue("Main-Class")
                if (mainClass != null) {
                    val cls = runCatching { loader.loadClass(mainClass) }.getOrNull()
                    val instance = cls?.let { tryInstantiate(it) }
                    if (instance != null) {
                        System.err.println("[executor] Found provider via manifest: $mainClass")
                        return instance
                    }
                }
            }

            // Strategy 2 — scan every .class in the JAR for a MainAPI subclass
            JarFile(file).use { jar ->
                val classNames = jar.entries().asSequence()
                    .filter { it.name.endsWith(".class") && !it.name.contains("$") }
                    .map { it.name.removeSuffix(".class").replace('/', '.') }
                    .toList()

                for (className in classNames) {
                    val cls = runCatching { loader.loadClass(className) }.getOrNull() ?: continue
                    val instance = tryInstantiate(cls) ?: continue
                    System.err.println("[executor] Found provider via scan: $className")
                    return instance
                }
            }

            System.err.println("[executor] No provider found in ${file.name} — using stub")
            null
        } catch (e: Exception) {
            System.err.println("[executor] Load error for ${file.name}: ${e.message}")
            null
        }
    }

    /**
     * Try to instantiate [cls] as a CloudstreamProvider.
     *
     * WHY NOT isAssignableFrom:
     * Real .cs3 plugins extend com.lagradost.cloudstream3.MainAPI, which is loaded
     * by the plugin's URLClassLoader. Our CloudstreamProvider interface is loaded by
     * the bridge's classloader. These are DIFFERENT Class objects even though they
     * describe the same contract — isAssignableFrom returns false across classloader
     * boundaries, so every real plugin would fall through to StubProvider.
     *
     * Instead we check the superclass/interface NAME, then wrap the instance in a
     * reflection-based adapter so the rest of the code can call it uniformly.
     */
    private fun tryInstantiate(cls: Class<*>): CloudstreamProvider? {
        // Check if any supertype is named MainAPI (the real CloudStream base class)
        val isMainApi = generateSequence(cls) { it.superclass }
            .any { it.name == "com.lagradost.cloudstream3.MainAPI" }
        // Also accept our own bridge interface (for unit-tested stubs)
        val isBridgeProvider = CloudstreamProvider::class.java.isAssignableFrom(cls)

        if (!isMainApi && !isBridgeProvider) return null

        return try {
            val raw = cls.getDeclaredConstructor().newInstance()
            if (isBridgeProvider) {
                raw as CloudstreamProvider
            } else {
                // Wrap the real MainAPI instance behind our interface via reflection
                ReflectiveProviderAdapter(raw)
            }
        } catch (e: Exception) {
            System.err.println("[executor] Could not instantiate ${cls.name}: ${e.message}")
            null
        }
    }

    // ── listPlugins ───────────────────────────────────────────────────────────

    fun listPlugins(): String = json.encodeToString(
        registry.listAll().map { m ->
            WirePluginMeta(
                id             = m.id,
                name           = m.name,
                version        = m.version,
                description    = m.description,
                author         = m.author,
                iconUrl        = m.iconUrl,
                supportedTypes = m.supportedTypes,
                isNative       = m.isNative,
            )
        }
    )

    // ── search ────────────────────────────────────────────────────────────────

    suspend fun search(pluginId: String, query: String): String =
        withContext(Dispatchers.IO) {
            val plugin  = registry.get(pluginId)
            val results = plugin.provider.search(query)
            json.encodeToString(results.map { r ->
                WireSearchResult(
                    id          = r.url,
                    title       = r.name,
                    posterUrl   = r.posterUrl,
                    mediaType   = r.type.toWireString(),
                    year        = r.year,
                    rating      = r.score?.toFloat()?.div(10f),
                    description = null,
                )
            })
        }

    // ── getEpisodes ───────────────────────────────────────────────────────────

    suspend fun getEpisodes(pluginId: String, showUrl: String): String =
        withContext(Dispatchers.IO) {
            val plugin = registry.get(pluginId)
            val loaded = plugin.provider.load(showUrl)
                ?: return@withContext json.encodeToString(emptyList<WireEpisode>())

            val episodes: List<WireEpisode> = when (loaded) {
                is LoadResponse.MovieLoadResponse -> listOf(
                    WireEpisode(
                        id            = loaded.dataUrl,
                        title         = loaded.name,
                        season        = null,
                        episodeNumber = 1,
                        thumbnailUrl  = loaded.posterUrl,
                        description   = loaded.plot,
                    )
                )
                is LoadResponse.TvSeriesLoadResponse -> loaded.episodes.mapIndexed { idx, ep ->
                    WireEpisode(
                        id            = ep.data,
                        title         = ep.name ?: "Episode ${ep.episode ?: idx + 1}",
                        season        = ep.season,
                        episodeNumber = ep.episode ?: idx + 1,
                        thumbnailUrl  = ep.posterUrl,
                        description   = ep.description,
                    )
                }
            }
            json.encodeToString(episodes)
        }

    // ── getStreams ────────────────────────────────────────────────────────────

    suspend fun getStreams(pluginId: String, mediaData: String): String =
        withContext(Dispatchers.IO) {
            val plugin  = registry.get(pluginId)
            val streams = mutableListOf<WireStreamSource>()
            val subs    = mutableListOf<WireSubtitle>()

            plugin.provider.loadLinks(
                data             = mediaData,
                isCasting        = false,
                subtitleCallback = { sub ->
                    subs.add(WireSubtitle(url = sub.url, language = sub.lang, label = sub.lang))
                },
                callback = { link ->
                    streams.add(
                        WireStreamSource(
                            url       = link.url,
                            quality   = if (link.quality > 0) "${link.quality}p" else "auto",
                            format    = when {
                                link.isM3u8 -> "hls"
                                link.isDash -> "dash"
                                else        -> "mp4"
                            },
                            subtitles = subs.toList(),
                            headers   = buildMap {
                                if (link.referer.isNotBlank()) put("Referer", link.referer)
                                putAll(link.headers)
                            },
                        )
                    )
                },
            )

            json.encodeToString(streams)
        }
}

// ── StubProvider ──────────────────────────────────────────────────────────────

class StubProvider(private val pluginId: String) : CloudstreamProvider {
    override val name           = pluginId
    override val mainUrl        = ""
    override val supportedTypes = listOf(TvType.Movie)

    override suspend fun search(query: String): List<SearchResponse> {
        System.err.println("[stub:$pluginId] search() — plugin failed to load")
        return emptyList()
    }

    override suspend fun load(url: String): LoadResponse? = null

    override suspend fun loadLinks(
        data:             String,
        isCasting:        Boolean,
        subtitleCallback: (SubtitleFile) -> Unit,
        callback:         (ExtractorLink) -> Unit,
    ): Boolean = false
}
