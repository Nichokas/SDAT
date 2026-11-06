package com.nichokas.sdat.core

import androidx.compose.runtime.mutableStateMapOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.FavouriteKind
import uniffi.tonearm_ffi.isFavourite
import uniffi.tonearm_ffi.setFavourite

/** Cached favourite state for tracks, so track menus reflect it. */
object Favs {
    private val scope = MainScope()
    private val tracks = mutableStateMapOf<String, Boolean>()
    private val writes = mutableMapOf<String, Job>()

    // Last value the server reported or accepted; a failed write falls back to it.
    private val confirmed = mutableMapOf<String, Boolean>()

    // Bumped on sign-out so requests from the previous session never write into the cache.
    private var generation = 0

    fun known(id: String): Boolean? = tracks[id]
    fun clear() {
        generation++
        tracks.clear()
        confirmed.clear()
        writes.values.forEach { it.cancel() }
        writes.clear()
    }

    suspend fun check(id: String) {
        if (tracks[id] != null) return
        val gen = generation
        try {
            val v = isFavourite(FavouriteKind.TRACK, id)
            // A toggle made while this was in flight wins.
            if (gen == generation && tracks[id] == null) {
                tracks[id] = v
                confirmed[id] = v
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
        }
    }

    /** Writes the state the menu advertised, which stays right even if the cached state was unknown or stale. */
    fun set(id: String, target: Boolean) {
        tracks[id] = target
        val gen = generation
        // The core doesn't order concurrent writes, so a fast second toggle waits for the first.
        val previous = writes[id]
        writes[id] = scope.launch {
            previous?.join()
            // The core writes as whoever is signed in now, so a queued write from the previous account must not run.
            if (gen != generation) return@launch
            try {
                setFavourite(FavouriteKind.TRACK, id, target)
                if (gen != generation) return@launch
                confirmed[id] = target
                Chrome.flash(if (target) "Added to favourites" else "Removed from favourites")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (gen != generation) return@launch
                // Only the newest write of this session decides what the cache shows.
                if (writes[id] == coroutineContext.job) {
                    val last = confirmed[id]
                    if (last == null) tracks.remove(id) else tracks[id] = last
                }
                Chrome.flash(friendlyMessage(e))
            }
        }
    }
}
