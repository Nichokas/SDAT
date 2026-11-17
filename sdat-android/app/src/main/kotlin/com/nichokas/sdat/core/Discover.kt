package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.nichokas.sdat.ui.CoverPool
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.tonearm_ffi.Mix
import uniffi.tonearm_ffi.Shuffle
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.homeMixes

private const val POOL_TARGET = 320

/**
 * Discover's plane and home mixes for the session. They outlive the tab, as on iOS, so switching tabs neither
 * reloads the covers nor cancels a load in flight.
 */
object Discover {
    private val scope = MainScope()
    private var job: Job? = null
    private var loaded = false

    var tracks by mutableStateOf(emptyList<Track>())
        private set
    var mixes by mutableStateOf(emptyList<Mix>())
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /** Shows the last pool and fetches the next one; does nothing while a load runs or once a fresh pool arrived. */
    fun load(cache: DiscoverCache) {
        if (loaded || job?.isActive == true) return
        job = scope.launch { fetch(cache) }
    }

    private suspend fun fetch(cache: DiscoverCache) {
        val epoch = DiscoverCache.epoch
        error = null
        if (tracks.isEmpty()) tracks = withContext(Dispatchers.IO) { cache.load() }
        val hadCache = tracks.isNotEmpty()
        val pool = CoverPool()
        try {
            mixes = homeMixes()
            Shuffle(mixes).use { source ->
                repeat(10) {
                    val batch = try {
                        source.nextBatch(if (hadCache || pool.tracks.isNotEmpty()) 80u else 30u)
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        // A refresh cut short by the network keeps what arrived.
                        if (pool.tracks.isEmpty()) throw e
                        return@use
                    }
                    pool.add(batch)
                    // The plane's layout depends on the pool size, so only the first covers and the
                    // final pool are published: tiles on screen change at most once, as on iOS.
                    if (tracks.isEmpty()) tracks = pool.tracks.toList()
                    if (pool.tracks.size >= POOL_TARGET) return@use
                }
            }
            loaded = pool.tracks.isNotEmpty()
            if (loaded) pool.tracks.toList().let { withContext(Dispatchers.IO) { cache.save(it, epoch) } }
            if (pool.tracks.isEmpty() && !hadCache) error = "Your home mixes have no covers yet."
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (pool.tracks.isEmpty() && !hadCache) error = e.message ?: e.javaClass.simpleName
        }
        // The cached plane stays on screen this session; the fresh pool shows on the next launch.
        if (!hadCache) tracks = pool.tracks.toList()
    }

    /** The next account starts from its own pool. */
    fun reset() {
        job?.cancel()
        job = null
        loaded = false
        tracks = emptyList()
        mixes = emptyList()
        error = null
    }
}
