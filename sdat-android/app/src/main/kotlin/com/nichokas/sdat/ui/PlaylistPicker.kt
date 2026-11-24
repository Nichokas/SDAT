package com.nichokas.sdat.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.OwnPlaylists
import com.nichokas.sdat.core.friendlyMessage
import com.nichokas.sdat.ui.kit.BottomSheet
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.addToPlaylist

/** iOS PlaylistPicker: the user's own playlists, freshly loaded; picking one adds [track] to it. */
@Composable
fun PlaylistPicker(track: Track, onDismiss: () -> Unit) {
    var loaded by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        OwnPlaylists.load(force = true)
        loaded = true
    }
    BottomSheet(onDismissRequest = onDismiss, title = "Add to Playlist") {
        val list = OwnPlaylists.list
        when {
            list.isNotEmpty() -> LazyColumn(Modifier.fillMaxWidth()) {
                items(list, key = { it.id }) { p ->
                    Row(
                        Modifier.fillMaxWidth()
                            .clickable(role = Role.Button) {
                                onDismiss()
                                Chrome.actionScope.launch {
                                    try {
                                        addToPlaylist(p.id, track.id)
                                        Chrome.flash("Added to ${p.name}")
                                    } catch (e: CancellationException) {
                                        throw e
                                    } catch (e: Exception) {
                                        Chrome.flash(friendlyMessage(e))
                                    }
                                }
                            }
                            .padding(horizontal = 20.dp, vertical = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(p.name, style = SdatType.body, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                        Text("${p.trackCount}", style = SdatType.body, color = SdatTheme.colors.secondaryContent, modifier = Modifier.padding(start = 12.dp))
                    }
                    HorizontalDivider(Modifier.padding(start = 20.dp), color = SdatTheme.colors.separator)
                }
            }
            loaded -> Unavailable("music.note.list", "No playlists")
            else -> Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                CircularProgressIndicator(color = SdatTheme.colors.secondaryContent)
            }
        }
    }
}
