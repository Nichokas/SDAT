package com.nichokas.sdat.ui

import android.content.Intent
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.Repeat
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.badgeText
import com.nichokas.sdat.core.coverUrlLarge
import com.nichokas.sdat.core.displayTitle
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.core.glasmioTypography
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.ui.GlasmioBottomSheet
import io.github.neoruaa.glasmio.ui.GlasmioCascadingMenu
import io.github.neoruaa.glasmio.ui.GlasmioSlider
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track

private val capsule = RoundedCornerShape(percent = 50)
private val dimmed = Color.White.copy(alpha = 0.65f)
private val favouriteYellow = Color(0xFFFFD60A)
private val removeRed = Color(0xFFFF453A)

/** The full player, dark like iOS's; swipe left, or tap the queue hint, for the queue. */
@Composable
fun PlayerSheet(playback: Playback, onDismiss: () -> Unit) {
    GlasmioTheme(isDark = true, typography = glasmioTypography(inter)) {
        GlasmioBottomSheet(onDismissRequest = onDismiss, title = "Now Playing") {
            val track = playback.track ?: return@GlasmioBottomSheet
            val pager = rememberPagerState { 2 }
            val scope = rememberCoroutineScope()
            // The sheet is its own window: its glass samples the cover backdrop, not the screen under it.
            GlasmioBackdropHost(
                modifier = Modifier.fillMaxSize(),
                sampledContent = { CoverBackdrop(track.coverUrlLarge) },
                overlayContent = {
                    HorizontalPager(pager, Modifier.fillMaxSize()) { page ->
                        if (page == 0) {
                            NowPlayingPage(playback, track, showQueue = { scope.launch { pager.animateScrollToPage(1) } })
                        } else {
                            QueuePage(playback)
                        }
                    }
                },
            )
        }
    }
}

@Composable
private fun CoverBackdrop(url: String?) {
    Box(Modifier.fillMaxSize().background(Color.Black)) {
        if (url != null) {
            AsyncImage(
                model = CoverUrl.sized(url, 320),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize().blur(80.dp).alpha(0.6f),
            )
        }
    }
}

@Composable
private fun NowPlayingPage(playback: Playback, track: Track, showQueue: () -> Unit) {
    var scrub by remember { mutableStateOf<Double?>(null) }
    var forward by remember { mutableStateOf(true) }
    LaunchedEffect(track.id) { Favs.check(track.id) }
    val slide: () -> ContentTransform = {
        val sign = if (forward) 1 else -1
        (slideInHorizontally { sign * it } + fadeIn()) togetherWith (slideOutHorizontally { -sign * it } + fadeOut())
    }
    Column(
        Modifier.fillMaxSize().padding(horizontal = 24.dp).padding(bottom = 12.dp),
        verticalArrangement = Arrangement.spacedBy(18.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        TopRow(track)
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
            val side = minOf(maxWidth, maxHeight)
            AnimatedContent(track, transitionSpec = { slide() }, contentKey = { it.id }, label = "cover") { t ->
                val scale by animateFloatAsState(if (playback.playing) 1f else 0.88f, spring(dampingRatio = 0.6f, stiffness = 300f), label = "cover scale")
                Cover(
                    t.coverUrlLarge,
                    side,
                    Modifier.graphicsLayer { scaleX = scale; scaleY = scale }.shadow(28.dp, RoundedCornerShape(16.dp)),
                    radius = 16.dp,
                )
            }
        }
        AnimatedContent(track, Modifier.fillMaxWidth(), transitionSpec = { slide() }, contentKey = { it.id }, label = "title") { t ->
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(t.displayTitle, style = IosTypography.title3, fontWeight = FontWeight.Bold, color = Color.White, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(t.artistLine, style = IosTypography.body, color = dimmed, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            val end = maxOf(playback.duration, 1.0)
            val shown = scrub ?: playback.position
            GlasmioSlider(
                value = shown.coerceIn(0.0, end).toFloat(),
                onValueChange = { scrub = it.toDouble() },
                valueRange = 0f..end.toFloat(),
                onValueChangeFinished = {
                    scrub?.let(playback::seek)
                    scrub = null
                },
            )
            Row(Modifier.fillMaxWidth()) {
                val digits = IosTypography.caption1.copy(fontFeatureSettings = "tnum")
                Text(Fmt.time(shown), style = digits, color = Color.White.copy(alpha = 0.6f))
                Spacer(Modifier.weight(1f))
                Text("-" + Fmt.time(playback.duration - shown), style = digits, color = Color.White.copy(alpha = 0.6f))
            }
        }
        QualityBadge(playback)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            IconTap("shuffle", "Shuffle", 22.dp, tint = if (playback.shuffle) GlasmioTheme.colors.accent else Color.White.copy(alpha = 0.8f)) {
                playback.setShuffle(!playback.shuffle)
            }
            IconTap("backward.fill", "Previous", 30.dp) { forward = false; playback.previous() }
            Box(
                Modifier.size(64.dp).clickable(onClickLabel = if (playback.playing) "Pause" else "Play", role = Role.Button) { playback.toggle() },
                contentAlignment = Alignment.Center,
            ) {
                if (playback.loading) {
                    CircularProgressIndicator(color = Color.White)
                } else {
                    SdatIcon(if (playback.playing) "pause.fill" else "play.fill", if (playback.playing) "Pause" else "Play", size = 46.dp, tint = Color.White)
                }
            }
            IconTap("forward.fill", "Next", 30.dp) { forward = true; playback.next() }
            val repeat = playback.repeat
            IconTap(if (repeat == Repeat.One) "repeat.1" else "repeat", "Repeat", 22.dp, tint = if (repeat == Repeat.Off) Color.White.copy(alpha = 0.8f) else GlasmioTheme.colors.accent) {
                playback.cycleRepeat()
            }
        }
        Row(Modifier.fillMaxWidth()) {
            Spacer(Modifier.weight(1f))
            GlassSurface(shape = capsule) {
                Row(
                    Modifier.clickable(onClickLabel = "Show queue", role = Role.Button, onClick = showQueue).padding(horizontal = 14.dp, vertical = 9.dp),
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    SdatIcon("list.bullet", "Queue", size = 20.dp, tint = Color.White.copy(alpha = 0.85f))
                    SdatIcon("chevron.right", null, size = 14.dp, tint = Color.White.copy(alpha = 0.85f))
                }
            }
        }
    }
}

@Composable
private fun TopRow(track: Track) {
    val context = LocalContext.current
    val fav = Favs.known(track.id) ?: false
    var menuBounds by remember { mutableStateOf<Rect?>(null) }
    var menuOpen by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().padding(top = 14.dp), verticalAlignment = Alignment.CenterVertically) {
        GlassSurface(shape = capsule) {
            Row {
                IconTap("square.and.arrow.up", "Share", 20.dp) {
                    val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, "https://tidal.com/track/${track.id}")
                    context.startActivity(Intent.createChooser(send, null))
                }
                IconTap(if (fav) "star.fill" else "star", if (fav) "Remove from Favourites" else "Add to Favourites", 20.dp, tint = if (fav) favouriteYellow else Color.White) {
                    Favs.set(track.id, !fav)
                }
            }
        }
        Spacer(Modifier.weight(1f))
        GlassSurface(Modifier.onGloballyPositioned { menuBounds = it.boundsInRoot() }, shape = CircleShape) {
            IconTap("ellipsis", "More", 20.dp) { menuOpen = true }
        }
    }
    if (menuOpen) {
        GlasmioCascadingMenu(
            anchorBounds = menuBounds,
            items = trackMenuItems(track),
            title = track.displayTitle,
            expandedDescription = "Expanded",
            collapsedDescription = "Collapsed",
            onDismiss = { menuOpen = false },
        )
    }
}

