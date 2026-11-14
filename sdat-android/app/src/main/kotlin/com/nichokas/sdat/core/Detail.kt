package com.nichokas.sdat.core

import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.ArtistOverview
import uniffi.tonearm_ffi.FavouriteKind

/** What a detail page shows while it loads its data. */
sealed interface PageLoad<out T> {
    data object Loading : PageLoad<Nothing>
    data class Failed(val message: String) : PageLoad<Nothing>
    data class Ready<T>(val value: T) : PageLoad<T>
}

const val TIMED_OUT = "The request timed out."

/** "2021" from TIDAL's "2021-03-05". */
fun year(releaseDate: String?): String? = releaseDate?.take(4)

/** "Artist · 2021", leaving out whichever part is missing. */
fun joinDot(vararg parts: String?): String = parts.filterNotNull().joinToString(" · ")

/** Four distinct album covers for a playlist's 2x2 artwork, or none so the playlist's own cover shows. */
fun mosaicCovers(covers: List<String?>): List<String> {
    val distinct = covers.filterNotNull().distinct().take(4)
    return if (distinct.size == 4) distinct else emptyList()
}

/** The user's own playlists get no favourite button: they are already in the collection. */
fun playlistFavourite(isMix: Boolean, own: Boolean): FavouriteKind? = when {
    own -> null
    isMix -> FavouriteKind.MIX
    else -> FavouriteKind.PLAYLIST
}

/**
 * Pulls pages from [next] into [onPage] until one is missing or empty, one fails, or [cap] items are in, counting
 * the [loaded] ones. Pages, not the whole list, go out: the page may remove rows while later pages arrive.
 */
suspend fun <T> pageThrough(loaded: Int, cap: Int, next: suspend () -> List<T>?, onPage: (List<T>) -> Unit) {
    var count = loaded
    while (count < cap) {
        val page = try {
            next()
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            null
        }
        if (page.isNullOrEmpty()) return
        onPage(page)
        count += page.size
    }
}

/** In-memory artist data so artist pages open instantly, from a cached overview or the row that was tapped. */
object ArtistCache {
    data class Hint(val name: String, val picture: String?)

    val overviews = mutableMapOf<String, ArtistOverview>()
    val hints = mutableMapOf<String, Hint>()

    fun hint(id: String, name: String, picture: String?) { hints[id] = Hint(name, picture) }
}
