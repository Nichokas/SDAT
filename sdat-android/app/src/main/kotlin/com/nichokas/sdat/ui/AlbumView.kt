package com.nichokas.sdat.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.OwnPlaylists
import com.nichokas.sdat.core.PageLoad
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.TIMED_OUT
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.friendlyMessage
import com.nichokas.sdat.core.joinDot
import com.nichokas.sdat.core.mosaicCovers
import com.nichokas.sdat.core.pageThrough
import com.nichokas.sdat.core.playlistFavourite
import com.nichokas.sdat.core.year
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.tonearm_ffi.AlbumDetail
import uniffi.tonearm_ffi.FavouriteKind
import uniffi.tonearm_ffi.Playlist
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.album
import uniffi.tonearm_ffi.playlist
import uniffi.tonearm_ffi.playlistTracks

private const val ALBUM_DEADLINE_MS = 10_000L
private const val PLAYLIST_DEADLINE_MS = 12_000L
private const val PLAYLIST_CAP = 5000

/** Runs [op], turning a timeout or a failure into a message; a hung request can't keep a page on its skeleton. */
private suspend fun <T : Any> loadWithin(ms: Long, op: suspend () -> T): PageLoad<T> = try {
    withTimeoutOrNull(ms) { op() }?.let { PageLoad.Ready(it) } ?: PageLoad.Failed(TIMED_OUT)
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    PageLoad.Failed(friendlyMessage(e))
}

/** Skeleton, error with retry, or [ready], inside a page that can always go back. */
@Composable
private fun <T : Any> LoadedPage(state: PageLoad<T>, onRetry: () -> Unit, ready: @Composable (T) -> Unit) = when (state) {
    is PageLoad.Ready -> ready(state.value)
    is PageLoad.Failed -> DetailScaffold("", null) { item { LoadFailed(state.message, onRetry) } }
    PageLoad.Loading -> DetailScaffold("", null) { item { DetailSkeleton() } }
}

@Composable
fun AlbumView(id: String) {
    var state by rememberRetained(id) { mutableStateOf<PageLoad<AlbumDetail>>(PageLoad.Loading) }
    var attempt by remember { mutableIntStateOf(0) }
    LaunchedEffect(id, attempt) {
        if (state is PageLoad.Ready) return@LaunchedEffect
        state = PageLoad.Loading
        state = loadWithin(ALBUM_DEADLINE_MS) { album(id) }
    }
    LoadedPage(state, onRetry = { attempt++ }) { d ->
        val a = d.album
        DetailPage(
            title = a.title,
            subtitle = joinDot(a.artistLine, year(a.releaseDate)),
            subtitleRoute = a.artists.firstOrNull()?.let { Route.Artist(it.id) },
            cover = a.coverUrlLarge ?: a.coverUrl,
            mosaic = emptyList(),
            tracks = d.tracks,
            loadingMore = false,
            footer = Fmt.summary(a.trackCount, a.durationSecs),
            favourite = FavouriteKind.ALBUM to a.id,
            numbered = true,
            ownPlaylistId = null,
            onRemoved = {},
        )
    }
}

/** A playlist's header and the tracks loaded so far; [loadingMore] while later pages still stream in. */
data class PlaylistPage(val playlist: Playlist, val tracks: List<Track>, val loadingMore: Boolean)

/**
 * A playlist page's data. Its pages keep streaming in while another page is pushed over it, and stop once it is
 * popped.
 */
private class PlaylistModel(private val id: String) : AutoCloseable {
    private val scope = MainScope()
    private var job: Job? = null
    var state by mutableStateOf<PageLoad<PlaylistPage>>(PageLoad.Loading)
        private set

    /** Starts loading unless a load is running or done. */
    fun load() {
        if (job?.isActive == true || state is PageLoad.Ready) return
        state = PageLoad.Loading
        job = scope.launch {
            launch { OwnPlaylists.load() }
            loadPlaylist(id) { update -> state = update(state) }
        }
    }

    fun removed(t: Track) {
        state = state.mapReady { p ->
            val i = p.tracks.indexOfFirst { it.id == t.id }
            if (i < 0) p else p.copy(tracks = p.tracks.toMutableList().apply { removeAt(i) })
        }
    }

    override fun close() = scope.cancel()
}

/** Header and first page render together; the remaining pages stream in behind. */
@Composable
fun PlaylistView(id: String) {
    val model = rememberRetained(id) { PlaylistModel(id) }
    LaunchedEffect(model) { model.load() }
    LoadedPage(model.state, onRetry = model::load) { page ->
        val pl = page.playlist
        val own = OwnPlaylists.contains(pl.id)
        DetailPage(
            title = pl.title,
            subtitle = if (own) "My playlist" else joinDot(pl.creator, if (pl.isMix) "Mix" else "Playlist"),
            subtitleRoute = null,
            cover = pl.coverUrlLarge ?: pl.coverUrl,
            mosaic = mosaicCovers(page.tracks.map { it.album?.coverUrl }),
            tracks = page.tracks,
            loadingMore = page.loadingMore,
            footer = listOfNotNull(Fmt.summary(pl.trackCount, pl.durationSecs), Fmt.updated(pl.updatedAt)).joinToString("\n"),
            favourite = playlistFavourite(pl.isMix, own)?.let { it to pl.id },
            numbered = false,
            ownPlaylistId = if (own) pl.id else null,
            onRemoved = model::removed,
        )
    }
}

private fun <T> PageLoad<T>.mapReady(f: (T) -> T): PageLoad<T> = if (this is PageLoad.Ready) PageLoad.Ready(f(value)) else this

/** [update] gets a function of the current state, so rows removed meanwhile stay removed as pages arrive. */
private suspend fun loadPlaylist(id: String, update: ((PageLoad<PlaylistPage>) -> PageLoad<PlaylistPage>) -> Unit) {
    playlistTracks(id).use { pager ->
        coroutineScope {
            val first = async {
                try {
                    pager.nextPage().orEmpty()
                } catch (e: CancellationException) {
                    throw e
                } catch (_: Exception) {
                    emptyList()
                }
            }
            val header = when (val h = loadWithin(PLAYLIST_DEADLINE_MS) { playlist(id) }) {
                is PageLoad.Ready -> h.value
                is PageLoad.Failed -> {
                    first.cancel()
                    update { h }
                    return@coroutineScope
                }
                PageLoad.Loading -> return@coroutineScope
            }
            val tracks = first.await()
            update { PageLoad.Ready(PlaylistPage(header, tracks, loadingMore = true)) }
            pageThrough(tracks.size, PLAYLIST_CAP, pager::nextPage) { page ->
                update { s -> s.mapReady { it.copy(tracks = it.tracks + page) } }
            }
            update { s -> s.mapReady { it.copy(loadingMore = false) } }
        }
    }
}
