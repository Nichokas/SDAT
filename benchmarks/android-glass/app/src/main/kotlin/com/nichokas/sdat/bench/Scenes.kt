package com.nichokas.sdat.bench

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import io.github.neoruaa.glasmio.core.GlassAppearance
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.icons.SfSymbol
import io.github.neoruaa.glasmio.ui.GlasmioNavigationBar
import io.github.neoruaa.glasmio.ui.GlasmioTabItem

/** The SDAT screens the benchmark recreates; each animates itself the way a user drives it. */
enum class Page(val label: String) {
    DiscoverPan("Discover, steady pan"),
    DiscoverFling("Discover, fast fling, covers decoded on entry"),
    Collection("Collection list scroll, tab bar morphing"),
    Album("Album page scroll over the blurred cover"),
    Player("Player, pager swipes over the blurred cover"),
}

/** Live is SDAT today; Reduced is Glasmio's reduceTransparency (glass drawn as solid fills, nothing sampled); None drops Glasmio's chrome. */
enum class Glass(val label: String) { Live("live glass"), Reduced("reduced transparency"), None("no glass") }

data class Scene(val page: Page, val glass: Glass, val busyCores: Int = 0) {
    val label get() = "${page.label} · ${glass.label}" + if (busyCores > 0) " · $busyCores busy cores" else ""
}

/** Light to worst case. Album and Player have no glass-free form in SDAT, so they run only live and reduced. */
val scenes = buildList {
    for (page in listOf(Page.DiscoverPan, Page.DiscoverFling, Page.Collection)) for (glass in Glass.entries) add(Scene(page, glass))
    for (page in listOf(Page.Album, Page.Player)) for (glass in listOf(Glass.Live, Glass.Reduced)) add(Scene(page, glass))
    // Playback decoding, a download and the core's network work all run beside the UI on a real phone.
    add(Scene(Page.DiscoverFling, Glass.Live, busyCores = 2))
}

/** Counts every cover request so the report shows the images really loaded. */
class ImageCounter {
    @Volatile var loaded = 0
    @Volatile var failed = 0
}

@Composable
fun SceneView(scene: Scene, covers: List<Cover>, images: ImageCounter) {
    val appearance = GlassAppearance(reduceTransparency = scene.glass == Glass.Reduced)
    // Album and Player are dark in SDAT whatever the system says; the tabs follow it.
    val dark = scene.page == Page.Album || scene.page == Page.Player || isSystemInDarkTheme()
    GlasmioTheme(isDark = dark, appearance = appearance) {
        val seconds = rememberSeconds()
        when (scene.page) {
            Page.DiscoverPan -> Discover(scene.glass, covers, images, seconds, speedDp = 900f, cold = false)
            Page.DiscoverFling -> Discover(scene.glass, covers, images, seconds, speedDp = 3500f, cold = true)
            Page.Collection -> Collection(scene.glass, covers, images, seconds)
            Page.Album -> Album(covers, images, seconds)
            Page.Player -> Player(covers, images, seconds)
        }
    }
}

/** Seconds since the scene appeared, advanced every frame; read in layout or draw so it never recomposes. */
@Composable
private fun rememberSeconds(): State<Float> {
    val seconds = remember { mutableFloatStateOf(0f) }
    LaunchedEffect(Unit) {
        val start = withFrameNanos { it }
        while (true) withFrameNanos { seconds.floatValue = (it - start) / 1e9f }
    }
    return seconds
}

@Composable
internal fun cover(file: Cover, px: Int, cold: Boolean, images: ImageCounter): ImageRequest {
    val context = LocalContext.current
    return remember(file, px, cold) {
        ImageRequest.Builder(context).data("file://${file.file(px).absolutePath}").size(px)
            .memoryCachePolicy(if (cold) CachePolicy.DISABLED else CachePolicy.ENABLED)
            .listener(onSuccess = { _, _ -> images.loaded++ }, onError = { _, _ -> images.failed++ })
            .build()
    }
}

/** Glasmio's host when there is glass to sample for, a plain box otherwise. */
@Composable
internal fun Host(glass: Glass, page: @Composable BoxScope.() -> Unit, chrome: @Composable BoxScope.() -> Unit) {
    if (glass == Glass.None) {
        Box(Modifier.fillMaxSize()) { page() }
    } else {
        GlasmioBackdropHost(Modifier.fillMaxSize(), sampledContent = { page() }, overlayContent = { chrome() })
    }
}

internal enum class Tab { Discover, Search, Collection }

private val tabs = listOf(
    GlasmioTabItem(Tab.Discover, "Discover", SfSymbol.SafariFilled),
    GlasmioTabItem(Tab.Search, "Search", SfSymbol.Search),
    GlasmioTabItem(Tab.Collection, "My Collection", SfSymbol.MusicNoteSquareStackFilled),
)

/** SDAT's bottom chrome: the tab bar (collapsing by [compact]) beside the mini player's circle with its progress ring. */
@Composable
internal fun BoxScope.BottomChrome(selected: Tab, compact: State<Float>, seconds: State<Float>?) {
    BoxWithConstraints(Modifier.align(Alignment.BottomCenter).fillMaxWidth().navigationBarsPadding().padding(start = 24.dp, end = 24.dp, bottom = 12.dp)) {
        val barWidth = (maxWidth - 72.dp)
        GlasmioNavigationBar(
            items = tabs,
            selectedKey = selected,
            onSelected = {},
            onExpand = {},
            compactProgress = compact,
            compactSize = 54.dp,
            modifier = Modifier.align(Alignment.CenterStart).width(barWidth),
        )
        GlassSurface(Modifier.align(Alignment.CenterEnd).size(62.dp), shape = CircleShape) {
            ProgressRing(seconds)
        }
    }
}

@Composable
internal fun ProgressRing(seconds: State<Float>?) {
    Canvas(Modifier.fillMaxSize().padding(14.dp)) {
        val sweep = ((seconds?.value ?: 0f) / 20f % 1f) * 360f
        drawArc(Color.Gray.copy(alpha = 0.4f), 0f, 360f, false, style = Stroke(3.dp.toPx()))
        drawArc(Color.White, -90f, sweep, false, style = Stroke(3.dp.toPx(), cap = StrokeCap.Round))
    }
}

@Composable
internal fun BoxScope.HeaderButtons() {
    Row(Modifier.align(Alignment.TopEnd).statusBarsPadding().padding(end = 16.dp, top = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        repeat(2) { GlassSurface(Modifier.size(44.dp), shape = CircleShape) {} }
    }
}
