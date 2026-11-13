package com.nichokas.sdat.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListScope
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
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.friendlyMessage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.ExplorePage
import uniffi.tonearm_ffi.ExploreSection
import uniffi.tonearm_ffi.PageLink
import uniffi.tonearm_ffi.explorePage

/** One TIDAL Explore page, loaded once; [asList] holds the titles of link sections shown as a list instead of chips. */
class ExploreState(private val path: String) {
    var page by mutableStateOf<ExplorePage?>(null)
        private set
    var error by mutableStateOf<String?>(null)
    var asList by mutableStateOf(emptySet<String>())

    suspend fun load() {
        if (page != null || error != null) return
        try {
            page = explorePage(path)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = friendlyMessage(e)
        }
    }

    fun toggleList(title: String) { asList = if (title in asList) asList - title else asList + title }
}

@Composable
fun rememberExplore(path: String): ExploreState {
    val state = rememberRetained(path) { ExploreState(path) }
    // Retry clears the error, which runs this again.
    LaunchedEffect(state, state.error) { state.load() }
    return state
}

/** A pushed Explore page, such as a genre or a mood. */
@Composable
fun ExploreView(path: String, title: String) {
    val state = rememberExplore(path)
    SdatPage(title, onBack = Chrome::pop) { explore(state) }
}

/** TIDAL Explore: chip clouds (Genres, Moods & Activities, Decades...) and media rows. */
@OptIn(ExperimentalLayoutApi::class)
fun LazyListScope.explore(state: ExploreState) {
    val error = state.error
    if (error != null) {
        item(key = "explore-error") {
            Column(Modifier.padding(horizontal = ListGutter, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                ErrorBanner(error)
                Chip("Retry") { state.error = null }
            }
        }
    }
    val page = state.page
    if (page == null) {
        if (error == null) {
            item(key = "explore-skeleton") {
                FlowRow(
                    Modifier.padding(ListGutter),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) { repeat(14) { i -> Skeleton(width = (60 + (i * 37) % 70).dp, height = 34.dp, radius = 17.dp) } }
            }
        }
        return
    }
    page.sections.forEachIndexed { i, s -> section(state, s, "s$i") }
}

@OptIn(ExperimentalLayoutApi::class)
private fun LazyListScope.section(state: ExploreState, s: ExploreSection, key: String) {
    val list = s.title in state.asList
    item(key = "$key-header") {
        Spacer(Modifier.height(24.dp))
        if (s.title.isNotEmpty()) {
            SectionHeader(s.title, Modifier.padding(bottom = 12.dp)) {
                if (s.links.isNotEmpty()) {
                    Text(
                        if (list) "View as chips" else "View as list",
                        style = IosTypography.subheadline,
                        color = GlasmioTheme.colors.accent,
                        modifier = Modifier.clickable(role = Role.Button) { state.toggleList(s.title) },
                    )
                }
            }
        }
    }
    if (s.links.isNotEmpty()) {
        if (list) {
            s.links.forEachIndexed { i, l -> item(key = "$key-link$i") { LinkRow(l) } }
        } else {
            item(key = "$key-chips") {
                FlowRow(
                    Modifier.padding(horizontal = ListGutter),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) { s.links.forEach { l -> Chip(l.title) { Chrome.push(Route.Explore(l.path, l.title)) } } }
            }
        }
    }
    if (s.playlists.isNotEmpty()) item(key = "$key-playlists") { MediaShelf(s.playlists.map { it.shelfItem() }) }
    if (s.albums.isNotEmpty()) item(key = "$key-albums") { MediaShelf(s.albums.map { it.shelfItem() }) }
    if (s.artists.isNotEmpty()) item(key = "$key-artists") { MediaShelf(s.artists.map { it.shelfItem() }) }
    if (s.tracks.isNotEmpty()) trackItems(s.tracks, s.title, "$key-tracks", limit = 8)
}

@Composable
private fun LinkRow(link: PageLink) {
    Column(Modifier.fillMaxWidth().clickable(role = Role.Button) { Chrome.push(Route.Explore(link.path, link.title)) }.padding(horizontal = ListGutter)) {
        Row(Modifier.fillMaxWidth().height(44.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(link.title, style = IosTypography.body, modifier = Modifier.weight(1f))
            SdatIcon("chevron.right", null, size = 14.dp, tint = GlasmioTheme.colors.tertiaryContent)
        }
        HorizontalDivider(color = GlasmioTheme.colors.tertiaryContent.copy(alpha = 0.3f))
    }
}
