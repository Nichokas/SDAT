package com.nichokas.sdat.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.ArtistCache
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.PagedLoader
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.SearchModel
import com.nichokas.sdat.core.SearchState
import com.nichokas.sdat.core.isTopArtist
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.ui.GlasmioSearchBar
import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.Album
import uniffi.tonearm_ffi.ArtistOverview
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Playlist
import uniffi.tonearm_ffi.SearchResults
import uniffi.tonearm_ffi.artistOverview
import uniffi.tonearm_ffi.search
import uniffi.tonearm_ffi.searchAlbums
import uniffi.tonearm_ffi.searchArtists
import uniffi.tonearm_ffi.searchPlaylists
import uniffi.tonearm_ffi.searchTracks

private enum class SearchFilter(val label: String) { Top("Top"), Tracks("Tracks"), Artists("Artists"), Albums("Albums"), Playlists("Playlists") }

/** A search's results, with the first artist's overview when it matches the query well enough to lead. */
private data class SearchHit(val results: SearchResults, val topArtist: ArtistOverview?)

private suspend fun searchHit(q: String): SearchHit {
    val results = search(q)
    val artist = results.artists.firstOrNull()?.takeIf { isTopArtist(it.name, q) }
    val overview = artist?.let {
        ArtistCache.overviews[it.id] ?: try {
            artistOverview(it.id).also { o -> ArtistCache.overviews[it.id] = o }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // As on iOS, the top result is a bonus: without it the plain results still show.
            null
        }
    }
    return SearchHit(results, overview)
}

/** The Search tab: Explore while the field is empty, otherwise the top results or one kind of result, paged. */
@Composable
fun SearchView() {
    val model = rememberRetained { SearchModel(search = ::searchHit) }
    var field by rememberSaveable(stateSaver = TextFieldValue.Saver) { mutableStateOf(TextFieldValue(model.state.query)) }
    var active by remember { mutableStateOf(false) }
    var filter by rememberSaveable { mutableStateOf(SearchFilter.Top) }
    val keyboard = LocalSoftwareKeyboardController.current
    val state = model.state
    LaunchedEffect(state.query) { model.settle() }
    val explore = rememberExplore("explore")
    val shown = state.shown
    // Only the selected filter gets a pager, made again for each query.
    val paging = if (shown.isEmpty()) null else filter
    val tracks = if (paging == SearchFilter.Tracks) rememberRetained(shown, paging) { PagedLoader(next = searchTracks(shown)::nextPage) } else null
    val artists = if (paging == SearchFilter.Artists) rememberRetained(shown, paging) { PagedLoader(next = searchArtists(shown)::nextPage) } else null
    val albums = if (paging == SearchFilter.Albums) rememberRetained(shown, paging) { PagedLoader(next = searchAlbums(shown)::nextPage) } else null
    val playlists = if (paging == SearchFilter.Playlists) rememberRetained(shown, paging) { PagedLoader(next = searchPlaylists(shown)::nextPage) } else null

    SdatPage("Search", collapsesChrome = true) {
        item(key = "search-field") {
            GlasmioSearchBar(
                query = field,
                onQueryChange = { field = it; model.edit(it.text) },
                active = active,
                onActiveChange = { active = it },
                onClose = { field = TextFieldValue(); model.edit(""); active = false },
                placeholder = "Artists, songs, albums",
                closeContentDescription = "Cancel",
                onSearch = { keyboard?.hide() },
                modifier = Modifier.padding(horizontal = ListGutter, vertical = 8.dp),
            )
        }
        if (shown.isEmpty()) {
            explore(explore)
            return@SdatPage
        }
        item(key = "search-filters") { Filters(filter) { filter = it } }
        when {
            tracks != null -> pagedTracks(tracks, "Search")
            artists != null -> paged(artists, { it.id }) { items, i -> ArtistRow(items[i]) }
            albums != null -> paged(albums, { it.id }) { items, i -> AlbumRow(items[i]) }
            playlists != null -> paged(playlists, { it.id }) { items, i -> PlaylistRow(items[i]) }
            else -> topResults(state)
        }
    }
}

@Composable
private fun Filters(selected: SearchFilter, onSelect: (SearchFilter) -> Unit) {
    LazyRow(
        Modifier.fillMaxWidth(),
        contentPadding = PaddingValues(horizontal = ListGutter, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(SearchFilter.entries) { f -> Chip(f.label, selected = f == selected) { onSelect(f) } }
    }
}

private fun LazyListScope.header(title: String) = item(key = "header-$title") {
    SectionHeader(title, Modifier.padding(top = 20.dp, bottom = 4.dp))
}

private fun LazyListScope.topResults(state: SearchState<SearchHit>) {
    state.error?.let { item(key = "search-error") { ErrorBanner(it, Modifier.padding(horizontal = ListGutter, vertical = 8.dp)) } }
    val hit = state.result
    if (hit == null) {
        if (state.searching) {
            item(key = "search-skeleton") {
                Column(Modifier.padding(ListGutter), verticalArrangement = Arrangement.spacedBy(14.dp)) { repeat(8) { RowSkeleton() } }
            }
        }
        return
    }
    val r = hit.results
    if (r.tracks.isEmpty() && r.albums.isEmpty() && r.artists.isEmpty() && r.playlists.isEmpty()) {
        item(key = "search-empty") {
            Unavailable("magnifyingglass", "No Results for “${state.shown}”", "Check the spelling or try a new search.")
        }
    }
    hit.topArtist?.let { o ->
        header("Top Result")
        item(key = "top-artist") { TopArtist(o) }
        trackItems(o.topTracks, o.artist.name, "top", limit = 5)
        if (o.albums.isNotEmpty()) {
            item(key = "top-albums") {
                AlbumShelf(o.albums, Modifier.padding(vertical = 6.dp))
            }
        }
    }
    if (r.artists.isNotEmpty()) {
        header("Artists")
        artistItems(r.artists.take(4))
    }
    if (r.tracks.isNotEmpty()) {
        header("Tracks")
        trackItems(r.tracks, "Search", "tracks")
    }
    if (r.albums.isNotEmpty()) {
        header("Albums")
        albumItems(r.albums)
    }
    if (r.playlists.isNotEmpty()) {
        header("Playlists")
        playlistItems(r.playlists)
    }
}

private fun LazyListScope.artistItems(artists: List<ArtistRef>) =
    artists.forEachIndexed { i, a -> item(key = "artist:${a.id}#$i") { ArtistRow(a, subtitle = "Artist") } }

private fun LazyListScope.albumItems(albums: List<Album>) =
    albums.forEachIndexed { i, a -> item(key = "album:${a.id}#$i") { AlbumRow(a) } }

private fun LazyListScope.playlistItems(playlists: List<Playlist>) =
    playlists.forEachIndexed { i, p -> item(key = "playlist:${p.id}#$i") { PlaylistRow(p) } }

@Composable
private fun TopArtist(o: ArtistOverview) {
    Row(
        Modifier.fillMaxWidth().clickable(role = Role.Button) {
            ArtistCache.overviews[o.artist.id] = o
            Chrome.push(Route.Artist(o.artist.id))
        }
            .padding(horizontal = ListGutter, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Cover(o.artist.pictureUrl, 72.dp, circle = true)
        Column(Modifier.weight(1f)) {
            Text(o.artist.name, style = IosTypography.title3, fontWeight = FontWeight.Bold)
            Text("Artist", style = IosTypography.subheadline, color = GlasmioTheme.colors.secondaryContent)
        }
        SdatIcon("chevron.right", null, size = 14.dp, tint = GlasmioTheme.colors.tertiaryContent)
    }
}
