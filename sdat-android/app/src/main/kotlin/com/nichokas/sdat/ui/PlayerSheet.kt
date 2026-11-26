package com.nichokas.sdat.ui

import android.animation.ValueAnimator
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.media.MediaRouter2
import android.os.Build
import android.view.HapticFeedbackConstants
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.Image
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitHorizontalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.horizontalDrag
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListLayoutInfo
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import coil3.BitmapImage
import coil3.SingletonImageLoader
import coil3.request.SuccessResult
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.Origin
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.QueueSlice
import com.nichokas.sdat.core.Repeat
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.badgeText
import com.nichokas.sdat.core.coverUrlLarge
import com.nichokas.sdat.core.displayTitle
import com.nichokas.sdat.ui.kit.BottomSheet
import com.nichokas.sdat.ui.kit.GlassHost
import com.nichokas.sdat.ui.kit.MenuAction
import com.nichokas.sdat.ui.kit.PopupMenu
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import com.nichokas.sdat.ui.kit.Slider
import com.nichokas.sdat.ui.kit.menuAnchor
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.roundToInt
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track

private val dimmed = Color.White.copy(alpha = 0.65f)
private val quiet = Color.White.copy(alpha = 0.85f)
private val inactive = Color.White.copy(alpha = 0.45f)
private val queueRowBackground = Color.White.copy(alpha = 0.06f)
private val removeRed = Color(0xFFFF453A)
private val queueHandleWidth = 44.dp

