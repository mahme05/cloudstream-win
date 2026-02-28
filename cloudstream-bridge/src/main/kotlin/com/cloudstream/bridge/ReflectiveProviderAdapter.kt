package com.cloudstream.bridge

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import java.lang.reflect.Method
import kotlin.coroutines.Continuation
import kotlin.coroutines.intrinsics.COROUTINE_SUSPENDED

/**
 * Adapts a real CloudStream MainAPI instance (loaded by a plugin's URLClassLoader)
 * to our bridge-local CloudstreamProvider interface.
 *
 * WHY THIS IS NEEDED:
 * When URLClassLoader loads a .cs3 plugin, the plugin's MainAPI class is a
 * DIFFERENT Class object from our bridge's CloudstreamProvider, even though
 * they describe the same methods. Java's type system considers them unrelated,
 * so `instance as CloudstreamProvider` would throw a ClassCastException.
 *
 * We resolve this by calling every method via reflection, which bypasses the
 * classloader boundary entirely. It is slightly slower than a direct call, but
 * the overhead is negligible compared to the network I/O each plugin does.
 */
class ReflectiveProviderAdapter(private val target: Any) : CloudstreamProvider {

    private val cls: Class<*> = target.javaClass

    // ── Property helpers ──────────────────────────────────────────────────────

    private fun stringProp(name: String, default: String = ""): String =
        runCatching {
            // Kotlin properties compile to getXxx() methods
            val getter = "get${name.replaceFirstChar { it.uppercase() }}"
            (cls.getMethod(getter).invoke(target) as? String) ?: default
        }.getOrElse {
            runCatching { cls.getField(name).get(target) as? String ?: default }.getOrElse { default }
        }

    private fun intProp(name: String, default: Int = 1): Int =
        runCatching {
            val getter = "get${name.replaceFirstChar { it.uppercase() }}"
            (cls.getMethod(getter).invoke(target) as? Int) ?: default
        }.getOrElse { default }

    @Suppress("UNCHECKED_CAST")
    private fun listProp(name: String): List<*> =
        runCatching {
            val getter = "get${name.replaceFirstChar { it.uppercase() }}"
            cls.getMethod(getter).invoke(target) as? List<*> ?: emptyList<Any>()
        }.getOrElse { emptyList<Any>() }

    // ── CloudstreamProvider properties ───────────────────────────────────────

    override val name:        String       get() = stringProp("name", cls.simpleName)
    override val mainUrl:     String       get() = stringProp("mainUrl")
    override val version:     Int          get() = intProp("version", 1)
    override val iconUrl:     String?      get() = runCatching { stringProp("iconUrl").ifBlank { null } }.getOrNull()
    override val description: String?      get() = runCatching { stringProp("description").ifBlank { null } }.getOrNull()
    override val authors:     List<String> get() = listProp("authors").filterIsInstance<String>()

    override val supportedTypes: List<TvType>
        get() {
            // The real MainAPI returns a List<TvType> but its TvType is a different
            // class from ours. Map by name.
            return listProp("supportedTypes").mapNotNull { item ->
                val itemName = item?.javaClass?.simpleName ?: return@mapNotNull null
                TvType.values().firstOrNull { it.name == itemName }
            }.ifEmpty { listOf(TvType.Movie) }
        }

    // ── search ────────────────────────────────────────────────────────────────

    override suspend fun search(query: String): List<SearchResponse> =
        withContext(Dispatchers.IO) {
            val method = findMethod("search", String::class.java)
                ?: return@withContext emptyList()

            @Suppress("UNCHECKED_CAST")
            val raw = invokeSuspend(method, query) as? List<*> ?: return@withContext emptyList()

            raw.mapNotNull { item ->
                if (item == null) return@mapNotNull null
                val ic = item.javaClass
                SearchResponse(
                    name      = reflectString(ic, item, "name"),
                    url       = reflectString(ic, item, "url"),
                    apiName   = reflectString(ic, item, "apiName"),
                    type      = reflectTvType(ic, item),
                    posterUrl = reflectStringNullable(ic, item, "posterUrl"),
                    year      = reflectInt(ic, item, "year"),
                    score     = reflectInt(ic, item, "score"),
                )
            }
        }

    // ── load ─────────────────────────────────────────────────────────────────

    override suspend fun load(url: String): LoadResponse? =
        withContext(Dispatchers.IO) {
            val method = findMethod("load", String::class.java)
                ?: return@withContext null
            val raw    = invokeSuspend(method, url) ?: return@withContext null
            val ic     = raw.javaClass
            val typeName = ic.simpleName

            when {
                typeName.contains("Movie") -> LoadResponse.MovieLoadResponse(
                    name      = reflectString(ic, raw, "name"),
                    url       = url,
                    type      = TvType.Movie,
                    dataUrl   = reflectString(ic, raw, "dataUrl").ifBlank { url },
                    plot      = reflectStringNullable(ic, raw, "plot"),
                    year      = reflectInt(ic, raw, "year"),
                    posterUrl = reflectStringNullable(ic, raw, "posterUrl"),
                )
                else -> {
                    @Suppress("UNCHECKED_CAST")
                    val epList = reflectList(ic, raw, "episodes")
                    val episodes = epList.mapNotNull { ep ->
                        if (ep == null) return@mapNotNull null
                        val ec = ep.javaClass
                        Episode(
                            data        = reflectString(ec, ep, "data"),
                            name        = reflectStringNullable(ec, ep, "name"),
                            season      = reflectInt(ec, ep, "season"),
                            episode     = reflectInt(ec, ep, "episode"),
                            posterUrl   = reflectStringNullable(ec, ep, "posterUrl"),
                            description = reflectStringNullable(ec, ep, "description"),
                        )
                    }
                    LoadResponse.TvSeriesLoadResponse(
                        name      = reflectString(ic, raw, "name"),
                        url       = url,
                        type      = TvType.TvSeries,
                        episodes  = episodes,
                        plot      = reflectStringNullable(ic, raw, "plot"),
                        year      = reflectInt(ic, raw, "year"),
                        posterUrl = reflectStringNullable(ic, raw, "posterUrl"),
                    )
                }
            }
        }

