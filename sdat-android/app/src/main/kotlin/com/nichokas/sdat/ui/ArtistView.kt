package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.ArtistCache
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Fav
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.PageLoad
import com.nichokas.sdat.core.friendlyMessage
import com.nichokas.sdat.ui.kit.Button
import com.nichokas.sdat.ui.kit.ButtonSize
import com.nichokas.sdat.ui.kit.Emphasis
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.isActive
import kotlinx.coroutines.job
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.ArtistOverview
import uniffi.tonearm_ffi.FavouriteKind
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.artistOverview
import uniffi.tonearm_ffi.artistTopTracksPager

// A follow tapped just before going back still goes through.

/** Opens at once with the header from the cached overview or the tapped row, then loads the rest. */
@Composable
fun ArtistView(id: String) {
    var state by remember(id) { mutableStateOf<PageLoad<ArtistOverview>>(ArtistCache.overviews[id]?.let { PageLoad.Ready(it) } ?: PageLoad.Loading) }
    var attempt by remember { mutableIntStateOf(0) }
    val scope = rememberCoroutineScope()
    val more = remember(id) { MoreTopTracks(id, scope) }
    DisposableEffect(more) { onDispose { more.close() } }
    LaunchedEffect(id, attempt) {
        try {
            val o = artistOverview(id)
            ArtistCache.overviews[id] = o
            // The extra tracks were picked against the old list.
            if ((state as? PageLoad.Ready)?.value?.topTracks?.map { it.id } != o.topTracks.map { it.id }) more.reset()
            state = PageLoad.Ready(o)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            // A cached overview stays up when the refresh fails.
            if (state !is PageLoad.Ready) state = PageLoad.Failed(friendlyMessage(e))
        }
    }
    val overview = (state as? PageLoad.Ready)?.value
    val hint = ArtistCache.hints[id]
    val name = overview?.artist?.name ?: hint?.name.orEmpty()
    val picture = listOf(overview?.artist?.pictureUrl, hint?.picture).firstOrNull { !it.isNullOrEmpty() }
    SdatPage(name, onBack = Chrome::pop) {
        item(key = "header") {
            Column(
                Modifier.fillMaxWidth().padding(vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Cover(picture, 180.dp, circle = true)
                if (name.isEmpty()) {
                    Skeleton(width = 160.dp, height = 28.dp)
                } else {
                    Text(name, style = SdatType.title1, fontWeight = FontWeight.Bold, textAlign = TextAlign.Center)
                }
                FollowButton(id)
            }
        }
        when (val s = state) {
            is PageLoad.Ready -> sections(s.value, more)
            is PageLoad.Failed -> item(key = "error") {
                Column(Modifier.fillMaxWidth().padding(horizontal = ListGutter, vertical = 16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    ErrorBanner(s.message)
                    Button({ state = PageLoad.Loading; attempt++ }, Modifier.padding(top = 12.dp), size = ButtonSize.Medium) { Text("Retry") }
                }
            }
            PageLoad.Loading -> items(6) { Box(Modifier.padding(horizontal = ListGutter)) { RowSkeleton() } }
        }
    }
}

internal interface TopTracksSource : AutoCloseable {
    suspend fun next(limit: UInt): List<Track>
    fun isDone(): Boolean
}

private fun openTopTracks(artistId: String, shownIds: List<String>): TopTracksSource {
    val pager = artistTopTracksPager(artistId, shownIds)
    return object : TopTracksSource {
        override suspend fun next(limit: UInt) = pager.next(limit)
        override fun isDone() = pager.isDone()
        override fun close() = pager.close()
    }
}

/** Top tracks past the overview's few, a page per "Show More". */
internal class MoreTopTracks(
    private val artistId: String,
    private val scope: CoroutineScope,
    private val open: (String, List<String>) -> TopTracksSource = ::openTopTracks,
    private val onError: (String) -> Unit = Chrome::flash,
) : AutoCloseable {
    var tracks by mutableStateOf(emptyList<Track>())
        private set
    var canLoad by mutableStateOf(true)
        private set
    var loading by mutableStateOf(false)
        private set

    private var pager: TopTracksSource? = null
    private var job: Job? = null
    private var generation = 0
    private var closed = false

    fun reset() {
        generation++
        job?.cancel()
        job = null
        pager?.close()
        pager = null
        loading = false
        tracks = emptyList()
        canLoad = true
    }

    fun load(shown: List<Track>): Job? {
        if (closed || loading || !canLoad || !scope.isActive) return null
        val gen = ++generation
        loading = true
        val shownIds = shown.map { it.id }
        val current = scope.launch {
            try {
                if (gen != generation) return@launch
                val p = pager ?: open(artistId, shownIds).also { pager = it }
                val page = p.next(PAGE)
                if (gen != generation || pager !== p) return@launch
                tracks = tracks + page
                canLoad = !p.isDone()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (gen == generation) onError(friendlyMessage(e))
            } finally {
                if (gen == generation) {
                    if (job === coroutineContext.job) job = null
                    loading = false
                }
            }
        }
        if (gen == generation && current.isActive) job = current
        return current
    }

    override fun close() {
        closed = true
        generation++
        job?.cancel()
        job = null
        pager?.close()
        pager = null
        loading = false
    }

    private companion object {
        const val PAGE = 10u
    }
}

private fun LazyListScope.sections(o: ArtistOverview, more: MoreTopTracks) {
    val top = o.topTracks + more.tracks
    if (top.isNotEmpty()) {
        item(key = "top-title") { SectionTitle("Top Tracks") }
        item(key = "top-buttons") {
            val player = LocalTrackPlayer.current
            PlayShuffleButtons(
                play = { player.playInOrder(top, o.artist.name) },
                shuffle = { player.playShuffled(top, o.artist.name) },
                compact = true,
                modifier = Modifier.padding(start = ListGutter, end = ListGutter, bottom = 8.dp),
            )
        }
        itemsIndexed(top, key = { i, t -> "top:${t.id}#$i" }) { i, t ->
            val player = LocalTrackPlayer.current
            Column(Modifier.padding(horizontal = ListGutter)) {
                TrackMenuRow(t, onClick = { player.play(top, i, o.artist.name) }) {
                    TrackRow(t, Modifier.heightIn(min = 72.dp), coverSize = 56.dp)
                }
                if (i < top.lastIndex) {
                    Box(Modifier.padding(start = 68.dp).fillMaxWidth().height(0.5.dp).background(SdatTheme.colors.separator))
                }
            }
        }
        if (more.canLoad) {
            item(key = "top-more") {
                Box(Modifier.fillMaxWidth().padding(top = 8.dp), contentAlignment = Alignment.Center) {
                    Button({ more.load(top) }, enabled = !more.loading, size = ButtonSize.Medium) { Text("Show More") }
                }
            }
        }
    }
    if (o.albums.isNotEmpty()) {
        item(key = "albums-title") { SectionTitle("Albums") }
        item(key = "albums") { AlbumShelf(o.albums) }
    }
    if (o.artist.bio.isNotEmpty()) {
        item(key = "about-title") { SectionTitle("About") }
        item(key = "about") { BioText(o.artist.bio, Modifier.padding(horizontal = ListGutter)) }
    }
}

@Composable
private fun SectionTitle(text: String) = SectionHeader(text, Modifier.padding(top = 24.dp, bottom = 10.dp), style = SdatType.title2)

/** Follow / Following: optimistic, saved as an artist favourite, reverted on error. */
@Composable
fun FollowButton(artistId: String) {
    val fav = Fav(FavouriteKind.ARTIST, artistId)
    // As on iOS, a failed check leaves the button usable as "Follow".
    var checked by remember(fav) { mutableStateOf(false) }
    LaunchedEffect(fav) { Favs.check(fav); checked = true }
    val following = Favs.known(fav)
    val on = following == true
    Button(
        onClick = { Favs.set(fav, !on) },
        enabled = following != null || checked,
        emphasis = if (on) Emphasis.Prominent else Emphasis.Regular,
        size = ButtonSize.Medium,
    ) {
        SdatIcon(if (on) "checkmark" else "plus", null, size = 16.dp)
        Text(
            if (on) "Following" else "Follow",
            style = SdatType.subheadline,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(start = 6.dp, end = 6.dp),
        )
    }
}

/** Six lines, with More / Less only when the text is longer than that. */
@Composable
private fun BioText(text: String, modifier: Modifier = Modifier) {
    var expanded by remember { mutableStateOf(false) }
    var overflows by remember { mutableStateOf(false) }
    Column(modifier, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(
            text,
            style = SdatType.subheadline,
            color = SdatTheme.colors.secondaryContent,
            maxLines = if (expanded) Int.MAX_VALUE else 6,
            overflow = TextOverflow.Ellipsis,
            onTextLayout = { if (!expanded) overflows = it.hasVisualOverflow },
        )
        if (overflows || expanded) {
            Text(
                if (expanded) "Less" else "More",
                style = SdatType.subheadline,
                fontWeight = FontWeight.SemiBold,
                color = SdatTheme.colors.accent,
                modifier = Modifier.clickable(role = Role.Button) { expanded = !expanded },
            )
        }
    }
}
