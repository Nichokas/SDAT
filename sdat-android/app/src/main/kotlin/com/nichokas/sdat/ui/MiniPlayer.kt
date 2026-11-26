package com.nichokas.sdat.ui

import android.view.HapticFeedbackConstants
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathMeasure
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.AppTab
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.MiniFan
import com.nichokas.sdat.core.NavState
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.ui.kit.Raised
import com.nichokas.sdat.ui.kit.SdatTheme
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.math.roundToInt

private val AppTab.label get() = when (this) {
    AppTab.Discover -> "Discover"
    AppTab.Search -> "Search"
    AppTab.Collection -> "My Collection"
}

/** Outlined; the selected tab adds `.fill`. */
private val AppTab.symbol get() = when (this) {
    AppTab.Discover -> "safari"
    AppTab.Search -> "magnifyingglass"
    AppTab.Collection -> "music.note.house"
}

private const val MINI = 56
private const val RING = 3
private val miniCorner = 20.dp
private val coverCorner = 14.dp

/** The bottom bar: one slot per tab, and the mini player in the last slot while something is queued. */
@Composable
internal fun BottomBar(playback: Playback, nav: NavState) {
    val colors = SdatTheme.colors
    Row(
        Modifier.fillMaxWidth().background(colors.sheet).navigationBarsPadding().height(64.dp).padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        for (tab in AppTab.entries) {
            Box(Modifier.weight(1f), contentAlignment = Alignment.Center) { TabButton(tab, nav.tab == tab) { Chrome.select(tab) } }
        }
        if (playback.track != null) Box(Modifier.weight(1f), contentAlignment = Alignment.Center) { MiniPlayer(playback) }
    }
    if (Chrome.showPlayer && playback.track != null) PlayerSheet(playback) { Chrome.showPlayer = false }
}

@Composable
private fun TabButton(tab: AppTab, selected: Boolean, onClick: () -> Unit) {
    val colors = SdatTheme.colors
    Box(
        Modifier.size(64.dp, 44.dp).clip(CircleShape)
            .then(if (selected) Modifier.background(colors.fill) else Modifier)
            .selectable(selected, role = Role.Tab, onClick = onClick)
            .semantics { contentDescription = tab.label },
        contentAlignment = Alignment.Center,
    ) {
        SdatIcon(if (selected) "${tab.symbol}.fill" else tab.symbol, null, tint = if (selected) colors.content else colors.secondaryContent)
    }
}

/** The cover inside a progress ring. A tap opens the player and a hold opens [MiniFan]. */
@Composable
private fun MiniPlayer(playback: Playback) {
    val track = playback.track ?: return
    val onOpen = { Chrome.showPlayer = true }
    val label = playLabel(playback)
    val focus = remember { MutableInteractionSource() }
    val focused by focus.collectIsFocusedAsState()
    val view = LocalView.current
    // Whatever hides the mini player (the end of the queue, Settings) also takes the fan with it.
    DisposableEffect(Unit) { onDispose { MiniFan.close() } }
    Box(
        Modifier.size(MINI.dp)
            .then(if (focused) Modifier.border(2.dp, SdatTheme.colors.content, RoundedCornerShape(miniCorner)) else Modifier)
            .semantics {
                role = Role.Button
                contentDescription = "Now playing, ${track.title}"
                onClick("Open player") { onOpen(); true }
                // The fan's controls, as actions: the fan itself is a drag, which assistive technology can't do.
                customActions = listOf(
                    CustomAccessibilityAction(label) { runFanItem(MiniFan.Item.Toggle, playback); true },
                    CustomAccessibilityAction("Previous") { runFanItem(MiniFan.Item.Previous, playback); true },
                    CustomAccessibilityAction("Next") { runFanItem(MiniFan.Item.Next, playback); true },
                )
            }
            .onKeyEvent { e ->
                val activate = e.key == Key.Enter || e.key == Key.NumPadEnter || e.key == Key.Spacebar
                if (activate && e.type == KeyEventType.KeyUp) onOpen()
                activate
            }
            .focusable(interactionSource = focus)
            .onGloballyPositioned { MiniFan.anchor = it.boundsInRoot().center }
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    var up: PointerInputChange? = null
                    val released = withTimeoutOrNull(MiniFan.HOLD_MS) { up = waitForUpOrCancellation(); true }
                    // A cancelled gesture (finger slid off, or consumed by another gesture) does nothing.
                    if (released != null) {
                        if (up != null) onOpen()
                        return@awaitEachGesture
                    }
                    view.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
                    MiniFan.hold()
                    try {
                        while (true) {
                            val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                            // Compose delivers a cancelled gesture as an up that is already consumed: it must not run the
                            // control under the finger.
                            if (change.isConsumed) break
                            change.consume()
                            val d = change.position - down.position
                            val moved = MiniFan.move(d.x.toDp().value, d.y.toDp().value)
                            if (!change.pressed) {
                                MiniFan.release()?.let { runFanItem(it, playback) }
                                break
                            }
                            if (moved) view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                        }
                    } finally {
                        if (MiniFan.state is MiniFan.State.Held) MiniFan.close()
                    }
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        MiniRing({ if (playback.loading) 0.0 else playback.progress }, playback.loading, Modifier.fillMaxSize())
        Box(Modifier.size((MINI - 2 * (RING + 3)).dp), contentAlignment = Alignment.Center) {
            Cover(track.coverUrl, (MINI - 2 * (RING + 3)).dp, radius = coverCorner)
            // Playing shows the cover alone; anything else says why it isn't playing.
            val symbol = when {
                playback.error != null -> "exclamationmark.triangle.fill"
                playback.loading || playback.playing -> null
                else -> "play.fill"
            }
            if (symbol != null) {
                Box(Modifier.fillMaxSize().clip(RoundedCornerShape(coverCorner)).background(Color.Black.copy(alpha = 0.4f)), contentAlignment = Alignment.Center) {
                    SdatIcon(symbol, null, size = 22.dp, tint = Color.White)
                }
            }
        }
    }
}