/** The full player, dark like iOS's; swipe left, or tap the queue button, for the queue. */
@Composable
fun PlayerSheet(playback: Playback, onDismiss: () -> Unit) {
    val lightBackdrop = !SdatTheme.colors.isDark
    SdatTheme(isDark = true) {
        BottomSheet(onDismissRequest = onDismiss, title = "Now Playing") {
            val track = playback.track ?: return@BottomSheet
            val pager = rememberPagerState { 2 }
            val scope = rememberCoroutineScope()
            // The sheet is its own window, so its glass blurs the cover behind it rather than the app.
            GlassHost(
                modifier = Modifier.fillMaxSize(),
                content = { CoverBackdrop(track.coverUrlLarge, playback.playing, track.bpm, lightBackdrop) },
                overlay = {
                    HorizontalPager(pager, Modifier.fillMaxSize().navigationBarsPadding()) { page ->
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
private fun CoverBackdrop(url: String?, playing: Boolean, bpm: Double?, light: Boolean) {
    val context = LocalContext.current
    val animate = ValueAnimator.areAnimatorsEnabled()
    var art by remember { mutableStateOf<Bitmap?>(null) }
    val beats = remember { mutableDoubleStateOf(0.0) }
    LaunchedEffect(context, url) {
        val result = url?.let { SingletonImageLoader.get(context).execute(backdropRequest(context, it, radius = 8)) }
        art = ((result as? SuccessResult)?.image as? BitmapImage)?.bitmap
    }
    LaunchedEffect(playing, bpm, animate) {
        if (!playing || !animate) return@LaunchedEffect
        val beatsPerSecond = (bpm?.takeIf { it.isFinite() } ?: 100.0).coerceIn(60.0, 160.0) / 60
        var last = withFrameNanos { it }
        while (true) {
            withFrameNanos { now ->
                beats.doubleValue += (now - last).coerceIn(0L, 100_000_000L) * beatsPerSecond / 1e9
                last = now
            }
        }
    }
    Box(Modifier.fillMaxSize().background(if (light) Color(0xFF16161B) else Color.Black).clipToBounds()) {
        Crossfade(
            art, Modifier.fillMaxSize(), animationSpec = tween(if (animate) 800 else 0), label = "Backdrop cover",
        ) { bitmap ->
            if (bitmap != null) {
                Image(
                    bitmap = remember(bitmap) { bitmap.asImageBitmap() },
                    contentDescription = null,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize().graphicsLayer {
                        val phase = beats.doubleValue
                        val breath = (0.5 - 0.5 * cos(phase * Math.PI / 2)).toFloat()
                        val smallerSide = minOf(size.width, size.height).coerceAtLeast(1f)
                        val fit = hypot(size.width, size.height) / smallerSide
                        rotationZ = (phase * 4.5 % 360).toFloat()
                        scaleX = fit.coerceAtLeast(1f) * (1f + 0.06f * breath)
                        scaleY = scaleX
                        alpha = if (light) 0.46f else 0.38f
                    },
                )
            }
        }
    }
}

@Composable
private fun NowPlayingPage(playback: Playback, track: Track, showQueue: () -> Unit) {
    var forward by remember { mutableStateOf(true) }
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val previousWiggle = remember { Animatable(0f) }
    val nextWiggle = remember { Animatable(0f) }
    LaunchedEffect(track.id) {
        forward = true
        Favs.check(track.id)
    }
    val slide: () -> ContentTransform = {
        val sign = if (forward) 1 else -1
        (slideInHorizontally { sign * it } + fadeIn()) togetherWith (slideOutHorizontally { -sign * it } + fadeOut())
    }
    // A short window (landscape, split screen) can't fit the controls below a weighted cover, so it scrolls instead.
    BoxWithConstraints(Modifier.fillMaxSize()) {
    val compact = maxHeight < 560.dp
    Column(
        Modifier.fillMaxSize().then(if (compact) Modifier.verticalScroll(rememberScrollState()) else Modifier)
            .padding(horizontal = 24.dp).padding(top = 24.dp, bottom = 12.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        BoxWithConstraints(if (compact) Modifier.fillMaxWidth().height(200.dp) else Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
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
        TitleRow(playback, track, slide, Modifier.padding(top = 24.dp))
        Scrubber(playback, Modifier.padding(top = 14.dp))
        QualityBadge(playback, Modifier.padding(top = 2.dp))
        Row(Modifier.padding(top = 20.dp), horizontalArrangement = Arrangement.spacedBy(46.dp), verticalAlignment = Alignment.CenterVertically) {
            IconTap("backward.fill", "Previous", 30.dp, Modifier.graphicsLayer {
                alpha = if (Live.isGuest) 0.4f else 1f
                rotationZ = previousWiggle.value
            }) {
                if (!Live.guestRefuses()) {
                    view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                    scope.launch {
                        previousWiggle.snapTo(-12f)
                        previousWiggle.animateTo(0f, spring(dampingRatio = 0.3f, stiffness = 600f))
                    }
                    val before = playback.track?.id
                    playback.previous()
                    // A restart (seek to 0) keeps the track, so the next change must still slide forward.
                    if (playback.track?.id != before) forward = false
                }
            }
            Box(
                Modifier.size(64.dp).alpha(if (Live.playbackLocked) 0.4f else 1f)
                    .clickable(onClickLabel = if (playback.playing) "Pause" else "Play", role = Role.Button) { Live.userToggle() },
                contentAlignment = Alignment.Center,
            ) {
                if (playback.loading) {
                    CircularProgressIndicator(color = Color.White)
                } else {
                    Crossfade(playback.playing, label = "play control") { playing ->
                        SdatIcon(if (playing) "pause.fill" else "play.fill", null, size = 46.dp, tint = Color.White)
                    }
                }
            }
            IconTap("forward.fill", "Next", 30.dp, Modifier.graphicsLayer {
                alpha = if (Live.playbackLocked) 0.4f else 1f
                rotationZ = nextWiggle.value
            }) {
                if (!Live.playbackLocked) {
                    view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                    scope.launch {
                        nextWiggle.snapTo(12f)
                        nextWiggle.animateTo(0f, spring(dampingRatio = 0.3f, stiffness = 600f))
                    }
                }
                forward = true
                Live.userNext()
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = 16.dp)) {
            val context = LocalContext.current
            IconTap("airplayaudio", "Audio output", 22.dp, tint = quiet) {
                if (!showOutputSwitcher(context)) Chrome.flash("Couldn't open the audio output picker")
            }
            Spacer(Modifier.weight(1f))
            IconTap("list.bullet", "Queue", 22.dp, tint = quiet, onClick = showQueue)
        }
    }
    }
}

internal fun interpolatedPosition(
    anchorPosition: Double,
    elapsedSecs: Double,
    duration: Double,
    advancing: Boolean,
): Double {
    if (!advancing) return anchorPosition
    val dt = elapsedSecs.coerceIn(0.0, 0.5)
    return (anchorPosition + dt).coerceAtMost(maxOf(duration, anchorPosition))
}

/** Isolates vsync scrubber interpolation so NowPlayingPage doesn't recompose while playing. */
@Composable
private fun Scrubber(playback: Playback, modifier: Modifier = Modifier) {
    // A new queue entry, even of the same track, drops the slider and any drag on it, so a release can't seek it to the old one's place.
    key(playback.entry) {
        var scrub by remember { mutableStateOf<Double?>(null) }
        var dragEnd by remember { mutableStateOf<Double?>(null) }
        val locked = Live.playbackLocked
        // Slider keys pointerInput on enabled and valueRange: clear scrub if a lock or track change cancels the drag, and freeze the range while dragging.
        LaunchedEffect(locked) {
            if (locked) {
                scrub = null
                dragEnd = null
            }
        }
        LaunchedEffect(playback.track?.id) {
            scrub = null
            dragEnd = null
        }
        val advancing = playback.playing && !playback.loading && scrub == null
        var shown by remember { mutableDoubleStateOf(playback.position) }
        LaunchedEffect(playback.position, playback.entry, advancing) {
            val base = playback.position
            shown = base
            if (!advancing) return@LaunchedEffect
            val startNs = withFrameNanos { it }
            while (true) {
                withFrameNanos { now ->
                    shown = interpolatedPosition(base, (now - startNs) / 1e9, playback.duration, advancing = true)
                }
            }
        }
        val end = dragEnd ?: maxOf(playback.duration, 1.0)
        val displayed = if (locked) shown else scrub ?: shown
        Column(modifier, verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Slider(
                value = displayed.coerceIn(0.0, end).toFloat(),
                onValueChange = {
                    if (!Live.playbackLocked) {
                        if (dragEnd == null) dragEnd = end
                        scrub = it.toDouble()
                    }
                },
                valueRange = 0f..end.toFloat(),
                enabled = !locked,
                onValueChangeFinished = {
                    scrub?.let {
                        if (!Live.userSeek(it)) {
                            playback.seek(it)
                            shown = playback.position
                        }
                    }
                    scrub = null
                    dragEnd = null
                },
            )
            Row(Modifier.fillMaxWidth()) {
                val digits = SdatType.caption1.copy(fontFeatureSettings = "tnum")
                Text(Fmt.time(displayed), style = digits, color = Color.White.copy(alpha = 0.6f))
                Spacer(Modifier.weight(1f))
                Text("-" + Fmt.time(maxOf(0.0, playback.duration - displayed)), style = digits, color = Color.White.copy(alpha = 0.6f))
            }
        }
    }
}

@Composable
private fun TitleRow(playback: Playback, track: Track, slide: () -> ContentTransform, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val fav = Favs.known(track.id) ?: false
    var menuBounds by remember { mutableStateOf<Rect?>(null) }
    var menuOpen by remember { mutableStateOf(false) }
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        AnimatedContent(track, Modifier.weight(1f), transitionSpec = { slide() }, contentKey = { it.id }, label = "title") { t ->
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(t.displayTitle, style = SdatType.title3, fontWeight = FontWeight.Bold, color = Color.White, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (playback.origin == Origin.DOWNLOADED && playback.track?.id == t.id) SdatIcon("arrow.down.circle.fill", "Playing downloaded copy", size = 15.dp, tint = dimmed)
                    Text(t.artistLine, style = SdatType.body, color = dimmed, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
        IconTap(if (fav) "star.fill" else "star", if (fav) "Unfavourite" else "Favourite", 22.dp) { Favs.set(track.id, !fav) }
        IconTap("ellipsis", "More", 22.dp, Modifier.onGloballyPositioned { menuBounds = it.boundsInRoot() }.menuAnchor(menuOpen)) { menuOpen = true }
    }
    if (menuOpen) {
        val share = MenuAction("Share", "square.and.arrow.up", onClick = {
            val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, "https://tidal.com/track/${track.id}")
            context.startActivity(Intent.createChooser(send, null))
        })
        val items = trackMenuItems(track)
        PopupMenu(
            anchorBounds = menuBounds,
            actions = listOf(share) + items.mapIndexed { i, item -> if (i == 0) item.copy(separatorBefore = true) else item } + sendToMenuItems(),
            title = track.displayTitle,
            onDismiss = { menuOpen = false },
        )
    }
}

/** Opens the system's picker for where this app's audio plays, Android's counterpart of AirPlay. */
private fun showOutputSwitcher(context: Context): Boolean {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) return MediaRouter2.getInstance(context).showSystemOutputSwitcher()
    // Android 13 has no API for it; System UI listens for this broadcast, as androidx.mediarouter sends it.
    val intent = Intent("com.android.systemui.action.LAUNCH_MEDIA_OUTPUT_DIALOG").setPackage("com.android.systemui").putExtra("package_name", context.packageName)
    val receivers = context.packageManager.queryBroadcastReceivers(intent, PackageManager.ResolveInfoFlags.of(0))
    if (receivers.none { (it.activityInfo?.applicationInfo?.flags ?: 0) and ApplicationInfo.FLAG_SYSTEM != 0 }) return false
    context.sendBroadcast(intent)
    return true
}

/** The quality playing now; tapping it picks the streaming quality, which applies at once. */
@Composable
private fun QualityBadge(playback: Playback, modifier: Modifier = Modifier) {
    val q = playback.badgeQuality
    QualityMenu(playback.streamingQuality, playback::setStreamingQuality) { toggle ->
        Row(
            modifier.clickable(onClickLabel = "Maximum quality", role = Role.Button, onClick = toggle).heightIn(min = 44.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            QualityLogo(q, 16.dp)
            Text(q.badgeText, style = SdatType.caption1, fontWeight = FontWeight.SemiBold, color = Color.White.copy(alpha = 0.7f), maxLines = 1)
            SdatIcon("chevron.up.chevron.down", null, size = 11.dp, tint = Color.White.copy(alpha = 0.5f))
        }
    }
}

@Composable
private fun QueuePage(playback: Playback) {
    val guest = Live.isGuest
    val user = playback.userQueueSlice.let { slice -> remember(slice) { EditableQueueRows("user", slice) } }
    val upcoming = playback.upcomingQueueSlice.let { slice -> remember(slice) { EditableQueueRows("base", slice) } }
    val sections = remember(guest, user, upcoming) { if (guest) emptyList() else listOf(user, upcoming) }
    val listState = rememberLazyListState()
    val listPadding = PaddingValues(start = 20.dp, end = 12.dp, bottom = 24.dp)
    val direction = LocalLayoutDirection.current
    val edge = with(LocalDensity.current) { 56.dp.toPx() }
    var drag by remember(sections) { mutableStateOf<QueueDrag?>(null) }
    LaunchedEffect(drag) {
        val active = drag ?: return@LaunchedEffect
        var previousFrame = withFrameNanos { it }
        while (true) {
            val frame = withFrameNanos { it }
            val seconds = (frame - previousFrame).coerceAtMost(50_000_000L) / 1_000_000_000f
            previousFrame = frame
            val speed = active.scrollSpeed(listState.layoutInfo, edge)
            if (speed != 0f) listState.scrollBy(speed * seconds)
        }
    }
    Column(Modifier.fillMaxSize()) {
        Box(Modifier.padding(start = 20.dp, end = 12.dp)) { QueueHeader(playback) }
        Box(Modifier.weight(1f).fillMaxWidth().clipToBounds()) {
            LazyColumn(
                Modifier.fillMaxSize().queueReorder(listState, sections, listPadding, direction, playback) { drag = it },
                state = listState,
                contentPadding = listPadding,
            ) {
                playback.track?.let { t ->
                    header("Now Playing")
                    item(key = "now") {
                        TrackRow(t, Modifier.fillMaxWidth().background(queueRowBackground).padding(horizontal = 8.dp, vertical = 6.dp))
                    }
                }
                if (guest) {
                    // Guests follow the host's queue; the header opens the session sheet.
                    item(key = "live header") {
                        Row(
                            Modifier.padding(top = 20.dp, bottom = 4.dp).clickable(role = Role.Button) { Live.showSheet = true },
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            SdatIcon("dot.radiowaves.left.and.right", null, size = 18.dp, tint = Color.White)
                            Text("Playing from Listen Together", style = SdatType.headline, color = Color.White)
                        }
                    }

                    items(Live.upNext, key = { "live ${it.id}" }) { row ->
                        TrackMenuRow(row.track, onClick = {}, Modifier.animateItem().background(queueRowBackground)) {
                            TrackRow(row.track, Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 6.dp))
                        }
                    }
                    return@LazyColumn
                }
                if (user.slice.tracks.isNotEmpty()) {
                    header("Next in Queue")
                    queueRows(user, playback::jumpToUser, playback) { drag }
                }
                header(playback.source?.let { "Playing from $it" } ?: "Up Next")
                queueRows(upcoming, playback::jumpToUpNext, playback) { drag }
            }
            drag?.let { active ->
                Row(
                    Modifier.fillMaxWidth().padding(
                        start = listPadding.calculateStartPadding(direction),
                        end = listPadding.calculateEndPadding(direction),
                    )
                        .graphicsLayer {
                            translationY = active.center - active.height / 2f - listState.layoutInfo.viewportStartOffset
                        }
                        .background(Color(0xFF292929)).clearAndSetSemantics {},
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    TrackRow(active.rows.slice.tracks[active.from], Modifier.weight(1f).padding(vertical = 6.dp))
                    Spacer(Modifier.size(queueHandleWidth))
                    QueueDragHandle()
                }
            }
        }
    }
}

@Composable
private fun QueueHeader(playback: Playback) {
    val colors = SdatTheme.colors
    Box(Modifier.fillMaxWidth().heightIn(min = 44.dp), contentAlignment = Alignment.Center) {
        IconTap("dot.radiowaves.left.and.right", "Listen Together", 20.dp, Modifier.align(Alignment.CenterStart), tint = if (Live.active) Color.White else inactive) {
            Live.showSheet = true
        }
        Text("Queue", style = SdatType.headline, color = colors.content)
        Row(Modifier.align(Alignment.CenterEnd)) {
            IconTap("shuffle", if (playback.shuffle) "Shuffle: On" else "Shuffle: Off", 20.dp, tint = if (playback.shuffle) Color.White else inactive) {
                playback.setShuffle(!playback.shuffle)
            }
            val repeat = playback.repeat
            IconTap(if (repeat == Repeat.One) "repeat.1" else "repeat", "Repeat: " + when (repeat) { Repeat.Off -> "Off"; Repeat.All -> "All"; Repeat.One -> "One" }, 20.dp, tint = if (repeat == Repeat.Off) inactive else Color.White) {
                playback.cycleRepeat()
            }
        }
    }
}

private fun LazyListScope.header(title: String) {
    item(key = "header $title") {
        Text(title, style = SdatType.headline, color = SdatTheme.colors.content, modifier = Modifier.padding(top = 20.dp, bottom = 4.dp))
    }
}

private class EditableQueueRows(section: String, val slice: QueueSlice<Track>) {
    val keys = occurrenceKeys(slice.tracks.map { it.id }).map { "$section $it" }
    private val offsets = keys.withIndex().associate { it.value to it.index }

    fun offset(key: Any): Int? = (key as? String)?.let(offsets::get)
}

private class QueueDrag(val rows: EditableQueueRows, val from: Int, val height: Int, center: Float, listState: LazyListState) {
    var center by mutableFloatStateOf(center)

    val destination by derivedStateOf {
        listState.layoutInfo.visibleItemsInfo
            .filter { rows.offset(it.key) != null }
            .minByOrNull { abs(center - (it.offset + it.size / 2f)) }
            ?.let { rows.offset(it.key) }
    }

    fun scrollSpeed(layout: LazyListLayoutInfo, edge: Float): Float {
        val visible = layout.visibleItemsInfo
        val top = (layout.viewportStartOffset + edge - center) / edge
        val bottom = (center - (layout.viewportEndOffset - edge)) / edge
        val first = visible.firstOrNull { it.key == rows.keys.first() }
        val last = visible.firstOrNull { it.key == rows.keys.last() }
        return when {
            top > 0f && (first == null || first.offset < layout.viewportStartOffset) -> -top.coerceAtMost(1f) * edge * 14f
            bottom > 0f && (last == null || last.offset + last.size > layout.viewportEndOffset) -> bottom.coerceAtMost(1f) * edge * 14f
            else -> 0f
        }
    }
}

private fun Modifier.queueReorder(
    listState: LazyListState,
    sections: List<EditableQueueRows>,
    padding: PaddingValues,
    direction: LayoutDirection,
    playback: Playback,
    onDrag: (QueueDrag?) -> Unit,
): Modifier = if (sections.isEmpty()) this else pointerInput(sections, padding, direction, playback) {
    val end = padding.calculateEndPadding(direction).toPx()
    val handleWidth = queueHandleWidth.toPx()
    awaitEachGesture {
        val down = awaitFirstDown(pass = PointerEventPass.Initial)
        val distanceFromEnd = if (direction == LayoutDirection.Ltr) size.width - down.position.x else down.position.x
        if (distanceFromEnd < end || distanceFromEnd > end + handleWidth) return@awaitEachGesture
        val layout = listState.layoutInfo
        val y = down.position.y + layout.viewportStartOffset
        val row = layout.visibleItemsInfo.firstOrNull { y >= it.offset && y < it.offset + it.size } ?: return@awaitEachGesture
        val rows = sections.firstOrNull { it.keys.size > 1 && it.offset(row.key) != null } ?: return@awaitEachGesture
        val from = rows.offset(row.key) ?: return@awaitEachGesture
        val rowCenter = row.offset + row.size / 2f
        var active: QueueDrag? = null
        try {
            // The list's own scroll detector would claim vertical slop on Main.
            while (true) {
                val change = awaitPointerEvent(PointerEventPass.Initial).changes.firstOrNull { it.id == down.id } ?: break
                if (change.isConsumed) break
                val delta = change.position - down.position
                val started = active
                if (started != null) {
                    change.consume()
                    if (!change.pressed) {
                        started.destination?.let { playback.moveQueueTrack(rows.slice, from, it) }
                        break
                    }
                    started.center = (rowCenter + delta.y).coerceIn(
                        layout.viewportStartOffset.toFloat(),
                        layout.viewportEndOffset.toFloat(),
                    )
                } else {
                    if (!change.pressed || abs(delta.x) > viewConfiguration.touchSlop && abs(delta.x) >= abs(delta.y)) break
                    if (abs(delta.y) > viewConfiguration.touchSlop) {
                        change.consume()
                        active = QueueDrag(rows, from, row.size, rowCenter + delta.y, listState)
                        onDrag(active)
                    }
                }
            }
        } finally {
            if (active != null) onDrag(null)
        }
    }
}

private fun LazyListScope.queueRows(
    rows: EditableQueueRows,
    onTap: (Int) -> Unit,
    playback: Playback,
    drag: () -> QueueDrag?,
) {
    val slice = rows.slice
    itemsIndexed(slice.tracks, key = { i, _ -> rows.keys[i] }) { i, track ->
        val action = "Remove ${track.displayTitle}"
        val rtl = LocalLayoutDirection.current == LayoutDirection.Rtl
        val towardsStart = if (rtl) 1 else -1
        var distance by remember(slice, i) { mutableFloatStateOf(0f) }
        Box(
            Modifier.animateItem().graphicsLayer {
                val active = drag()?.takeIf { it.rows === rows }
                alpha = if (active?.from == i) 0f else 1f
                val destination = active?.destination
                translationY = if (active == null || destination == null) 0f else when (i) {
                    in (active.from + 1)..destination -> -active.height.toFloat()
                    in destination until active.from -> active.height.toFloat()
                    else -> 0f
                }
            }.pointerInput(slice, i, rtl, playback) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    try {
                        val start = awaitHorizontalTouchSlopOrCancellation(down.id) { change, overSlop ->
                            val delta = change.position - down.position
                            if (overSlop * towardsStart > 0f && abs(delta.x) > abs(delta.y)) {
                                change.consume()
                                distance = (overSlop * towardsStart).coerceAtMost(size.width.toFloat())
                            }
                        } ?: return@awaitEachGesture
                        val ended = horizontalDrag(start.id) { change ->
                            distance = (distance + change.positionChange().x * towardsStart).coerceIn(0f, size.width.toFloat())
                            change.consume()
                        }
                        if (ended && distance >= size.width * 0.35f) playback.removeQueueTrack(slice, i)
                    } finally {
                        distance = 0f
                    }
                }
            },
        ) {
            if (distance > 0f) {
                val revealedWidth = with(LocalDensity.current) { distance.roundToInt().toDp() }
                Box(Modifier.matchParentSize()) {
                    Box(
                        Modifier.align(Alignment.CenterEnd).fillMaxHeight().width(revealedWidth)
                            .background(removeRed).clipToBounds(),
                        contentAlignment = Alignment.Center,
                    ) {
                        if (revealedWidth >= 36.dp) SdatIcon("trash", null, size = 20.dp, tint = Color.White)
                    }
                }
            }
            Box(Modifier.fillMaxWidth().clipToBounds()) {
                Row(Modifier.fillMaxWidth().offset { IntOffset(-distance.roundToInt(), 0) }.background(queueRowBackground), verticalAlignment = Alignment.CenterVertically) {
                    TrackMenuRow(
                        track,
                        onClick = { onTap(i) },
                        Modifier.weight(1f).semantics {
                            customActions = buildList {
                                add(CustomAccessibilityAction(action) { playback.removeQueueTrack(slice, i) })
                                if (i > 0) {
                                    add(CustomAccessibilityAction("Move up") { playback.moveQueueTrack(slice, i, i - 1) })
                                }
                                if (i < slice.tracks.lastIndex) {
                                    add(CustomAccessibilityAction("Move down") { playback.moveQueueTrack(slice, i, i + 1) })
                                }
                            }
                        },
                    ) { TrackRow(track, Modifier.padding(vertical = 6.dp)) }
                    IconTap("minus.circle", action, 20.dp, tint = removeRed) { playback.removeQueueTrack(slice, i) }
                    if (slice.tracks.size > 1) QueueDragHandle()
                }
            }
        }
    }
}

@Composable
private fun QueueDragHandle() {
    Box(Modifier.size(queueHandleWidth), contentAlignment = Alignment.Center) {
        Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
            repeat(2) {
                Spacer(Modifier.size(16.dp, 2.dp).background(dimmed, RoundedCornerShape(1.dp)))
            }
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