/** What the badge shows; iOS's quality menu waits for the quality reload. */
@Composable
private fun QualityBadge(playback: Playback) {
    val q = playback.badgeQuality
    GlassSurface(shape = capsule) {
        Row(
            Modifier.heightIn(min = 44.dp).padding(horizontal = 18.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            QualityLogo(q, 22.dp)
            Text(q.badgeText, style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, color = Color.White.copy(alpha = 0.9f), maxLines = 1)
        }
    }
}

@Composable
private fun QueuePage(playback: Playback) {
    val colors = GlasmioTheme.colors
    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = 20.dp, end = 12.dp, bottom = 24.dp)) {
        item(key = "title") {
            Box(Modifier.fillMaxWidth().heightIn(min = 44.dp), contentAlignment = Alignment.Center) {
                Text("Queue", style = IosTypography.headline, color = colors.content)
                IconTap("shuffle", "Shuffle", 20.dp, Modifier.align(Alignment.CenterEnd), tint = if (playback.shuffle) colors.accent else colors.content) {
                    playback.setShuffle(!playback.shuffle)
                }
            }
        }
        playback.track?.let { t ->
            header("Now Playing")
            item(key = "now") { TrackRow(t, Modifier.padding(vertical = 6.dp).padding(end = 8.dp)) }
        }
        if (playback.userQueue.isNotEmpty()) {
            header("Next in Queue")
            queueRows("user", playback.userQueue, playback::jumpToUser, playback::removeUser)
        }
        header(playback.source?.let { "Playing from $it" } ?: "Up Next")
        queueRows("base", playback.upNext, playback::jumpToUpNext, playback::removeUpNext)
    }
}

private fun LazyListScope.header(title: String) {
    item(key = "header $title") {
        Text(title, style = IosTypography.headline, color = GlasmioTheme.colors.content, modifier = Modifier.padding(top = 20.dp, bottom = 4.dp))
    }
}

private fun LazyListScope.queueRows(section: String, tracks: List<Track>, onTap: (Int) -> Unit, onRemove: (Int) -> Unit) {
    val keys = occurrenceKeys(tracks.map { it.id })
    itemsIndexed(tracks, key = { i, _ -> "$section ${keys[i]}" }) { i, t ->
        Row(Modifier.animateItem(), verticalAlignment = Alignment.CenterVertically) {
            TrackMenuRow(t, onClick = { onTap(i) }, Modifier.weight(1f)) { TrackRow(t, Modifier.padding(vertical = 6.dp)) }
            IconTap("minus.circle", "Remove ${t.displayTitle}", 20.dp, tint = removeRed) { onRemove(i) }
        }
    }
}

/** Row keys that survive a removal above them: a track's id and which occurrence of it this is. */
internal fun occurrenceKeys(ids: List<String>): List<String> {
    val seen = HashMap<String, Int>()
    return ids.map { id ->
        val n = seen[id] ?: 0
        seen[id] = n + 1
        "$id#$n"
    }
}

@Composable
private fun IconTap(
    symbol: String,
    label: String,
    size: Dp,
    modifier: Modifier = Modifier,
    tint: Color = Color.White,
    onClick: () -> Unit,
) {
    Box(
        modifier.size(maxOf(size + 20.dp, 44.dp)).clickable(onClickLabel = label, role = Role.Button, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        SdatIcon(symbol, label, size = size, tint = tint)
    }
}
