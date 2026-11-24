package com.nichokas.sdat.ui

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.AudioCache
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.DownloadState
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.badgeText
import com.nichokas.sdat.core.displayTitle
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonLayout
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonRole
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonSpec
import io.github.neoruaa.glasmio.ui.GlasmioAlertDialog
import io.github.neoruaa.glasmio.ui.GlasmioCascadingMenuItem
import uniffi.tonearm_ffi.Track

private val orange = Color(0xFFFF9500)

/** Download, cancel or remove for one or more tracks (iOS DownloadButton): the track menu and album and playlist menus. */
fun downloadMenuItems(tracks: List<Track>): List<GlasmioCascadingMenuItem> {
    val ids = tracks.map { it.id }
    val states = ids.map(Downloads::state)
    val many = tracks.size > 1
    val failed = states.firstNotNullOfOrNull { (it as? DownloadState.Failed)?.message }
    return buildList {
        when {
            states.any { it is DownloadState.Downloading || it == DownloadState.Queued } -> {
                val one = states.singleOrNull() as? DownloadState.Downloading
                val title = if (one != null) "Downloading… ${(one.fraction * 100).toInt()}%" else "Downloading…"
                add(GlasmioCascadingMenuItem(title = title, systemName = "arrow.down.circle", enabled = false, onClick = {}))
                add(GlasmioCascadingMenuItem(title = "Cancel Download", systemName = "xmark.circle", onClick = { ids.forEach(Downloads::cancel) }))
            }
            ids.isNotEmpty() && ids.all(Downloads::isDownloaded) ->
                add(GlasmioCascadingMenuItem(title = if (many) "All Downloaded" else "Downloaded", systemName = "arrow.down.circle.fill", enabled = false, onClick = {}))
            else -> add(
                GlasmioCascadingMenuItem(
                    title = if (failed != null) "Retry Download" else if (many) "Download All" else "Download",
                    systemName = if (failed != null) "arrow.clockwise" else "arrow.down.circle",
                    subtitle = failed?.let { "Download failed: $it" },
                    onClick = { Downloads.download(tracks); Chrome.flash("Downloading") },
                ),
            )
        }
        if (ids.any(Downloads::isDownloaded)) {
            add(GlasmioCascadingMenuItem(title = if (many) "Remove Downloads" else "Remove Download", systemName = "trash", destructive = true, onClick = { Downloads.remove(tracks) }))
        }
    }
}

/** Small mark on track rows: a ring while queued or downloading, a filled arrow once saved. */
@Composable
fun DownloadBadge(id: String) {
    // Every fragment updates the queue; only the row whose state changed redraws.
    val state by remember(id) { derivedStateOf { Downloads.state(id) } }
    when (val s = state) {
        DownloadState.Done -> SdatIcon("arrow.down.circle.fill", "Downloaded", size = 14.dp, tint = GlasmioTheme.colors.secondaryContent)
        is DownloadState.Downloading -> DownloadRing(s.fraction)
        DownloadState.Queued -> DownloadRing(0.0)
        is DownloadState.Failed -> SdatIcon("exclamationmark.circle", "Download failed", size = 14.dp, tint = orange)
        DownloadState.None -> {}
    }
}

@Composable
private fun DownloadRing(value: Double, size: Dp = 13.dp) {
    val v by animateFloatAsState(maxOf(0.02f, value.toFloat()), tween(200), label = "download")
    Canvas(Modifier.size(size).semantics { contentDescription = "Downloading" }) {
        val w = 2.dp.toPx()
        drawCircle(Color.White.copy(alpha = 0.2f), radius = (this.size.minDimension - w) / 2, style = Stroke(w))
        drawArc(Color.White, -90f, 360f * v, false, topLeft = Offset(w / 2, w / 2),
            size = Size(this.size.width - w, this.size.height - w), style = Stroke(w, cap = StrokeCap.Round))
    }
}

/** Shown beside the profile button while downloads run; the ring is the whole batch's progress. */
@Composable
fun DownloadStatusButton(modifier: Modifier = Modifier) {
    val p by animateFloatAsState(Downloads.queue.batchProgress.toFloat(), tween(350), label = "batch")
    GlassSurface(modifier.size(44.dp), shape = CircleShape) {
        Box(
            Modifier.fillMaxSize()
                .clickable(role = Role.Button, onClickLabel = "Downloads") { Chrome.push(Route.Downloads) }
                .semantics { contentDescription = "Downloads"; stateDescription = "${(p * 100).toInt()} percent" },
            contentAlignment = Alignment.Center,
        ) {
            SdatIcon("arrow.down", null, size = 20.dp, tint = Color.White)
            Canvas(Modifier.fillMaxSize().padding(1.5.dp)) {
                val w = 3.dp.toPx()
                drawArc(iosBlue, -90f, 360f * maxOf(0.001f, p), false, topLeft = Offset(w / 2, w / 2),
                    size = Size(size.width - w, size.height - w), style = Stroke(w, cap = StrokeCap.Round))
            }
        }
    }
}

