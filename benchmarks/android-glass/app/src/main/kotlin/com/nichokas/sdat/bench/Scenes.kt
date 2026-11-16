package com.nichokas.sdat.bench

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import io.github.neoruaa.glasmio.core.GlassAppearance
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassEmphasis
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.glass.LocalGlassBackdrop
import io.github.neoruaa.glasmio.glass.glassBackdropSource
import io.github.neoruaa.glasmio.glass.render.backdrops.rememberLayerBackdrop
import io.github.neoruaa.glasmio.glass.trackBackdropPosition
import io.github.neoruaa.glasmio.icons.SfSymbol
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedListPage
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedPage
import io.github.neoruaa.glasmio.scaffold.rememberGlasmioHeaderScrollEdge
import io.github.neoruaa.glasmio.ui.GlasmioButton
import io.github.neoruaa.glasmio.ui.GlasmioButtonSize
import io.github.neoruaa.glasmio.ui.GlasmioNavigationBar
import io.github.neoruaa.glasmio.ui.GlasmioSlider
import io.github.neoruaa.glasmio.ui.GlasmioTabItem
import java.io.File
import kotlin.math.abs
import kotlin.math.floor
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlinx.coroutines.delay

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
fun SceneView(scene: Scene, covers: List<File>, images: ImageCounter) {
    val appearance = GlassAppearance(reduceTransparency = scene.glass == Glass.Reduced)
    // Album and Player are dark in SDAT whatever the system says; the tabs follow it.
    val dark = scene.page == Page.Album || scene.page == Page.Player || isSystemInDarkTheme()
    GlasmioTheme(isDark = dark, appearance = appearance) {
        val seconds = rememberSeconds()
        when (scene.page) {
            Page.DiscoverPan -> Discover(scene.glass, covers, images, seconds, speedDp = 900f, cold = false)
            Page.DiscoverFling -> Discover(scene.glass, covers, images, seconds, speedDp = 3500f, cold = true)
            Page.Collection -> Collection(scene.glass, covers, images, seconds)
            Page.Album -> Album(covers, images)
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
private fun cover(file: File, px: Int, cold: Boolean, images: ImageCounter): ImageRequest {
    val context = LocalContext.current
    return remember(file, px, cold) {
        ImageRequest.Builder(context).data("file://${file.absolutePath}").size(px)
            .memoryCachePolicy(if (cold) CachePolicy.DISABLED else CachePolicy.ENABLED)
            .listener(onSuccess = { _, _ -> images.loaded++ }, onError = { _, _ -> images.failed++ })
            .build()
    }
}

/** Glasmio's host when there is glass to sample for, a plain box otherwise. */
@Composable
private fun Host(glass: Glass, page: @Composable BoxScope.() -> Unit, chrome: @Composable BoxScope.() -> Unit) {
    if (glass == Glass.None) {
        Box(Modifier.fillMaxSize()) { page() }
    } else {
        GlasmioBackdropHost(Modifier.fillMaxSize(), sampledContent = { page() }, overlayContent = { chrome() })
    }
}

private enum class Tab { Discover, Search, Collection }

private val tabs = listOf(
    GlasmioTabItem(Tab.Discover, "Discover", SfSymbol.SafariFilled),
    GlasmioTabItem(Tab.Search, "Search", SfSymbol.Search),
    GlasmioTabItem(Tab.Collection, "My Collection", SfSymbol.MusicNoteSquareStackFilled),
)

/** SDAT's bottom chrome: the tab bar (collapsing by [compact]) beside the mini player's circle with its progress ring. */
@Composable
private fun BoxScope.BottomChrome(selected: Tab, compact: State<Float>, seconds: State<Float>?) {
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
private fun ProgressRing(seconds: State<Float>?) {
    Canvas(Modifier.fillMaxSize().padding(14.dp)) {
        val sweep = ((seconds?.value ?: 0f) / 20f % 1f) * 360f
        drawArc(Color.Gray.copy(alpha = 0.4f), 0f, 360f, false, style = Stroke(3.dp.toPx()))
        drawArc(Color.White, -90f, sweep, false, style = Stroke(3.dp.toPx(), cap = StrokeCap.Round))
    }
}

@Composable
private fun BoxScope.HeaderButtons() {
    Row(Modifier.align(Alignment.TopEnd).statusBarsPadding().padding(end = 16.dp, top = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        repeat(2) { GlassSurface(Modifier.size(44.dp), shape = CircleShape) {} }
    }
}

// Discover: SDAT's CoverPlane and PlaneGeometry, with the covers from disk.

@Composable
private fun Discover(glass: Glass, covers: List<File>, images: ImageCounter, seconds: State<Float>, speedDp: Float, cold: Boolean) {
    val noCompact = remember { mutableFloatStateOf(0f) }
    Host(
        glass,
        page = {
            val background = GlasmioTheme.colors.groupedBackground
            Box(Modifier.fillMaxSize().background(background)) {
                Plane(covers, images, seconds, speedDp, cold)
                Box(
                    Modifier.fillMaxWidth().background(
                        Brush.verticalGradient(0f to background, 0.35f to background.copy(alpha = 0.92f), 0.7f to background.copy(alpha = 0.55f), 1f to background.copy(alpha = 0f)),
                    ).statusBarsPadding().height(170.dp),
                )
                Box(
                    Modifier.align(Alignment.BottomCenter).fillMaxWidth().background(
                        Brush.verticalGradient(0f to background.copy(alpha = 0f), 0.5f to background.copy(alpha = 0.5f), 1f to background.copy(alpha = 0.85f)),
                    ).navigationBarsPadding().height(150.dp),
                )
                Text("Discover", color = GlasmioTheme.colors.content, fontSize = 34.sp, fontWeight = FontWeight.Bold, modifier = Modifier.statusBarsPadding().padding(start = 20.dp, top = 4.dp))
            }
        },
        chrome = {
            HeaderButtons()
            BottomChrome(Tab.Discover, noCompact, seconds)
        },
    )
}

@Composable
private fun Plane(covers: List<File>, images: ImageCounter, seconds: State<Float>, speedDp: Float, cold: Boolean) {
    val density = LocalDensity.current.density
    val placeholder = GlasmioTheme.colors.content.copy(alpha = 0.07f)
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val plane = remember(covers.size) { PlaneGeometry(covers.size) }
        val margin = 75f
        val (w, h) = maxWidth.value to maxHeight.value
        // Diagonal, so new covers enter from two edges at once.
        val pan by remember(speedDp) { derivedStateOf { Offset(speedDp * seconds.value, 0.45f * speedDp * seconds.value) } }
        val cells by remember(plane, w, h) {
            derivedStateOf { plane.cells(pan.x - margin, pan.y - margin, pan.x + w + margin, pan.y + h + margin) }
        }
        for (cell in cells) {
            val frame = plane.frame(cell)
            key(cell) {
                AsyncImage(
                    model = cover(covers[plane.index(cell)], (frame.size * density).roundToInt(), cold, images),
                    contentDescription = null,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier
                        .offset { IntOffset(((frame.x - pan.x) * density).roundToInt(), ((frame.y - pan.y) * density).roundToInt()) }
                        .size(frame.size.dp)
                        .background(placeholder),
                )
            }
        }
    }
}

data class Cell(val c: Int, val r: Int)

data class Tile(val x: Float, val y: Float, val size: Float)

/** Copied from SDAT's ui/CoverPlane.kt. */
class PlaneGeometry(private val count: Int) {
    private val sizes = floatArrayOf(96f, 140f, 118f, 100f, 132f, 150f, 108f)
    private val gap = 12f
    private val period = sizes.sum() + gap * sizes.size
    private val rows = maxOf(12, kotlin.math.sqrt(count / 1.4).toInt())
    private val stride = maxOf(1, count / rows)

    fun rowSize(r: Int) = sizes[r.mod(sizes.size)]

    fun rowY(r: Int): Float {
        val k = floor(r.toDouble() / sizes.size).toInt()
        var y = k * period
        for (i in 0 until r - k * sizes.size) y += sizes[i] + gap
        return y
    }

    fun rowOffset(r: Int): Float {
        val h = ((r.toLong() * 2_654_435_761L).toULong() % 1000uL).toFloat() / 1000f
        return h * (rowSize(r) + gap)
    }

    fun frame(cell: Cell): Tile {
        val size = rowSize(cell.r)
        return Tile(cell.c * (size + gap) - rowOffset(cell.r), rowY(cell.r), size)
    }

    fun cells(minX: Float, minY: Float, maxX: Float, maxY: Float): List<Cell> {
        val out = mutableListOf<Cell>()
        var r = floor(minY / period).toInt() * sizes.size - 1
        while (rowY(r) <= maxY) {
            val size = rowSize(r)
            val step = size + gap
            val off = rowOffset(r)
            if (rowY(r) + size >= minY) {
                val c0 = floor((minX + off - size) / step).toInt()
                val c1 = floor((maxX + off) / step).toInt()
                for (c in c0..maxOf(c0, c1)) out += Cell(c, r)
            }
            r++
        }
        return out
    }

    fun index(cell: Cell) = (cell.c + cell.r.toLong() * stride).mod(count.toLong()).toInt()
}

// Collection: a long list of cover rows, scrolled up and down, with the tab bar collapsing as SDAT's does on scroll.

@Composable
private fun Collection(glass: Glass, covers: List<File>, images: ImageCounter, seconds: State<Float>) {
    val list = rememberLazyListState()
    AutoScroll(list, speedDp = 2200f)
    // Collapses and expands every 2 s with a 0.3 s morph, as a user scrolling down and back up triggers it.
    val compact = remember {
        derivedStateOf {
            val p = seconds.value % 4f
            when {
                p < 1.7f -> 0f
                p < 2f -> (p - 1.7f) / 0.3f
                p < 3.7f -> 1f
                else -> 1f - (p - 3.7f) / 0.3f
            }
        }
    }
    val rows: LazyListScope.() -> Unit = {
        items(400) { i -> CoverRow(covers[i % covers.size], "Track ${i + 1}", "Artist ${i % 37} · Album ${i % 53}", images) }
    }
    Host(
        glass,
        page = {
            if (glass == Glass.None) {
                LazyColumn(Modifier.fillMaxSize().background(GlasmioTheme.colors.groupedBackground).statusBarsPadding(), state = list, content = rows)
            } else {
                GlasmioPinnedListPage(title = "My Collection", listState = list, bottomPaddingValues = PaddingValues(bottom = 96.dp), content = rows)
            }
        },
        chrome = { BottomChrome(Tab.Collection, compact, null) },
    )
}

@Composable
private fun CoverRow(file: File, title: String, subtitle: String, images: ImageCounter) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        val px = (56 * LocalDensity.current.density).roundToInt()
        AsyncImage(cover(file, px, false, images), null, Modifier.size(56.dp).clip(RoundedCornerShape(8.dp)), contentScale = ContentScale.Crop)
        Column(Modifier.padding(start = 12.dp)) {
            Text(title, style = IosTypography.body, color = GlasmioTheme.colors.content)
            Text(subtitle, style = IosTypography.subheadline, color = GlasmioTheme.colors.secondaryContent)
        }
    }
}

/** Scrolls [list] at a fixed speed, turning at each end. */
@Composable
private fun AutoScroll(list: LazyListState, speedDp: Float) {
    val density = LocalDensity.current.density
    LaunchedEffect(list) {
        var down = true
        var last = withFrameNanos { it }
        while (true) {
            withFrameNanos { now ->
                val px = speedDp * density * (now - last) / 1e9f
                last = now
                val moved = list.dispatchRawDelta(if (down) px else -px)
                if (abs(moved) < px / 2) down = !down
            }
        }
    }
}

// Album: SDAT's DetailPage with its blurred cover backdrop, glass Play and Shuffle in the scrolled header, and the wide mini player.

@Composable
private fun Album(covers: List<File>, images: ImageCounter) {
    val list = rememberLazyListState()
    AutoScroll(list, speedDp = 1600f)
    val pastHeader by remember { derivedStateOf { list.firstVisibleItemIndex > 0 } }
    val backdrop = rememberLayerBackdrop()
    val art = covers.first()
    GlasmioBackdropHost(
        Modifier.fillMaxSize(),
        sampledContent = {
            GlasmioPinnedPage(
                title = "Album",
                collapseProgress = if (pastHeader) 1f else 0f,
                topBarBlurProgress = rememberGlasmioHeaderScrollEdge(list),
                backgroundColor = Color.Black,
                background = { CoverBackdrop(art, images, Modifier.matchParentSize().glassBackdropSource(backdrop).trackBackdropPosition(backdrop)) },
            ) { padding ->
                CompositionLocalProvider(LocalGlassBackdrop provides backdrop) {
                    LazyColumn(state = list, modifier = Modifier.fillMaxSize(), contentPadding = PaddingValues(top = padding.calculateTopPadding(), bottom = 120.dp)) {
                        item {
                            Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                val px = (260 * LocalDensity.current.density).roundToInt()
                                AsyncImage(cover(art, px, false, images), null, Modifier.size(260.dp).shadow(20.dp, RoundedCornerShape(12.dp)).clip(RoundedCornerShape(12.dp)), contentScale = ContentScale.Crop)
                                Text("Album title", style = IosTypography.title2, color = Color.White, modifier = Modifier.padding(top = 16.dp))
                                Row(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                                    GlasmioButton({}, Modifier.weight(1f), emphasis = GlassEmphasis.Prominent, size = GlasmioButtonSize.Large) { Text("Play", color = Color.White) }
                                    GlasmioButton({}, Modifier.weight(1f), size = GlasmioButtonSize.Large) { Text("Shuffle", color = Color.White) }
                                }
                            }
                        }
                        items(60) { i -> CoverRow(covers[(i + 1) % covers.size], "${i + 1}. Track", "Artist", images) }
                    }
                }
            }
        },
        overlayContent = {
            GlassSurface(Modifier.align(Alignment.BottomCenter).fillMaxWidth().navigationBarsPadding().padding(start = 24.dp, end = 24.dp, bottom = 12.dp).height(62.dp), shape = RoundedCornerShape(percent = 50)) {}
        },
    )
}

/** SDAT's DetailPage CoverBackdrop: the cover zoomed and blurred into colour fields. */
@Composable
private fun CoverBackdrop(file: File, images: ImageCounter, modifier: Modifier) {
    Box(modifier.background(Color.Black).clipToBounds()) {
        AsyncImage(
            cover(file, 160, false, images),
            null,
            contentScale = ContentScale.Crop,
            colorFilter = ColorFilter.colorMatrix(ColorMatrix().apply { setToSaturation(1.8f) }),
            modifier = Modifier.fillMaxSize().graphicsLayer { scaleX = 2.2f; scaleY = 2.2f }.blur(50.dp),
        )
        Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = 0.32f)))
        Box(Modifier.fillMaxSize().background(Brush.verticalGradient(0.5f to Color.Transparent, 1f to Color.Black.copy(alpha = 0.55f))))
    }
}

