package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitHorizontalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.horizontalDrag
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.foundation.layout.absoluteOffset
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.QueueSlice
import com.nichokas.sdat.core.Repeat
import com.nichokas.sdat.core.displayTitle
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import kotlin.math.abs
import kotlin.math.roundToInt
import uniffi.tonearm_ffi.Track

private val dimmed = Color.White.copy(alpha = 0.65f)
private val inactive = Color.White.copy(alpha = 0.45f)
private val queueHandleWidth = 44.dp
private val removeRed = Color(0xFFFF453A)
private val queueRowBackground = Color.White.copy(alpha = 0.06f)
@Composable
internal fun QueuePage(playback: Playback) {
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
                Row(Modifier.fillMaxWidth().absoluteOffset { IntOffset(towardsStart * distance.roundToInt(), 0) }.background(queueRowBackground), verticalAlignment = Alignment.CenterVertically) {
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
