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

/** One favourite: a track, an album or playlist in the collection, or a followed artist. */
data class Fav(val kind: FavouriteKind, val id: String)

/** Cached favourite state, so menus, pages and follow buttons reflect it and writes reach the core in order. */
object Favs {
    private val scope = MainScope()
    private val known = mutableStateMapOf<Fav, Boolean>()
    private val writes = mutableMapOf<Fav, Job>()

    // Last value the server reported or accepted; a failed write falls back to it.
    private val confirmed = mutableMapOf<Fav, Boolean>()

    // Bumped on sign-out so requests from the previous session never write into the cache.
    private var generation = 0

    fun known(fav: Fav): Boolean? = known[fav]

    fun known(trackId: String): Boolean? = known(Fav(FavouriteKind.TRACK, trackId))

    /** Tracks the server listed as favourites; a write still in flight wins. */
    fun mark(ids: List<String>) {
        for (id in ids) {
            val fav = Fav(FavouriteKind.TRACK, id)
            if (writes[fav]?.isActive == true) continue
            known[fav] = true
            confirmed[fav] = true
        }
    }

    fun clear() {
        generation++
        known.clear()
        confirmed.clear()
        writes.values.forEach { it.cancel() }
        writes.clear()
    }

    suspend fun check(fav: Fav) {
        if (known[fav] != null) return
        val gen = generation
        try {
            val v = isFavourite(fav.kind, fav.id)
            // A toggle made while this was in flight wins.
            if (gen == generation && known[fav] == null) {
                known[fav] = v
                confirmed[fav] = v
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
        }
    }

    suspend fun check(trackId: String) = check(Fav(FavouriteKind.TRACK, trackId))

    /** Writes the state the control advertised, which stays right even if the cached state was unknown or stale. */
    fun set(fav: Fav, target: Boolean) {
        known[fav] = target
        val gen = generation
        // The core doesn't order concurrent writes, so a fast second toggle waits for the first.
        val previous = writes[fav]
        writes[fav] = scope.launch {
            previous?.join()
            // The core writes as whoever is signed in now, so a queued write from the previous account must not run.
            if (gen != generation) return@launch
            try {
                setFavourite(fav.kind, fav.id, target)
                if (gen != generation) return@launch
                confirmed[fav] = target
                done(fav.kind, target)?.let(Chrome::flash)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (gen != generation) return@launch
                // Only the newest write of this session decides what the cache shows.
                if (writes[fav] == coroutineContext.job) {
                    val last = confirmed[fav]
                    if (last == null) {
                        known.remove(fav)
                        check(fav)
                    } else {
                        known[fav] = last
                    }
                }
                Chrome.flash(friendlyMessage(e))
            }
        }
    }

    fun set(trackId: String, target: Boolean) = set(Fav(FavouriteKind.TRACK, trackId), target)

    private fun done(kind: FavouriteKind, on: Boolean) = when (kind) {
        FavouriteKind.TRACK -> if (on) "Added to favourites" else "Removed from favourites"
        FavouriteKind.ARTIST -> if (on) "Following" else "Unfollowed"
        // As on iOS, the collection button's own state is the confirmation.
        FavouriteKind.ALBUM, FavouriteKind.PLAYLIST, FavouriteKind.MIX -> null
    }
}