/**
 * The played part in the accent, clockwise from the top centre of a rounded square, over the rest in the separator
 * colour. While [spinning], a quarter of the ring runs around it instead. [progress] is a lambda so the player's position
 * ticks redraw the ring, not its caller.
 */
@Composable
private fun MiniRing(progress: () -> Double, spinning: Boolean, modifier: Modifier) {
    val colors = SdatTheme.colors
    val q by animateFloatAsState(progress().coerceIn(0.0, 1.0).toFloat(), tween(250, easing = LinearEasing), label = "progress")
    // Composed only while spinning: an idle infinite transition would keep animating unseen.
    val phase = if (spinning) {
        rememberInfiniteTransition(label = "spin").animateFloat(0f, 1f, infiniteRepeatable(tween(1200, easing = LinearEasing)), label = "phase")
    } else {
        null
    }
    Canvas(modifier) {
        val w = RING.dp.toPx()
        val inset = w / 2
        val r = miniCorner.toPx() - inset
        val left = inset
        val top = inset
        val right = size.width - inset
        val bottom = size.height - inset
        val stroke = Stroke(w, cap = StrokeCap.Round)
        drawRoundRect(colors.separator, Offset(left, top), Size(right - left, bottom - top), CornerRadius(r), style = Stroke(w))
        // Traced by hand from the top centre: addRoundRect starts at a corner.
        val path = Path().apply {
            moveTo(size.width / 2, top)
            lineTo(right - r, top)
            arcTo(Rect(right - 2 * r, top, right, top + 2 * r), -90f, 90f, false)
            lineTo(right, bottom - r)
            arcTo(Rect(right - 2 * r, bottom - 2 * r, right, bottom), 0f, 90f, false)
            lineTo(left + r, bottom)
            arcTo(Rect(left, bottom - 2 * r, left + 2 * r, bottom), 90f, 90f, false)
            lineTo(left, top + r)
            arcTo(Rect(left, top, left + 2 * r, top + 2 * r), 180f, 90f, false)
            lineTo(size.width / 2, top)
        }
        val measure = PathMeasure().apply { setPath(path, false) }
        val length = measure.length
        val played = Path()
        if (phase != null) {
            val start = phase.value * length
            val end = start + length / 4
            measure.getSegment(start, minOf(end, length), played, true)
            if (end > length) measure.getSegment(0f, end - length, played, true)
        } else if (q > 0f) {
            measure.getSegment(0f, q * length, played, true)
        }
        drawPath(played, colors.accent, style = stroke)
    }
}