// Player: SDAT's PlayerSheet content (the sheet's own window frame is left out, so this window's metrics see it),
// swiping between Now Playing and the queue while the slider moves.

@Composable
private fun Player(covers: List<File>, images: ImageCounter, seconds: State<Float>) {
    val pager = rememberPagerState { 2 }
    LaunchedEffect(pager) {
        while (true) {
            delay(900)
            pager.animateScrollToPage(1 - pager.currentPage)
        }
    }
    val art = covers[1]
    GlasmioBackdropHost(
        Modifier.fillMaxSize(),
        sampledContent = {
            Box(Modifier.fillMaxSize().background(Color.Black)) {
                AsyncImage(cover(art, 320, false, images), null, contentScale = ContentScale.Crop, modifier = Modifier.fillMaxSize().blur(80.dp).graphicsLayer { alpha = 0.6f })
            }
        },
        overlayContent = {
            HorizontalPager(pager, Modifier.fillMaxSize()) { page ->
                if (page == 0) NowPlaying(art, images, seconds) else Queue(covers, images)
            }
        },
    )
}

@Composable
private fun NowPlaying(art: File, images: ImageCounter, seconds: State<Float>) {
    Column(Modifier.fillMaxSize().statusBarsPadding().padding(24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        val px = (320 * LocalDensity.current.density).roundToInt()
        AsyncImage(cover(art, px, false, images), null, Modifier.fillMaxWidth().aspectRatio(1f).shadow(28.dp, RoundedCornerShape(16.dp)).clip(RoundedCornerShape(16.dp)), contentScale = ContentScale.Crop)
        Text("Track title", style = IosTypography.title3, color = Color.White, modifier = Modifier.padding(top = 24.dp))
        Text("Artist", style = IosTypography.body, color = Color.White.copy(alpha = 0.6f))
        Spacer(Modifier.height(14.dp))
        val position by remember { derivedStateOf { (seconds.value * 4f) % 240f } }
        GlasmioSlider(value = position, onValueChange = {}, valueRange = 0f..240f)
        Row(Modifier.padding(top = 20.dp), horizontalArrangement = Arrangement.spacedBy(46.dp)) {
            repeat(3) { GlassSurface(Modifier.size(56.dp), shape = CircleShape) {} }
        }
        Spacer(Modifier.height(16.dp))
        // A pulse like the cover's play/pause spring, so the page redraws every frame.
        Box(Modifier.size(8.dp).graphicsLayer { val s = 1f + 0.3f * sin(seconds.value * 6f); scaleX = s; scaleY = s }.background(Color.White, CircleShape))
    }
}

@Composable
private fun Queue(covers: List<File>, images: ImageCounter) {
    LazyColumn(Modifier.fillMaxSize().statusBarsPadding(), contentPadding = PaddingValues(bottom = 24.dp)) {
        items(80) { i -> CoverRow(covers[(i + 5) % covers.size], "Up next ${i + 1}", "Artist", images) }
    }
}
