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
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableDoubleStateOf
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
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
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
import kotlin.math.cos
import kotlin.math.hypot
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track

private val dimmed = Color.White.copy(alpha = 0.65f)
private val quiet = Color.White.copy(alpha = 0.85f)

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
                    .semantics { contentDescription = if (playback.playing) "Pause" else "Play" }
                    .clickable(role = Role.Button) { Live.userToggle() },
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
internal fun IconTap(
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
