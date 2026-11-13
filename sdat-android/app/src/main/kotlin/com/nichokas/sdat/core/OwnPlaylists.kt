package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.OwnPlaylist
import uniffi.tonearm_ffi.ownPlaylists

/** The user's own playlists: their pages say "My playlist", offer removing tracks and hide the favourite button. */
object OwnPlaylists {
    var list by mutableStateOf(emptyList<OwnPlaylist>())
        private set
    private var loaded = false

    // Bumped on sign-out so a load from the previous session never fills the next one's list.
    private var generation = 0

    fun contains(id: String) = list.any { it.id == id }

    suspend fun load(force: Boolean = false) {
        if (loaded && !force) return
        val gen = generation
        try {
            val fresh = ownPlaylists()
            if (gen != generation) return
            list = fresh
            loaded = true
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
        }
    }

    fun clear() {
        generation++
        list = emptyList()
        loaded = false
    }
}
