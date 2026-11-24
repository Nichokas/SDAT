package com.nichokas.sdat.ui

import android.view.HapticFeedbackConstants
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
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
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.MiniFan
import com.nichokas.sdat.core.Morph
import com.nichokas.sdat.core.NavState
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.coverUrl
import com.nichokas.sdat.core.displayTitle
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.ui.GlasmioNavigationBar
import kotlin.math.roundToInt
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.tonearm_ffi.Track

internal val capsule = RoundedCornerShape(percent = 50)

private enum class MiniStyle { Circle, Capsule, Wide }

/**
 * iOS bottom chrome. At a tab root the tab bar sits beside the circle mini-player, and scrolling down morphs them into a
 * tab circle beside a capsule. Pushed pages show only a wide mini-player.
 */
@Composable
internal fun BottomChrome(playback: Playback, nav: NavState, modifier: Modifier) {
    val hasMini = playback.track != null
    val morphs = hasMini && !nav.inDetail
    val bar = Chrome.bar
    // Snapped to expanded while the morph is hidden, so a mini-player appearing or a return from a pushed page animates in
    // from the start.
    val clock = remember { Animatable(0f) }
    LaunchedEffect(morphs, bar.collapsed) {
        if (morphs) clock.animateTo(if (bar.collapsed) 1f else 0f, tween(Morph.DURATION_MS, easing = LinearEasing)) else clock.snapTo(0f)
    }
    BoxWithConstraints(modifier.fillMaxWidth().navigationBarsPadding().padding(start = 24.dp, end = 24.dp, bottom = 12.dp)) {
        val widthDp = maxWidth.value
        // Deferred out of composition so the 400 ms morph drives layout and draw phases directly instead of recomposing every frame.
        val frame = remember(widthDp, morphs, bar.collapsing) {
            derivedStateOf { Morph.frame(widthDp, if (morphs) clock.value else 0f, bar.collapsing) }
        }
        // Glasmio lerps the bar from its width constraint to compactSize, which at tabMax is exactly Morph's tab width.
        val compact = remember(frame, morphs) {
            derivedStateOf { if (morphs) frame.value.tabFraction else 0f }
        }
        if (!nav.inDetail) {
            GlasmioNavigationBar(
                items = tabItems,
                selectedKey = nav.tab,
                onSelected = Chrome::select,
                onExpand = { Chrome.setCollapsed(false) },
                compactProgress = compact,
                compactSize = Morph.ROUND.dp,
                modifier = Modifier.align(Alignment.CenterStart)
                    .then(if (hasMini) Modifier.width(Morph.tabMax(widthDp).dp) else Modifier.fillMaxWidth()),
            )
        }
        if (hasMini) {
            val wide = nav.inDetail
            val style by remember(frame, wide) {
                derivedStateOf {
                    when {
                        wide -> MiniStyle.Wide
                        frame.value.miniFraction >= 0.5f -> MiniStyle.Capsule
                        else -> MiniStyle.Circle
                    }
                }
            }
            MiniPlayer(
                playback,
                style = style,
                fraction = { if (wide) 1f else frame.value.miniFraction },
                modifier = Modifier.align(Alignment.CenterEnd).then(
                    if (wide) Modifier.fillMaxWidth().height(Morph.CIRCLE.dp)
                    else Modifier.layout { measurable, constraints ->
                        val f = frame.value
                        val w = f.miniWidth.dp.roundToPx().coerceIn(constraints.minWidth, constraints.maxWidth)
                        val h = (Morph.CIRCLE - 8 * f.miniFraction).dp.roundToPx().coerceIn(constraints.minHeight, constraints.maxHeight)
                        val placeable = measurable.measure(Constraints.fixed(w, h))
                        layout(w, h) { placeable.place(0, 0) }
                    },
                ),
            )
        }
    }
}

/**
 * The iOS mini-player. Circle: a tap opens the player and a hold opens [MiniFan]. Capsule and wide: a tap opens the
 * player and the ring toggles play; wide adds next. [fraction] runs the crossfade from circle (0) to capsule (1).
 */
