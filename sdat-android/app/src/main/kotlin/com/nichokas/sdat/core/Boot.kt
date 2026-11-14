package com.nichokas.sdat.core

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import coil3.SingletonImageLoader
import coil3.request.ImageRequest
import com.nichokas.sdat.ui.CoverPool
import com.nichokas.sdat.ui.CoverUrl
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.roundToInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.tonearm_ffi.Shuffle
import uniffi.tonearm_ffi.collectionAlbums
import uniffi.tonearm_ffi.collectionArtists
import uniffi.tonearm_ffi.collectionPlaylists
import uniffi.tonearm_ffi.collectionTracks
import uniffi.tonearm_ffi.homeMixes

/** The warm-up's parts and their share of the boot loader's ring, as on iOS. */
enum class BootPart(val weight: Double) { Discover(0.5), Playlists(0.15), Albums(0.15), Favs(0.1), Covers(0.1) }

/** How far each part got. Fractions only grow, so the ring never runs backwards. */
class BootProgress private constructor(private val parts: Map<BootPart, Double>) {
    constructor() : this(emptyMap())

    val value: Double get() = BootPart.entries.sumOf { it.weight * minOf(1.0, parts[it] ?: 0.0) }

    fun with(part: BootPart, fraction: Double): BootProgress =
        if (fraction > (parts[part] ?: 0.0)) BootProgress(parts + (part to fraction)) else this
}

/** One warm-up step: [run] reports its own fraction done; failing or passing [timeoutMs] counts as done. */
class WarmUpStep(val part: BootPart, val timeoutMs: Long, val run: suspend (report: (Double) -> Unit) -> Unit)

/**
 * iOS Boot.warmUp: [discover] and [library] run in parallel; [covers] starts once the library steps are done or
 * [libraryWaitMs] passed. Everything stops at [capMs], and every part ends reported as done.
 */
suspend fun warmUp(
    discover: WarmUpStep,
    library: List<WarmUpStep>,
    covers: WarmUpStep,
    report: (BootPart, Double) -> Unit,
    libraryWaitMs: Long = 10_000,
    capMs: Long = 25_000,
) {
    suspend fun run(step: WarmUpStep) {
        try {
            withTimeoutOrNull(step.timeoutMs) { step.run { report(step.part, it) } }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
        }
        report(step.part, 1.0)
    }
    withTimeoutOrNull(capMs) {
        coroutineScope {
            launch { run(discover) }
            val lib = library.map { async { run(it) } }
            launch {
                withTimeoutOrNull(libraryWaitMs) { lib.awaitAll() }
                run(covers)
            }
        }
    }
    BootPart.entries.forEach { report(it, 1.0) }
}

/**
 * Warm-up after a fresh sign-in (never on launch): a varied Discover pool saved to [DiscoverCache], the first
 * page of each library list, the favourite tracks' state and the lists' covers in Coil's cache.
 */
object Boot {
    /** Distinct covers wanted before Discover counts as ready. */
    private const val TARGET_COVERS = 100
    private const val LIST_COVERS = 24

    // The Cover size library rows use on iOS.
    private const val ROW_COVER_DP = 52

    var active by mutableStateOf(false)
        private set
    var progress by mutableStateOf(BootProgress())
        private set

    /** From the end of a warm-up until the shell has faded the loader out. */
    var fadingOut by mutableStateOf(false)
        private set

    private val scope = MainScope()
    private var job: Job? = null

    fun start(context: Context) {
        job?.cancel()
        val app = context.applicationContext
        val cache = DiscoverCache.of(app)
        cache.clear()
        progress = BootProgress()
        fadingOut = false
        active = true
        job = scope.launch {
            val px = (ROW_COVER_DP * app.resources.displayMetrics.density).roundToInt()
            val covers = mutableListOf<String>()
            fun addCovers(urls: List<String?>, kind: CoverUrl.Kind = CoverUrl.Kind.Album) {
                urls.take(LIST_COVERS).filterNotNull().mapTo(covers) { CoverUrl.sized(it, px, kind) }
            }
            warmUp(
                discover = WarmUpStep(BootPart.Discover, 15_000) { report -> discover(cache, report) },
                library = listOf(
                    WarmUpStep(BootPart.Playlists, 8_000) { collectionPlaylists().use { p -> addCovers(p.nextPage().orEmpty().map { it.coverUrl }) } },
                    WarmUpStep(BootPart.Albums, 8_000) { collectionAlbums().use { p -> addCovers(p.nextPage().orEmpty().map { it.coverUrl }) } },
                    WarmUpStep(BootPart.Favs, 8_000) {
                        coroutineScope {
                            launch { attempt { collectionTracks().use { it.nextPage() } }?.let { t -> Favs.mark(t.map { it.id }) } }
                            launch { attempt { collectionArtists().use { it.nextPage() } }?.let { a -> addCovers(a.map { it.pictureUrl }, CoverUrl.Kind.Artist) } }
                        }
                    },
                ),
                covers = WarmUpStep(BootPart.Covers, 6_000) { report ->
                    val loader = SingletonImageLoader.get(app)
                    val list = covers.toList()
                    var n = 0
                    coroutineScope {
                        list.map { url ->
                            async {
                                loader.execute(ImageRequest.Builder(app).data(url).size(px).build())
                                report(++n / list.size.toDouble())
                            }
                        }.awaitAll()
                    }
                },
                report = { part, f -> progress = progress.with(part, f) },
            )
            delay(350) // lets the ring visibly reach 100%
            active = false
            fadingOut = true
        }
    }

    fun cancel() {
        job?.cancel()
        job = null
        active = false
        fadingOut = false
    }

    fun fadedOut() {
        fadingOut = false
    }

    /** Same source and dedupe as Discover's load, but continues until the pool is varied. */
    private suspend fun discover(cache: DiscoverCache, report: (Double) -> Unit) {
        val pool = CoverPool()
        Shuffle(homeMixes()).use { source ->
            repeat(12) {
                val batch = source.nextBatch(60u)
                pool.add(batch)
                // Incremental, so a timeout keeps what arrived.
                if (pool.tracks.isNotEmpty()) pool.tracks.toList().let { withContext(Dispatchers.IO) { cache.save(it) } }
                report(pool.tracks.size / TARGET_COVERS.toDouble())
                if (pool.tracks.size >= TARGET_COVERS * 2 || (pool.tracks.size >= TARGET_COVERS && batch.isEmpty())) return
            }
        }
    }

    private suspend fun <T> attempt(block: suspend () -> T): T? = try {
        block()
    } catch (e: CancellationException) {
        throw e
    } catch (_: Exception) {
        null
    }
}
