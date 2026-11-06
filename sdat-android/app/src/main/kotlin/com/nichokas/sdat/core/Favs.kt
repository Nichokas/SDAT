package com.nichokas.sdat.core

import androidx.compose.runtime.mutableStateMapOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Job
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

    // Bumped on sign-out so requests from the previous session never write into the cache.
    private var generation = 0

    fun known(id: String): Boolean? = tracks[id]
    fun mark(ids: List<String>, v: Boolean) = ids.forEach { tracks[it] = v }
    fun clear() {
        generation++
        tracks.clear()
        writes.clear()
    }

    suspend fun check(id: String) {
        if (tracks[id] != null) return
        val gen = generation
        try {
            val v = isFavourite(FavouriteKind.TRACK, id)
            // A toggle made while this was in flight wins.
            if (gen == generation && tracks[id] == null) tracks[id] = v
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
        }
    }

    fun toggle(id: String) {
        if (tracks[id] == null) {
            scope.launch {
                check(id)
                if (tracks[id] != null) toggle(id) else Chrome.flash("Couldn't load favourite state")
            }
            return
        }
        val target = !tracks.getValue(id)
        tracks[id] = target
        val gen = generation
        // The core doesn't order concurrent writes, so a fast second toggle waits for the first.
        val previous = writes[id]
        writes[id] = scope.launch {
            previous?.join()
            try {
                setFavourite(FavouriteKind.TRACK, id, target)
                Chrome.flash(if (target) "Added to favourites" else "Removed from favourites")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // A newer toggle or a sign-out owns the state now.
                if (gen == generation && tracks[id] == target) tracks[id] = !target
                Chrome.flash(friendlyMessage(e))
            }
        }
    }
}