private fun playLabel(playback: Playback) = when {
    playback.error != null -> "Retry playback"
    playback.playing -> "Pause"
    else -> "Play"
}

private fun runFanItem(item: MiniFan.Item, playback: Playback) {
    when (item) {
        MiniFan.Item.Close -> MiniFan.close()
        MiniFan.Item.Toggle -> Live.userToggle()
        MiniFan.Item.Previous -> if (!Live.guestRefuses()) playback.previous()
        MiniFan.Item.Next -> Live.userNext()
    }
}

/** Draws [MiniFan] over the whole screen, so the controls can leave the mini player and a tap beside them closes it. */
@Composable
internal fun MiniFanOverlay(playback: Playback) {
    val state = MiniFan.state
    val shown = state != MiniFan.State.Closed
    val t = remember { Animatable(0f) }
    LaunchedEffect(shown) { t.animateTo(if (shown) 1f else 0f, spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow)) }
    BackHandler(enabled = shown) { MiniFan.close() }
    if (!shown && t.value == 0f) return
    val hover = (state as? MiniFan.State.Held)?.hover
    var origin by remember { mutableStateOf(Offset.Zero) }
    Box(
        Modifier.fillMaxSize()
            .onGloballyPositioned { origin = it.positionInRoot() }
            .drawBehind { drawRect(Color.Black, alpha = 0.32f * t.value.coerceIn(0f, 1f)) }
            // Only once lifted: while held the finger still belongs to the mini player, and a closing fan must not swallow taps.
            .then(if (state == MiniFan.State.Open) Modifier.pointerInput(Unit) { detectTapGestures { MiniFan.close() } } else Modifier),
    ) {
        for (item in MiniFan.Item.entries) {
            FanButton(item, playback, hovered = item == hover, tappable = state == MiniFan.State.Open, t = { t.value }) {
                MiniFan.anchor - origin
            }
        }
    }
}

@Composable
private fun FanButton(item: MiniFan.Item, playback: Playback, hovered: Boolean, tappable: Boolean, t: () -> Float, center: () -> Offset) {
    val colors = SdatTheme.colors
    val (symbol, label) = when (item) {
        MiniFan.Item.Close -> "xmark" to "Close"
        MiniFan.Item.Toggle -> (if (playback.playing) "pause.fill" else "play.fill") to playLabel(playback)
        MiniFan.Item.Previous -> "backward.fill" to "Previous"
        MiniFan.Item.Next -> "forward.fill" to "Next"
    }
    val locked = when (item) {
        MiniFan.Item.Close -> false
        MiniFan.Item.Previous -> Live.isGuest
        else -> Live.playbackLocked
    }
    val lift by animateFloatAsState(if (hovered) 1.15f else 1f, spring(dampingRatio = 0.6f), label = "lift")
    val size = item.size.dp
    val toggle = item == MiniFan.Item.Toggle
    val content = if (toggle) colors.onInverse else colors.content
    Raised(
        Modifier
            .offset {
                val c = center()
                val half = size.toPx() / 2
                val p = t()
                IntOffset((c.x + item.x.dp.toPx() * p - half).roundToInt(), (c.y + item.y.dp.toPx() * p - half).roundToInt())
            }
            .size(size)
            .graphicsLayer {
                val p = t()
                // The close button replaces the mini player in place; the controls grow out of it.
                val s = (if (item == MiniFan.Item.Close) 1f else 0.4f + 0.6f * p) * lift
                scaleX = s
                scaleY = s
                alpha = p.coerceIn(0f, 1f)
            },
        shape = if (item == MiniFan.Item.Close) RoundedCornerShape(miniCorner) else CircleShape,
        color = if (toggle) colors.inverse else colors.sheet,
        contentColor = content,
    ) {
        Box(
            Modifier.fillMaxSize()
                .then(if (hovered) Modifier.background(colors.accent.copy(alpha = 0.3f)) else Modifier)
                // Absent, not disabled, while closing: a disabled clickable still takes the touch from the player beneath.
                .then(if (tappable) Modifier.clickable(role = Role.Button, onClickLabel = label) { runFanItem(item, playback) } else Modifier),
            contentAlignment = Alignment.Center,
        ) {
            SdatIcon(symbol, label, size = size * 0.46f, tint = content.copy(alpha = if (locked) 0.4f else 1f))
        }
    }
}
