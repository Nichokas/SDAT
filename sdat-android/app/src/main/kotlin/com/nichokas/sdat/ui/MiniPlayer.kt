package com.nichokas.sdat.ui

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.layout.layout
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.onLongClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Live
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
 * The iOS mini-player. Circle: a tap opens the player and a short hold toggles play. Capsule and wide: a tap opens the
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
    Box(
        modifier.width(Morph.CIRCLE.dp).fillMaxHeight()
            .then(if (active && focused) Modifier.border(2.dp, GlasmioTheme.colors.content, capsule) else Modifier)
            .semantics {
                if (!enabled) return@semantics
                role = Role.Button
                contentDescription = track?.let { "$label ${it.title}" } ?: label
                if (track == null) disabled()
                onClick("Open player") { onOpen(); true }
                onLongClick(label) { Live.userToggle(); true }
            }
            // Only while active: the faded circle sits over the capsule's ring and next, and would take their touches.
            .then(if (!active) Modifier else Modifier.onKeyEvent { e ->
                val activate = e.key == Key.Enter || e.key == Key.NumPadEnter || e.key == Key.Spacebar
                if (activate && e.type == KeyEventType.KeyUp) onOpen()
                activate
            }.focusable(interactionSource = focus).pointerInput(Unit) {
                awaitEachGesture {
                    awaitFirstDown()
                    // iOS PressRecognizer.miniHold: released within 0.12 s is a tap, otherwise a hold.
                    var up: PointerInputChange? = null
                    val released = withTimeoutOrNull(120) { up = waitForUpOrCancellation(); true }
                    // A cancelled gesture (finger slid off, or consumed by another gesture) does nothing.
                    if (released == null) Live.userToggle() else if (up != null) onOpen()
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
