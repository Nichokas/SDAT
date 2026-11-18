package com.nichokas.sdat.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.LocalRecent
import com.nichokas.sdat.core.OwnPlaylists
import com.nichokas.sdat.core.PageLoad
import com.nichokas.sdat.core.Recents
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
import uniffi.tonearm_ffi.TrackPager
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
        (state as? PageLoad.Ready)?.let { Downloads.note(it.value) }
    }
    // Offline, the album's downloaded tracks stand in for the page, as on iOS.
    val offline = if (state is PageLoad.Failed) Downloads.tracksInAlbum(id) else emptyList()
    val album = offline.firstOrNull()?.album
    if (album != null) {
        val (artist, year) = Downloads.albumInfo(id)
        return DetailPage(
            title = album.title,
            subtitle = joinDot(artist ?: offline.first().artistLine, year),
            subtitleRoute = null,
            cover = album.coverUrlLarge ?: album.coverUrl,
            mosaic = emptyList(),
            tracks = offline,
            loadingMore = false,
            footer = "Offline – showing downloaded songs only\n" + Fmt.summary(offline.size.toUInt(), offline.sumOf { it.durationSecs }),
            favourite = null,
            numbered = false,
            ownPlaylistId = null,
            onRemoved = {},
            onRetry = { attempt++ },
        )
    }
    LoadedPage(state, onRetry = { attempt++ }) { d ->
        val a = d.album
        LaunchedEffect(a.id) { Recents.add(LocalRecent(LocalRecent.Kind.Album, a.id, a.title, a.artistLine, a.coverUrl)) }
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

private data class PlaylistPage(
    val playlist: Playlist,
    val tracks: List<Track>,
    val loadingMore: Boolean = true,
    val paginationError: String? = null,
)

/**
 * A playlist page's data. Its pages keep streaming in while another page is pushed over it, and stop once it is
 * popped.
 */
private class PlaylistModel(private val id: String) : AutoCloseable {
    private val scope = MainScope()
    private var job: Job? = null
    private var pager: TrackPager? = null
    private var tracksFetched = 0
    var state by mutableStateOf<PageLoad<PlaylistPage>>(PageLoad.Loading)
        private set

    /** Starts loading unless a load is running or done. */
    fun load() {
        if (job?.isActive == true || state is PageLoad.Ready) return
        state = PageLoad.Loading
        // Kept out of the job: the ownership lookup has no deadline and must not hold off a Retry.
        scope.launch { OwnPlaylists.load() }
        job = scope.launch {
            val currentPager = playlistTracks(id)
            pager = currentPager
            val first = loadPlaylist(id, currentPager)
            state = first
            if (first is PageLoad.Ready) {
                tracksFetched = first.value.tracks.size
                loadMore(currentPager)
            } else {
                closePager()
            }
        }
    }

    fun retryMore() {
        val currentPager = pager ?: return
        if (job?.isActive == true || (state as? PageLoad.Ready)?.value?.paginationError == null) return
        job = scope.launch { loadMore(currentPager) }
    }

    private suspend fun loadMore(currentPager: TrackPager) {
        state = state.mapReady { it.copy(loadingMore = true, paginationError = null) }
        val error = try {
            pageThrough(tracksFetched, PLAYLIST_CAP, currentPager::nextPage) { page ->
                tracksFetched += page.size
                state = state.mapReady { it.copy(tracks = it.tracks + page) }
            }
            null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            friendlyMessage(e)
        }
        if (error == null) closePager()
        state = state.mapReady { it.copy(loadingMore = false, paginationError = error) }
    }

    fun removed(itemId: String) {
        state = state.mapReady { p -> p.copy(tracks = p.tracks.filterNot { it.playlistItemId == itemId }) }
    }

    private fun closePager() {
        pager?.close()
        pager = null
    }

    override fun close() {
        scope.cancel()
        closePager()
    }
}

/** Header and first page render together; the remaining pages stream in behind. */
@Composable
fun PlaylistView(id: String) {
    val model = rememberRetained(id) { PlaylistModel(id) }
    LaunchedEffect(model) { model.load() }
    LoadedPage(model.state, onRetry = model::load) { page ->
        val pl = page.playlist
        val own = OwnPlaylists.contains(pl.id)
        LaunchedEffect(pl.id) {
            Recents.add(LocalRecent(if (pl.isMix) LocalRecent.Kind.Mix else LocalRecent.Kind.Playlist, pl.id, pl.title, pl.creator.orEmpty(), pl.coverUrl))
        }
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
            paginationError = page.paginationError,
            onRetryMore = model::retryMore,
        )
    }
}

private fun <T> PageLoad<T>.mapReady(f: (T) -> T): PageLoad<T> = if (this is PageLoad.Ready) PageLoad.Ready(f(value)) else this

private suspend fun loadPlaylist(id: String, pager: TrackPager): PageLoad<PlaylistPage> = coroutineScope {
    val first = async { loadWithin(PLAYLIST_DEADLINE_MS) { pager.nextPage().orEmpty() } }
    val header = when (val result = loadWithin(PLAYLIST_DEADLINE_MS) { playlist(id) }) {
        is PageLoad.Ready -> result.value
        is PageLoad.Failed -> {
            first.cancel()
            return@coroutineScope result
        }
        PageLoad.Loading -> return@coroutineScope PageLoad.Loading
    }
    when (val result = first.await()) {
        is PageLoad.Ready -> PageLoad.Ready(PlaylistPage(header, result.value))
        is PageLoad.Failed -> result
        PageLoad.Loading -> PageLoad.Loading
    }
}
