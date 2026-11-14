package com.nichokas.sdat.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.LocalRecent
import com.nichokas.sdat.core.PagedLoader
import com.nichokas.sdat.core.Recents
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.friendlyMessage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.RecentItem
import uniffi.tonearm_ffi.RecentKind
import uniffi.tonearm_ffi.collectionAlbums
import uniffi.tonearm_ffi.collectionArtists
import uniffi.tonearm_ffi.collectionPlaylists
import uniffi.tonearm_ffi.collectionTracks
import uniffi.tonearm_ffi.recents

private val sections = listOf(
    Triple("Playlists", "music.note.list", Route.CollectionPlaylists),
    Triple("Albums", "square.stack", Route.CollectionAlbums),
    Triple("Tracks", "music.note", Route.CollectionTracks),
    Triple("Artists", "music.microphone", Route.CollectionArtists),
    Triple("Downloads", "arrow.down.circle", Route.Downloads),
)

/** TIDAL's listening history, then whatever was opened on this device that it doesn't list. */
fun recentTiles(remote: List<RecentItem>, local: List<LocalRecent>): List<ShelfItem> {
    val fromTidal = remote.map {
        val route = when (it.kind) {
            RecentKind.ALBUM -> Route.Album(it.id)
            RecentKind.ARTIST -> Route.Artist(it.id)
            RecentKind.PLAYLIST, RecentKind.MIX -> Route.Playlist(it.id)
        }
        ShelfItem(it.id, it.title, it.subtitle, it.coverUrl, route, round = it.kind == RecentKind.ARTIST)
    }
    val known = fromTidal.map { it.route }.toSet()
    val fromDevice = local.map {
        val route = if (it.kind == LocalRecent.Kind.Album) Route.Album(it.id) else Route.Playlist(it.id)
        ShelfItem(it.id, it.title, it.subtitle, it.coverUrl, route)
    }
    return fromTidal + fromDevice.filterNot { it.route in known }
}

/** TIDAL's recents for the Collection root, kept while the tab lives. */
private class RemoteRecents {
    var items by mutableStateOf(emptyList<RecentItem>())
    var error by mutableStateOf<String?>(null)

    suspend fun load() {
        try {
            items = recents()
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = friendlyMessage(e)
        }
    }
}

@Composable
fun CollectionView() {
    val remote = rememberRetained { RemoteRecents() }
    // Android has no pull to refresh here, so the history refreshes each time the tab root shows instead.
    LaunchedEffect(remote) { remote.load() }
    val tiles = recentTiles(remote.items, Recents.items)
    SdatPage("My Collection", collapsesChrome = true) {
        item(key = "sections") {
            Column(Modifier.padding(horizontal = ListGutter)) {
                sections.forEachIndexed { i, (title, symbol, route) ->
                    if (i > 0) HorizontalDivider(Modifier.padding(start = 52.dp), color = GlasmioTheme.colors.separator)
                    SectionRow(title, symbol) { Chrome.push(route) }
                }
            }
        }
        item(key = "recents-title") {
            Text(
                "Recent activity",
                style = IosTypography.title2,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(start = ListGutter, end = ListGutter, top = 28.dp, bottom = 12.dp),
            )
        }
        val error = remote.error
        if (error != null && tiles.isEmpty()) item(key = "recents-error") { ErrorBanner(error, Modifier.padding(horizontal = ListGutter)) }
        itemsIndexed(tiles.chunked(2), key = { i, _ -> "recents#$i" }) { _, pair ->
            Row(
                Modifier.fillMaxWidth().padding(start = ListGutter, end = ListGutter, bottom = 18.dp),
                horizontalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                pair.forEach { RecentTile(it, Modifier.weight(1f)) }
                if (pair.size == 1) Spacer(Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun SectionRow(title: String, symbol: String, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().height(52.dp).clickable(role = Role.Button, onClick = onClick),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Column(Modifier.width(36.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            SdatIcon(symbol, null, size = 24.dp, tint = GlasmioTheme.colors.accent)
        }
        Text(title, style = IosTypography.title3, modifier = Modifier.weight(1f))
        SdatIcon("chevron.right", null, size = 16.dp, tint = GlasmioTheme.colors.tertiaryContent)
    }
}

@Composable
private fun RecentTile(item: ShelfItem, modifier: Modifier) {
    Column(modifier.clickable(role = Role.Button) { Chrome.push(item.route) }, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        BoxWithConstraints(Modifier.fillMaxWidth()) { Cover(item.cover, maxWidth, radius = 10.dp, circle = item.round) }
        Text(item.title, style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
        if (item.subtitle.isNotEmpty()) {
            Text(item.subtitle, style = IosTypography.caption1, color = GlasmioTheme.colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

/** iOS PagedTracks with favourites: every listed track is a favourite, so their menus say so without asking. */
@Composable
fun CollectionTracksView() {
    val loader = rememberRetained { collectionTracks().let { p -> PagedLoader(pager = p, next = p::nextPage) } }
    val items = loader.list.items
    LaunchedEffect(items.size) { Favs.mark(items.map { it.id }) }
    SdatPage("Tracks", onBack = Chrome::pop) { pagedTracks(loader, "Tracks") }
}

@Composable
fun CollectionAlbumsView() {
    val loader = rememberRetained { collectionAlbums().let { p -> PagedLoader(pager = p, next = p::nextPage) } }
    SdatPage("Albums", onBack = Chrome::pop) { paged(loader, { it.id }) { items, i -> AlbumRow(items[i]) } }
}

@Composable
fun CollectionArtistsView() {
    val loader = rememberRetained { collectionArtists().let { p -> PagedLoader(pager = p, next = p::nextPage) } }
    SdatPage("Artists", onBack = Chrome::pop) { paged(loader, { it.id }) { items, i -> ArtistRow(items[i]) } }
}

@Composable
fun CollectionPlaylistsView() {
    val loader = rememberRetained { collectionPlaylists().let { p -> PagedLoader(pager = p, next = p::nextPage) } }
    SdatPage("Playlists", onBack = Chrome::pop) { paged(loader, { it.id }) { items, i -> PlaylistRow(items[i]) } }
}
