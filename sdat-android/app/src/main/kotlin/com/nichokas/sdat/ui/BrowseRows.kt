package com.nichokas.sdat.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.ArtistCache
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.PagedLoader
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.year
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassEmphasis
import io.github.neoruaa.glasmio.ui.GlasmioButton
import io.github.neoruaa.glasmio.ui.GlasmioButtonSize
import uniffi.tonearm_ffi.Album
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Playlist
import uniffi.tonearm_ffi.Track

/** Side margin of browse lists, as iOS plain lists. Lists run edge to edge and each row keeps this margin. */
val ListGutter = 16.dp

@Composable
private fun RouteRow(route: Route, onOpen: () -> Unit = {}, content: @Composable () -> Unit) {
    Box(
        Modifier.fillMaxWidth().clickable(role = Role.Button) { onOpen(); Chrome.push(route) }
            .padding(horizontal = ListGutter, vertical = 6.dp),
    ) { content() }
}

@Composable
fun AlbumRow(album: Album) = RouteRow(Route.Album(album.id)) { MediaRow(album.title, album.artistLine, album.coverUrl) }

/** The artist page opens with the row's name and picture while it loads the rest. */
@Composable
fun ArtistRow(artist: ArtistRef, subtitle: String = "") =
    RouteRow(Route.Artist(artist.id), onOpen = { ArtistCache.hint(artist.id, artist.name, artist.pictureUrl) }) {
        MediaRow(artist.name, subtitle, artist.pictureUrl, round = true)
    }

@Composable
fun PlaylistRow(playlist: Playlist) =
    RouteRow(Route.Playlist(playlist.id)) { MediaRow(playlist.title, playlist.creator.orEmpty(), playlist.coverUrl) }

@Composable
fun SectionHeader(
    title: String,
    modifier: Modifier = Modifier,
    style: TextStyle = IosTypography.title3,
    trailing: @Composable () -> Unit = {},
) {
    Row(modifier.fillMaxWidth().padding(horizontal = ListGutter), verticalAlignment = Alignment.CenterVertically) {
        Text(title, style = style, fontWeight = FontWeight.Bold, modifier = Modifier.weight(1f))
        trailing()
    }
}

/** One card of a horizontal shelf. */
data class ShelfItem(val key: String, val title: String, val subtitle: String, val cover: String?, val route: Route, val round: Boolean = false)

fun Album.shelfItem(subtitle: String = artistLine) = ShelfItem(id, title, subtitle, coverUrl, Route.Album(id))
fun Playlist.shelfItem() = ShelfItem(id, title, creator.orEmpty(), coverUrl, Route.Playlist(id))
fun ArtistRef.shelfItem() = ShelfItem(id, name, "", pictureUrl, Route.Artist(id), round = true)

