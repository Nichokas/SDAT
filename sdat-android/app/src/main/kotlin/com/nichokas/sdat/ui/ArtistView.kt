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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Fav
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.ArtistCache
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.PageLoad
import com.nichokas.sdat.core.friendlyMessage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassEmphasis
import io.github.neoruaa.glasmio.ui.GlasmioButton
import io.github.neoruaa.glasmio.ui.GlasmioButtonSize
import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.ArtistOverview
import uniffi.tonearm_ffi.FavouriteKind
import uniffi.tonearm_ffi.artistOverview

// A follow tapped just before going back still goes through.

/** Opens at once with the header from the cached overview or the tapped row, then loads the rest. */
@Composable
fun ArtistView(id: String) {
    var state by remember(id) { mutableStateOf<PageLoad<ArtistOverview>>(ArtistCache.overviews[id]?.let { PageLoad.Ready(it) } ?: PageLoad.Loading) }
    var attempt by remember { mutableIntStateOf(0) }
    LaunchedEffect(id, attempt) {
        try {
            val o = artistOverview(id)
            ArtistCache.overviews[id] = o
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
    DetailScaffold(title = name, backdropUrl = picture) {
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
                    Text(name, style = IosTypography.title1, fontWeight = FontWeight.Bold, textAlign = TextAlign.Center)
                }
                FollowButton(id)
            }
        }
        when (val s = state) {
            is PageLoad.Ready -> sections(s.value)
            is PageLoad.Failed -> item(key = "error") {
                Column(Modifier.fillMaxWidth().padding(horizontal = ListGutter, vertical = 16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    ErrorBanner(s.message)
                    GlasmioButton({ state = PageLoad.Loading; attempt++ }, Modifier.padding(top = 12.dp), size = GlasmioButtonSize.Medium) { Text("Retry") }
                }
            }
            PageLoad.Loading -> items(6) { Box(Modifier.padding(horizontal = ListGutter)) { RowSkeleton() } }
        }
    }
}

private fun LazyListScope.sections(o: ArtistOverview) {
    if (o.topTracks.isNotEmpty()) {
        item(key = "top-title") { SectionTitle("Top Tracks") }
        item(key = "top-buttons") {
            val player = LocalTrackPlayer.current
            PlayShuffleButtons(
                play = { player.playInOrder(o.topTracks, o.artist.name) },
                shuffle = { player.playShuffled(o.topTracks, o.artist.name) },
                compact = true,
                modifier = Modifier.padding(start = ListGutter, end = ListGutter, bottom = 8.dp),
            )
        }
        itemsIndexed(o.topTracks, key = { i, t -> "top:${t.id}#$i" }) { i, t ->
            val player = LocalTrackPlayer.current
            Column(Modifier.padding(horizontal = ListGutter)) {
                TrackMenuRow(t, onClick = { player.play(o.topTracks, i, o.artist.name) }) {
                    TrackRow(t, Modifier.heightIn(min = 72.dp), coverSize = 56.dp)
                }
                if (i < o.topTracks.lastIndex) {
                    Box(Modifier.padding(start = 68.dp).fillMaxWidth().height(0.5.dp).background(Color.White.copy(alpha = 0.15f)))
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
private fun SectionTitle(text: String) = SectionHeader(text, Modifier.padding(top = 24.dp, bottom = 10.dp), style = IosTypography.title2)

/** Follow / Following: optimistic, saved as an artist favourite, reverted on error. */
@Composable
fun FollowButton(artistId: String) {
    val fav = Fav(FavouriteKind.ARTIST, artistId)
    LaunchedEffect(fav) { Favs.check(fav) }
    val following = Favs.known(fav)
    val on = following == true
    GlasmioButton(
        onClick = { Favs.set(fav, !on) },
        enabled = following != null,
        emphasis = if (on) GlassEmphasis.Prominent else GlassEmphasis.Regular,
        size = GlasmioButtonSize.Medium,
    ) {
        SdatIcon(if (on) "checkmark" else "plus", null, size = 16.dp)
        Text(
            if (on) "Following" else "Follow",
            style = IosTypography.subheadline,
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
            style = IosTypography.subheadline,
            color = GlasmioTheme.colors.secondaryContent,
            maxLines = if (expanded) Int.MAX_VALUE else 6,
            overflow = TextOverflow.Ellipsis,
            onTextLayout = { if (!expanded) overflows = it.hasVisualOverflow },
        )
        if (overflows || expanded) {
            Text(
                if (expanded) "Less" else "More",
                style = IosTypography.subheadline,
                fontWeight = FontWeight.SemiBold,
                color = GlasmioTheme.colors.accent,
                modifier = Modifier.clickable(role = Role.Button) { expanded = !expanded },
            )
        }
    }
}
