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

/** Blur is SDAT's frosted glass; Tint draws the same glass as its tint alone, nothing blurred; None drops the chrome. */
enum class Glass(val label: String) { Blur("frosted glass"), Tint("tint only"), None("no glass") }

/**
 * The SDAT screens the benchmark recreates, each with the glass modes it runs in; each animates itself the way a user
 * drives it. Album and Player have no glass-free form in SDAT, so they run only frosted and tinted.
 */
enum class Page(val label: String, val glass: List<Glass>) {
    DiscoverPan("Discover, steady pan", Glass.entries),
    DiscoverFling("Discover, fast fling, covers decoded on entry", Glass.entries),
    Collection("Collection list scroll, tab bar morphing", Glass.entries),
    Album("Album page scroll over the blurred cover", listOf(Glass.Blur, Glass.Tint)),
    Player("Player, pager swipes over the blurred cover", listOf(Glass.Blur, Glass.Tint)),
}

data class Scene(val page: Page, val glass: Glass, val busyCores: Int = 0) {
    init {
        require(glass in page.glass) { "${page.name} has no ${glass.name} form" }
    }

    val label get() = "${page.label} · ${glass.label}" + if (busyCores > 0) " · $busyCores busy cores" else ""
}

/** Light to worst case: every page in each of its glass modes, then the fling with busy cores. */
val scenes = Page.entries.flatMap { page -> page.glass.map { Scene(page, it) } } +
    // Playback decoding, a download and the core's network work all run beside the UI on a real phone.
    Scene(Page.DiscoverFling, Glass.Blur, busyCores = 2)

/** Counts every cover request so the report shows the images really loaded. */
class ImageCounter {
    @Volatile var loaded = 0
    @Volatile var failed = 0
}

@Composable
fun SceneView(scene: Scene, covers: List<Cover>, images: ImageCounter) {
    // Album and Player are dark in SDAT whatever the system says; the tabs follow it.
    val dark = scene.page == Page.Album || scene.page == Page.Player || isSystemInDarkTheme()
    Theme(dark) {
        val seconds = rememberSeconds()
        when (scene.page) {
            Page.DiscoverPan -> Discover(scene.glass, covers, images, seconds, speedDp = 900f, cold = false)
            Page.DiscoverFling -> Discover(scene.glass, covers, images, seconds, speedDp = 3500f, cold = true)
            Page.Collection -> Collection(scene.glass, covers, images, seconds)
            Page.Album -> Album(scene.glass, covers, images, seconds)
            Page.Player -> Player(scene.glass, covers, images, seconds)
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

internal enum class Tab { Discover, Search, Collection }

private val tabs = listOf("Discover", "Search", "My Collection")

/** SDAT's bottom chrome: the tab bar (collapsing by [compact]) beside the mini player's circle with its progress ring. */
@Composable
internal fun BoxScope.BottomChrome(selected: Tab, compact: State<Float>, seconds: State<Float>?) {
    BoxWithConstraints(Modifier.align(Alignment.BottomCenter).fillMaxWidth().navigationBarsPadding().padding(start = 24.dp, end = 24.dp, bottom = 12.dp)) {
        val barWidth = (maxWidth - 72.dp)
        TabBar(tabs, selected.ordinal, compact, compactSize = 54.dp, modifier = Modifier.align(Alignment.CenterStart).width(barWidth))
        Box(Modifier.align(Alignment.CenterEnd).size(62.dp).glass(CircleShape)) {
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
        repeat(2) { Box(Modifier.size(44.dp).glass(CircleShape)) }
    }
}
