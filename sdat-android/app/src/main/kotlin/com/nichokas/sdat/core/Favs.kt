package com.nichokas.sdat.core

import androidx.compose.runtime.mutableStateMapOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.FavouriteKind
import uniffi.tonearm_ffi.isFavourite
import uniffi.tonearm_ffi.setFavourite

/** Cached favourite state for tracks, so track menus reflect it. */
object Favs {
    private val scope = MainScope()
    private val tracks = mutableStateMapOf<String, Boolean>()

    fun known(id: String): Boolean? = tracks[id]
    fun mark(ids: List<String>, v: Boolean) = ids.forEach { tracks[it] = v }
    fun clear() = tracks.clear()

    suspend fun check(id: String) {
        if (tracks[id] != null) return
        try {
            val v = isFavourite(FavouriteKind.TRACK, id)
            // A toggle made while this was in flight wins.
            if (tracks[id] == null) tracks[id] = v
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
        }
    }

    fun toggle(id: String) {
        val target = !(tracks[id] ?: false)
        tracks[id] = target
        scope.launch {
            try {
                setFavourite(FavouriteKind.TRACK, id, target)
                Chrome.flash(if (target) "Added to favourites" else "Removed from favourites")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // A newer toggle owns the state now.
                if (tracks[id] == target) tracks[id] = !target
                Chrome.flash(friendlyMessage(e))
            }
        }
    }
}