/** Horizontal carousel of 140 dp covers that push their page. */
@Composable
fun MediaShelf(items: List<ShelfItem>, modifier: Modifier = Modifier) {
    LazyRow(
        modifier.fillMaxWidth(),
        contentPadding = PaddingValues(horizontal = ListGutter),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(items, key = { it.key }) { item ->
            Column(
                Modifier.width(140.dp).clickable(role = Role.Button) { Chrome.push(item.route) },
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Cover(item.cover, 140.dp, radius = 10.dp, circle = item.round)
                Text(item.title, style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (item.subtitle.isNotEmpty()) {
                    Text(item.subtitle, style = IosTypography.caption1, color = GlasmioTheme.colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
    }
}

/** An artist's albums, each under its year. */
@Composable
fun AlbumShelf(albums: List<Album>, modifier: Modifier = Modifier) =
    MediaShelf(albums.map { it.shelfItem(subtitle = year(it.releaseDate) ?: it.artistLine) }, modifier)

/** Play and Shuffle side by side; [compact] is the small pair under an artist's Top Tracks. */
@Composable
fun PlayShuffleButtons(play: () -> Unit, shuffle: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, compact: Boolean = false) {
    val size = if (compact) GlasmioButtonSize.Small else GlasmioButtonSize.Large
    val style = if (compact) IosTypography.subheadline.copy(fontWeight = FontWeight.SemiBold) else IosTypography.headline
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        GlasmioButton(play, Modifier.weight(1f), enabled = enabled, emphasis = GlassEmphasis.Prominent, size = size) {
            SdatIcon("play.fill", null, size = if (compact) 16.dp else 20.dp, tint = Color.White)
            Text("Play", style = style, color = Color.White, modifier = Modifier.padding(start = 6.dp))
        }
        GlasmioButton(shuffle, Modifier.weight(1f), enabled = enabled, size = size) {
            SdatIcon("shuffle", null, size = if (compact) 16.dp else 20.dp, tint = Color.White)
            Text("Shuffle", style = style, color = Color.White, modifier = Modifier.padding(start = 6.dp))
        }
    }
}

/** iOS ContentUnavailableView: a symbol, a title and a hint, centred. */
@Composable
fun Unavailable(symbol: String, title: String, message: String? = null) {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 48.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        SdatIcon(symbol, null, size = 44.dp, tint = GlasmioTheme.colors.secondaryContent)
        Text(title, style = IosTypography.title2, fontWeight = FontWeight.Bold, textAlign = TextAlign.Center)
        if (message != null) Text(message, style = IosTypography.body, color = GlasmioTheme.colors.secondaryContent, textAlign = TextAlign.Center)
    }
}

/** A track row inside the browse gutter; tapping it plays all of [tracks] from it. */
@Composable
fun TrackItem(
    tracks: List<Track>,
    index: Int,
    source: String?,
    showCover: Boolean = true,
    numbered: Boolean = false,
    ownPlaylistId: String? = null,
    onRemoved: ((Track) -> Unit)? = null,
) {
    val player = LocalTrackPlayer.current
    val t = tracks[index]
    TrackMenuRow(t, onClick = { player.play(tracks, index, source) }, ownPlaylistId = ownPlaylistId, onRemoved = { onRemoved?.invoke(t) }) {
        TrackRow(t, Modifier.padding(horizontal = ListGutter, vertical = 6.dp), showCover = showCover, number = if (numbered) index + 1 else null)
    }
}

/**
 * The first [limit] of [tracks] as [TrackItem]s. [keyPrefix] keeps the same track in two sections of one list apart,
 * and the index keeps duplicates apart, so a shifted row gets fresh menu state instead of another track's.
 */
fun LazyListScope.trackItems(
    tracks: List<Track>,
    source: String?,
    keyPrefix: String = "track",
    limit: Int = tracks.size,
    showCover: Boolean = true,
    numbered: Boolean = false,
    ownPlaylistId: String? = null,
    onRemoved: ((Track) -> Unit)? = null,
) {
    itemsIndexed(tracks.take(limit), key = { i, t -> "$keyPrefix:${t.id}#$i" }) { i, _ ->
        TrackItem(tracks, i, source, showCover, numbered, ownPlaylistId, onRemoved)
    }
}

/**
 * The iOS infinite list: rows, then a spinner that loads the next page, an error with Retry, or the empty state.
 * [key] must stay unique across the list's items.
 */
fun <T> LazyListScope.paged(loader: PagedLoader<T>, key: (T) -> Any, row: @Composable (items: List<T>, index: Int) -> Unit) {
    val list = loader.list
    itemsIndexed(list.items, key = { i, t -> "${key(t)}#$i" }) { i, _ ->
        LaunchedEffect(i, list.items.size) { if (list.wantsMore(i)) loader.more() }
        row(list.items, i)
    }
    val error = list.error
    when {
        error != null -> item(key = "paged-error") {
            Column(Modifier.padding(horizontal = ListGutter, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                ErrorBanner(error)
                Chip("Retry") { loader.retry() }
            }
        }
        !list.done -> item(key = "paged-more") {
            LaunchedEffect(Unit) { loader.more() }
            Box(Modifier.fillMaxWidth().padding(16.dp), contentAlignment = Alignment.Center) {
                CircularProgressIndicator(Modifier.width(24.dp), color = GlasmioTheme.colors.secondaryContent, strokeWidth = 2.dp)
            }
        }
        list.isEmpty -> item(key = "paged-empty") { Unavailable("tray", "Nothing here yet") }
    }
}

/** iOS PagedTracks: Play and Shuffle over the tracks loaded so far, then the paged rows. */
fun LazyListScope.pagedTracks(loader: PagedLoader<Track>, source: String) {
    val loaded = loader.list.items
    if (loaded.isNotEmpty()) {
        item(key = "paged-play") {
            val player = LocalTrackPlayer.current
            PlayShuffleButtons(
                play = { player.playInOrder(loaded, source) },
                shuffle = { player.playShuffled(loaded, source) },
                modifier = Modifier.padding(horizontal = ListGutter, vertical = 8.dp),
            )
        }
    }
    paged(loader, { it.id }) { items, i -> TrackItem(items, i, source) }
}