@Composable
private fun MiniPlayer(playback: Playback, style: MiniStyle, fraction: () -> Float, modifier: Modifier) {
    val track = playback.track
    val openPlayer = { Chrome.showPlayer = true }
    GlassSurface(modifier, shape = capsule) {
        if (track != null) {
            CapsuleContent(
                playback,
                track,
                wide = style == MiniStyle.Wide,
                onOpen = openPlayer,
                modifier = Modifier.align(Alignment.CenterEnd).morphLayer(
                    alpha = { if (style == MiniStyle.Wide) 1f else Morph.inAlpha(fraction()) },
                    scale = { if (style == MiniStyle.Wide) 1f else Morph.inScale(fraction()) },
                    pivotX = 1f,
                ),
                enabled = style != MiniStyle.Circle,
            )
        }
        CircleContent(
            playback,
            onOpen = openPlayer,
            modifier = Modifier.align(Alignment.CenterEnd).morphLayer(
                alpha = { if (style == MiniStyle.Wide) 0f else Morph.outAlpha(fraction()) },
                scale = { Morph.outScale(fraction()) },
            ),
            enabled = style == MiniStyle.Circle,
        )
    }
    if (Chrome.showPlayer && track != null) PlayerSheet(playback) { Chrome.showPlayer = false }
}

private fun Modifier.morphLayer(
    alpha: () -> Float,
    scale: () -> Float,
    pivotX: Float = 0.5f,
) = graphicsLayer {
    this.alpha = alpha()
    val s = scale()
    scaleX = s
    scaleY = s
    transformOrigin = TransformOrigin(pivotX, 0.5f)
}

@Composable
private fun CircleContent(playback: Playback, onOpen: () -> Unit, modifier: Modifier, enabled: Boolean) {
    val track = playback.track
    val label = playLabel(playback)
    val active = enabled && track != null
    val focus = remember { MutableInteractionSource() }
    val focused by focus.collectIsFocusedAsState()
    val view = LocalView.current
    // Whatever hides the circle (end of the queue, a pushed page) also takes the fan with it.
    if (active) DisposableEffect(Unit) { onDispose { MiniFan.close() } }
    Box(
        modifier.width(Morph.CIRCLE.dp).fillMaxHeight()
            .then(if (active && focused) Modifier.border(2.dp, GlasmioTheme.colors.content, capsule) else Modifier)
            .semantics {
                if (!enabled) return@semantics
                role = Role.Button
                contentDescription = track?.let { "$label ${it.title}" } ?: label
                if (track == null) disabled()
                onClick("Open player") { onOpen(); true }
                // The fan's controls, as actions: the fan itself is a drag, which assistive technology can't do.
                customActions = listOf(
                    CustomAccessibilityAction(label) { runFanItem(MiniFan.Item.Toggle, playback); true },
                    CustomAccessibilityAction("Previous") { runFanItem(MiniFan.Item.Previous, playback); true },
                    CustomAccessibilityAction("Next") { runFanItem(MiniFan.Item.Next, playback); true },
                )
            }
            // Only while active: the faded circle sits over the capsule's ring and next, and would take their touches.
            .then(if (!active) Modifier else Modifier.onKeyEvent { e ->
                val activate = e.key == Key.Enter || e.key == Key.NumPadEnter || e.key == Key.Spacebar
                if (activate && e.type == KeyEventType.KeyUp) onOpen()
                activate
            }.focusable(interactionSource = focus).onGloballyPositioned { MiniFan.anchor = it.boundsInRoot().center }.pointerInput(Unit) {
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
                            change.consume()
                            if (!change.pressed) {
                                MiniFan.release()?.let { runFanItem(it, playback) }
                                break
                            }
                            val d = change.position - down.position
                            if (MiniFan.move(d.x.toDp().value, d.y.toDp().value)) view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                        }
                    } finally {
                        if (MiniFan.state is MiniFan.State.Held) MiniFan.close()
                    }
                }
            }),
        contentAlignment = Alignment.Center,
    ) {
        RingIcon(playback, Morph.CIRCLE.dp, active = enabled)
    }
}

