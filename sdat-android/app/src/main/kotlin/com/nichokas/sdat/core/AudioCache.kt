package com.nichokas.sdat.core

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.tonearm_ffi.CacheUsage
import uniffi.tonearm_ffi.CachedTrack
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.audioCacheCancel
import uniffi.tonearm_ffi.audioCacheFill
import uniffi.tonearm_ffi.audioCacheLookup
import uniffi.tonearm_ffi.audioCacheOpen
import uniffi.tonearm_ffi.audioCachePurge
import uniffi.tonearm_ffi.audioCacheRemove
import uniffi.tonearm_ffi.audioCacheSetLimit
import uniffi.tonearm_ffi.audioCacheTracks
import uniffi.tonearm_ffi.audioCacheUsage

object AudioCache {
    private lateinit var prefs: Preferences
    private val scope = MainScope()
    // The calls touch the disk. One at a time, in call order, so the listing published last is the newest.
    private val io = Dispatchers.IO.limitedParallelism(1)

    /** Most recently played first. */
    var tracks by mutableStateOf<List<CachedTrack>>(emptyList())
        private set
    var usage by mutableStateOf(CacheUsage(0u, 0u, 0u))
        private set

    /** Bytes; 0 is off. */
    val limit get() = prefs.audioCacheLimit
    val on get() = limit > 0

    fun open(context: Context, prefs: Preferences) {
        this.prefs = prefs
        // The OS cache directory: never backed up, and the core copes with the system clearing it.
        val dir = File(context.cacheDir, "audio").path
        // Until it is open, lookups miss and fills fail, so playback streams.
        update { audioCacheOpen(dir, prefs.audioCacheLimit.toULong()) }
    }

    fun refresh() = update {}

    fun setLimit(bytes: Long) {
        prefs.audioCacheLimit = bytes
        update { audioCacheSetLimit(bytes.toULong()) }
    }

    fun remove(id: String) = update { audioCacheRemove(id) }

    /** Also stops the running fills. */
    fun purge() = update { audioCachePurge() }

    fun cancel(id: String) = audioCacheCancel(id)

    /** The copy of [id] at [quality] or better. */
    suspend fun lookup(id: String, quality: Quality): CachedTrack? = withContext(Dispatchers.IO) { audioCacheLookup(id, quality) }

    /** Downloads [t] unless a copy at [quality] or better is there; a fill of a track already filling joins it. */
    suspend fun fill(t: Track, quality: Quality): CachedTrack = audioCacheFill(t, quality).also { refresh() }

    private fun update(change: () -> Unit) {
        scope.launch {
            val (t, u) = withContext(io) {
                runCatching(change)
                audioCacheTracks() to audioCacheUsage()
            }
            tracks = t
            usage = u
        }
    }
}