    // ── loadLinks ─────────────────────────────────────────────────────────────

    override suspend fun loadLinks(
        data:             String,
        isCasting:        Boolean,
        subtitleCallback: (SubtitleFile) -> Unit,
        callback:         (ExtractorLink) -> Unit,
    ): Boolean = withContext(Dispatchers.IO) {
        // loadLinks takes a callback — we pass lambdas and let reflection invoke them.
        // Because Kotlin lambdas are SAM-compatible, we can pass them as Java functional
        // interfaces if the plugin was compiled expecting Function1<T, Unit>.
        val method = cls.methods.firstOrNull { m ->
            m.name == "loadLinks" && m.parameterCount >= 3
        } ?: return@withContext false

        try {
            // Invoke; the coroutine continuation is handled by invokeSuspend
            invokeSuspend(method, data, isCasting,
                subtitleCallback as Any,
                callback as Any
            )
            true
        } catch (e: Exception) {
            System.err.println("[adapter] loadLinks error: ${e.message}")
            false
        }
    }

    // ── Reflection helpers ────────────────────────────────────────────────────

    private fun findMethod(name: String, vararg paramTypes: Class<*>): Method? =
        runCatching { cls.getMethod(name, *paramTypes) }.getOrNull()
            ?: cls.methods.firstOrNull { it.name == name }

    /**
     * Invoke a Kotlin suspend function via reflection.
     *
     * Kotlin suspend functions compile to JVM methods with a hidden
     * `Continuation` last parameter. We supply a real Continuation via
     * `suspendCancellableCoroutine` so the plugin's coroutine machinery
     * works correctly across the classloader boundary.
     *
     * If the method is NOT a suspend function (no Continuation param)
     * we fall back to a plain blocking invoke on Dispatchers.IO.
     */
    private suspend fun invokeSuspend(method: Method, vararg args: Any?): Any? {
        // Check if last param is Continuation — if so it's a suspend function
        val params = method.parameterTypes
        val isSuspend = params.isNotEmpty() &&
            params.last().name == "kotlin.coroutines.Continuation"

        return if (isSuspend) {
            suspendCancellableCoroutine { cont ->
                val allArgs = arrayOf(*args, cont as Continuation<Any?>)
                try {
                    val result = method.invoke(target, *allArgs)
                    // COROUTINE_SUSPENDED means the plugin will resume the
                    // continuation itself asynchronously — nothing to do here.
                    if (result !== COROUTINE_SUSPENDED) {
                        @Suppress("UNCHECKED_CAST")
                        cont.resumeWith(Result.success(result))
                    }
                } catch (e: java.lang.reflect.InvocationTargetException) {
                    cont.resumeWith(Result.failure(e.cause ?: e))
                } catch (e: Exception) {
                    cont.resumeWith(Result.failure(e))
                }
            }
        } else {
            withContext(Dispatchers.IO) {
                try {
                    method.invoke(target, *args)
                } catch (e: java.lang.reflect.InvocationTargetException) {
                    throw e.cause ?: e
                }
            }
        }
    }

    private fun reflectString(cls: Class<*>, obj: Any, name: String): String =
        runCatching {
            val getter = "get${name.replaceFirstChar { it.uppercase() }}"
            cls.getMethod(getter).invoke(obj) as? String ?: ""
        }.getOrElse {
            runCatching { cls.getField(name).get(obj) as? String ?: "" }.getOrElse { "" }
        }

    private fun reflectStringNullable(cls: Class<*>, obj: Any, name: String): String? =
        runCatching {
            val getter = "get${name.replaceFirstChar { it.uppercase() }}"
            cls.getMethod(getter).invoke(obj) as? String
        }.getOrElse {
            runCatching { cls.getField(name).get(obj) as? String }.getOrElse { null }
        }

    private fun reflectInt(cls: Class<*>, obj: Any, name: String): Int? =
        runCatching {
            val getter = "get${name.replaceFirstChar { it.uppercase() }}"
            cls.getMethod(getter).invoke(obj) as? Int
        }.getOrElse {
            runCatching { cls.getField(name).get(obj) as? Int }.getOrElse { null }
        }

    @Suppress("UNCHECKED_CAST")
    private fun reflectList(cls: Class<*>, obj: Any, name: String): List<Any?> =
        runCatching {
            val getter = "get${name.replaceFirstChar { it.uppercase() }}"
            cls.getMethod(getter).invoke(obj) as? List<*> ?: emptyList<Any>()
        }.getOrElse {
            runCatching { cls.getField(name).get(obj) as? List<*> ?: emptyList<Any>() }.getOrElse { emptyList() }
        }

    private fun reflectTvType(cls: Class<*>, obj: Any): TvType =
        runCatching {
            val getter = "getType"
            val raw    = cls.getMethod(getter).invoke(obj)
            val name   = raw?.javaClass?.simpleName ?: return@runCatching TvType.Movie
            TvType.values().firstOrNull { it.name == name } ?: TvType.Movie
        }.getOrElse { TvType.Movie }
}