@Composable
private fun CapsuleContent(playback: Playback, track: Track, wide: Boolean, onOpen: () -> Unit, modifier: Modifier, enabled: Boolean) {
    val colors = GlasmioTheme.colors
    Row(
        modifier.fillMaxSize()
            .clickable(enabled = enabled, onClickLabel = "Open player", interactionSource = null, indication = null, onClick = onOpen)
            .padding(start = 9.dp, end = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Cover(track.coverUrl, 38.dp)
        Column(Modifier.weight(1f)) {
            Text(track.displayTitle, style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(track.artistLine, style = IosTypography.caption1, color = colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        val label = playLabel(playback)
        Box(
            Modifier.clickable(enabled = enabled, onClickLabel = label, role = Role.Button, interactionSource = null, indication = null) { Live.userToggle() }
                .alpha(if (Live.playbackLocked) 0.4f else 1f)
                .semantics { contentDescription = label },
        ) { RingIcon(playback, 40.dp, active = enabled) }
        if (wide) {
            Box(
                Modifier.size(36.dp, 40.dp).clickable(enabled = enabled, role = Role.Button, interactionSource = null, indication = null) { Live.userNext() }
                    .alpha(if (Live.playbackLocked) 0.4f else 1f),
                contentAlignment = Alignment.Center,
            ) { SdatIcon("forward.fill", contentDescription = "Next", size = 18.dp, tint = colors.content) }
        }
    }
}

private fun playLabel(playback: Playback) = when {
    playback.error != null -> "Retry playback"
    playback.playing -> "Pause"
    else -> "Play"
}

@Composable
private fun RingIcon(playback: Playback, size: Dp, active: Boolean) {
    val symbol = when {
        playback.error != null -> "exclamationmark.triangle.fill"
        playback.playing -> "pause.fill"
        else -> "play.fill"
    }
    val tint = if (playback.track == null) GlasmioTheme.colors.tertiaryContent else GlasmioTheme.colors.content
    Box(Modifier.size(size), contentAlignment = Alignment.Center) {
        // Hidden, not removed, while loading: the ring spins instead of a spinner.
        SdatIcon(symbol, contentDescription = null, size = size * 0.32f, tint = tint, modifier = Modifier.alpha(if (playback.loading) 0f else 1f))
        // Only the visible style spins so the hidden alpha=0 ring doesn't run a second infinite transition.
        ProgressRing({ if (playback.loading) 0.0 else playback.progress }, playback.loading && active, Modifier.fillMaxSize().padding(size * 0.14f))
    }
}

private fun runFanItem(item: MiniFan.Item, playback: Playback) {
    when (item) {
        MiniFan.Item.Close -> MiniFan.close()
        MiniFan.Item.Toggle -> Live.userToggle()
        MiniFan.Item.Previous -> if (!Live.guestRefuses()) playback.previous()
        MiniFan.Item.Next -> Live.userNext()
    }
}

/** Draws [MiniFan] over the whole screen, so the controls can leave the circle and a tap beside them closes it. */
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
            .drawBehind { drawRect(Color.Black, alpha = 0.25f * t.value.coerceIn(0f, 1f)) }
            // Only once lifted: while held the finger still belongs to the circle, and a closing fan must not swallow taps.
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
    GlassSurface(
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
                // The close button replaces the circle in place; the controls grow out of it.
                val s = (if (item == MiniFan.Item.Close) 1f else 0.4f + 0.6f * p) * lift
                scaleX = s
                scaleY = s
                alpha = p.coerceIn(0f, 1f)
            },
        shape = CircleShape,
    ) {
        Box(
            Modifier.fillMaxSize()
                .then(if (hovered) Modifier.background(iosBlue.copy(alpha = 0.3f), CircleShape) else Modifier)
                .clickable(enabled = tappable, role = Role.Button, onClickLabel = label) { runFanItem(item, playback) },
            contentAlignment = Alignment.Center,
        ) {
            SdatIcon(symbol, label, size = size * 0.42f, tint = GlasmioTheme.colors.content.copy(alpha = if (locked) 0.4f else 1f))
        }
    }
}
