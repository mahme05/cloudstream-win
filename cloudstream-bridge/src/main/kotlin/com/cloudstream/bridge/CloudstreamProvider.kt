package com.cloudstream.bridge

// ─────────────────────────────────────────────────────────────────────────────
// CloudstreamProvider — the interface every loaded plugin must implement.
//
// This mirrors CloudStream's real `MainAPI` interface so that dynamically
// loaded provider classes (via URLClassLoader from a .cs3 JAR) can be
// cast to it and called uniformly.
// ─────────────────────────────────────────────────────────────────────────────

interface CloudstreamProvider {
    val name:           String
    val mainUrl:        String
    val supportedTypes: List<TvType>
    val version:        Int     get() = 1
    val iconUrl:        String? get() = null
    val authors:        List<String> get() = emptyList()
    val description:    String? get() = null

    suspend fun search(query: String): List<SearchResponse>

    /** Load detailed info (episode list for shows, dataUrl for movies). */
    suspend fun load(url: String): LoadResponse?

    /**
     * Extract playable stream URLs.
     * Calls [subtitleCallback] for each subtitle track found.
     * Calls [callback] for each stream link found.
     */
    suspend fun loadLinks(
        data:             String,
        isCasting:        Boolean,
        subtitleCallback: (SubtitleFile) -> Unit,
        callback:         (ExtractorLink) -> Unit,
    ): Boolean
}

// ── Enums ─────────────────────────────────────────────────────────────────────

enum class TvType {
    Movie, TvSeries, Anime, AnimeMovie, OVA,
    Torrent, Documentary, AsianDrama, Live, Others;

    fun toWireString(): String = when (this) {
        Movie, Documentary -> "movie"
        Anime, AnimeMovie, OVA -> "anime"
        else -> "show"
    }
}

// ── Response types ────────────────────────────────────────────────────────────

data class SearchResponse(
    val name:      String,
    val url:       String,       // canonical URL — passed to load()
    val apiName:   String,
    val type:      TvType,
    val posterUrl: String? = null,
    val year:      Int?    = null,
    val score:     Int?    = null,   // 0–100
)

sealed class LoadResponse {
    abstract val name:     String
    abstract val url:      String
    abstract val type:     TvType
    abstract val plot:     String?
    abstract val year:     Int?
    abstract val rating:   Int?
    abstract val posterUrl: String?

    data class MovieLoadResponse(
        override val name:      String,
        override val url:       String,
        override val type:      TvType,
        /** The data string passed to loadLinks() to get stream URLs */
        val dataUrl:            String,
        override val plot:      String?  = null,
        override val year:      Int?     = null,
        override val rating:    Int?     = null,
        override val posterUrl: String?  = null,
    ) : LoadResponse()

    data class TvSeriesLoadResponse(
        override val name:      String,
        override val url:       String,
        override val type:      TvType,
        val episodes:           List<Episode>,
        override val plot:      String?  = null,
        override val year:      Int?     = null,
        override val rating:    Int?     = null,
        override val posterUrl: String?  = null,
    ) : LoadResponse()
}

data class Episode(
    val data:        String,        // passed to loadLinks()
    val name:        String? = null,
    val season:      Int?    = null,
    val episode:     Int?    = null,
    val posterUrl:   String? = null,
    val description: String? = null,
    val date:        Long?   = null,
)

data class SubtitleFile(
    val url:  String,
    val lang: String,
)

data class ExtractorLink(
    val source:   String,
    val name:     String,
    val url:      String,
    val referer:  String  = "",
    val quality:  Int     = -1,     // -1 = unknown, otherwise pixels (e.g. 1080)
    val isM3u8:   Boolean = false,
    val isDash:   Boolean = false,
    val headers:  Map<String, String> = emptyMap(),
)