@Composable
fun DownloadsView() {
    val player = LocalTrackPlayer.current
    val saved = Downloads.tracks
    val cached = AudioCache.tracks.filter { !Downloads.isDownloaded(it.track.id) }
    val cachedTracks = cached.map { it.track }
    val queue = Downloads.queue
    val active = queue.active
    val failed = queue.failures.entries.sortedBy { it.key }
    var confirmAll by remember { mutableStateOf(false) }
    val secondary = GlasmioTheme.colors.secondaryContent
    LaunchedEffect(Unit) { AudioCache.refresh() }
    SdatPage("Downloads", onBack = Chrome::pop, actions = {
        if (saved.isNotEmpty()) {
            DetailMenu("Downloads", listOf(
                GlasmioCascadingMenuItem(title = "Play", systemName = "play.fill", onClick = { player.playInOrder(saved, "Downloads") }),
                GlasmioCascadingMenuItem(title = "Shuffle", systemName = "shuffle", onClick = { player.playShuffled(saved, "Downloads") }),
                GlasmioCascadingMenuItem(title = "Delete All Downloads", systemName = "trash", destructive = true, onClick = { confirmAll = true }),
            ))
        }
    }) {
        if (active.isNotEmpty()) {
            item(key = "active-header") {
                SectionHeader("Downloading", style = IosTypography.footnote) {
                    Text(
                        "Cancel all",
                        style = IosTypography.footnote,
                        color = GlasmioTheme.colors.accent,
                        modifier = Modifier.clickable(role = Role.Button) { Downloads.cancelAll() }.padding(vertical = 4.dp),
                    )
                }
            }
            items(active, key = { "active:${it.id}" }) { t ->
                Row(Modifier.fillMaxWidth().padding(horizontal = ListGutter, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                    MediaRow(t.displayTitle, t.artistLine, t.album?.coverUrl, Modifier.weight(1f))
                    DownloadBadge(t.id)
                    Box(Modifier.padding(start = 8.dp).size(36.dp).clickable(role = Role.Button) { Downloads.cancel(t.id) }, contentAlignment = Alignment.Center) {
                        SdatIcon("xmark.circle", "Cancel", size = 20.dp, tint = secondary)
                    }
                }
            }
        }
        if (failed.isNotEmpty()) {
            item(key = "failed-header") { SectionHeader("Failed", Modifier.padding(top = 12.dp), style = IosTypography.footnote) }
            items(failed, key = { "failed:${it.key}" }) { (_, message) ->
                ErrorBanner(message, Modifier.padding(horizontal = ListGutter, vertical = 4.dp))
            }
        }
        item(key = "saved-header") {
            if (saved.isEmpty()) {
                Text(
                    "Downloaded tracks play without a connection. Use “Download” in a track, album or playlist menu.",
                    style = IosTypography.footnote,
                    color = secondary,
                    modifier = Modifier.padding(horizontal = ListGutter, vertical = 12.dp),
                )
            } else {
                SectionHeader("${saved.size} tracks · ${Fmt.bytes(Downloads.totalSize)}", Modifier.padding(top = 12.dp), style = IosTypography.footnote)
            }
        }
        itemsIndexed(saved, key = { i, t -> "saved:${t.id}#$i" }) { i, t ->
            val entry = Downloads.entries[t.id]
            TrackMenuRow(t, onClick = { player.play(saved, i, "Downloads") }) {
                Row(Modifier.fillMaxWidth().padding(horizontal = ListGutter, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                    TrackRow(t, Modifier.weight(1f))
                    Column(Modifier.padding(start = 8.dp), horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(Fmt.bytes(entry?.size ?: 0), style = IosTypography.caption1.copy(fontFeatureSettings = "tnum"), color = secondary)
                        entry?.qualityText?.let { Text(it, style = IosTypography.caption2, fontWeight = FontWeight.SemiBold, color = secondary, maxLines = 1) }
                    }
                }
            }
        }
        if (cached.isNotEmpty()) {
            item(key = "cached-header") {
                SectionHeader("Cached · ${cached.size} tracks · ${Fmt.cacheBytes(cached.sumOf { it.bytes.toLong() })}", Modifier.padding(top = 12.dp), style = IosTypography.footnote)
            }
            itemsIndexed(cached, key = { i, c -> "cached:${c.track.id}#$i" }) { i, c ->
                val remove = GlasmioCascadingMenuItem(title = "Remove from Cache", systemName = "trash", destructive = true, onClick = { AudioCache.remove(c.track.id) })
                TrackMenuRow(c.track, onClick = { player.play(cachedTracks, i, "Cached") }, extraItems = listOf(remove)) {
                    Row(Modifier.fillMaxWidth().padding(horizontal = ListGutter, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                        TrackRow(c.track, Modifier.weight(1f))
                        Column(Modifier.padding(start = 8.dp), horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(Fmt.cacheBytes(c.bytes.toLong()), style = IosTypography.caption1.copy(fontFeatureSettings = "tnum"), color = secondary)
                            Text(c.quality.badgeText, style = IosTypography.caption2, fontWeight = FontWeight.SemiBold, color = secondary, maxLines = 1)
                        }
                    }
                }
            }
        }
    }
    if (confirmAll) {
        GlasmioAlertDialog(
            onDismissRequest = { confirmAll = false },
            title = "Delete all downloads?",
            message = null,
            buttonLayout = GlasmioAlertButtonLayout.Stacked,
            buttons = listOf(
                GlasmioAlertButtonSpec("Delete ${Fmt.bytes(Downloads.totalSize)}", { confirmAll = false; Downloads.removeAll() }, GlasmioAlertButtonRole.Destructive),
                GlasmioAlertButtonSpec("Cancel", { confirmAll = false }, GlasmioAlertButtonRole.Cancel),
            ),
        )
    }
}
