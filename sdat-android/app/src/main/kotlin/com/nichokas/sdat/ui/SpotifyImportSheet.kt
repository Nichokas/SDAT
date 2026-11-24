package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.SpotifyImporter
import com.nichokas.sdat.core.SpotifyImporter.State
import com.nichokas.sdat.ui.kit.BottomSheet
import com.nichokas.sdat.ui.kit.GroupedFooter
import com.nichokas.sdat.ui.kit.GroupedList
import com.nichokas.sdat.ui.kit.ListRow
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SectionTitle
import com.nichokas.sdat.ui.kit.TextField
import uniffi.tonearm_ffi.ImportProgress
import uniffi.tonearm_ffi.ImportStage
import uniffi.tonearm_ffi.isSpotifyPlaylistLink

/** iOS SpotifyImportSheet: copies a public Spotify playlist into a new TIDAL playlist. */
@Composable
fun SpotifyImportSheet(onDismiss: () -> Unit) {
    val colors = SdatTheme.colors
    var link by rememberSaveable { mutableStateOf("") }
    val start = { if (isSpotifyPlaylistLink(link)) SpotifyImporter.start(link) }
    BottomSheet(onDismissRequest = onDismiss, title = "Import from Spotify") {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).navigationBarsPadding()) {
            when (val state = SpotifyImporter.state) {
                State.Idle, is State.Failed -> {
                    GroupedList {
                        TextField(
                            value = link,
                            onValueChange = { link = it },
                            placeholder = "Spotify playlist link",
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Go),
                            keyboardActions = KeyboardActions(onGo = { start() }),
                        )
                        ListRow("Import", titleColor = colors.accent, enabled = isSpotifyPlaylistLink(link), showDisclosure = false, onClick = start)
                        if (state is State.Failed) ListRow(state.message, titleColor = colors.destructive)
                    }
                    GroupedFooter("The playlist must be public. In Spotify, use Share, then Copy link to playlist. SDAT creates a TIDAL playlist with the tracks TIDAL has.")
                }
                is State.Running -> GroupedList {
                    ListRow(
                        state.progress.label(),
                        showTopSeparator = false,
                        trailing = { CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp) },
                    )
                    // Saving always finishes, so that a cancel never leaves half a playlist.
                    ListRow("Cancel", titleColor = colors.destructive, enabled = state.progress?.stage != ImportStage.SAVING, showDisclosure = false, onClick = SpotifyImporter::cancel)
                }
                is State.Done -> {
                    val report = state.report
                    GroupedList {
                        ListRow(report.name, detail = "${report.imported} tracks", showTopSeparator = false)
                        ListRow("Open playlist", titleColor = colors.accent, showDisclosure = false, onClick = {
                            onDismiss()
                            SpotifyImporter.reset()
                            Chrome.push(Route.Playlist(report.playlistId))
                        })
                        ListRow("Import another", titleColor = colors.accent, showDisclosure = false, onClick = { link = ""; SpotifyImporter.reset() })
                    }
                    if (report.skipped > 0u) GroupedFooter("${report.skipped} entries were skipped: local files, podcast episodes or tracks Spotify no longer has.")
                    if (report.missing.isNotEmpty()) {
                        SectionTitle("Not on TIDAL (${report.missing.size})")
                        GroupedList {
                            report.missing.forEachIndexed { i, track -> ListRow(track, showTopSeparator = i > 0) }
                        }
                    }
                }
            }
        }
    }
}

private fun ImportProgress?.label(): String {
    val stage = when (this?.stage) {
        null, ImportStage.READING -> "Reading from Spotify"
        ImportStage.MATCHING -> "Finding tracks on TIDAL"
        ImportStage.SAVING -> "Creating the playlist"
    }
    return if (this == null || total == 0u) "$stage…" else "$stage… $done of $total"
}
