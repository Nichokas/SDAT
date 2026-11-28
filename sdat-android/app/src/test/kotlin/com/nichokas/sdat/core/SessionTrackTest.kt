package com.nichokas.sdat.core

import android.app.Application
import androidx.annotation.OptIn
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.Timeline
import androidx.media3.common.util.UnstableApi
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.tonearm_ffi.Track

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
@OptIn(UnstableApi::class)
class SessionTrackTest {
    @Test fun queuedTrackDoesNotInheritTheStoppedStreamsPositionOrDuration() {
        val track = Track(
            id = "b", title = "Next song", version = "", durationSecs = 180u, explicit = false, streamable = true,
            artists = emptyList(), album = null,
        )
        val stopped = SimpleBasePlayer.MediaItemData.Builder("a")
            .setMediaItem(
                MediaItem.Builder().setMediaId("a")
                    .setMediaMetadata(MediaMetadata.Builder().setTitle(track.title).build()).build(),
            )
            .setDurationUs(240_000_000)
            .setIsSeekable(true)
            .build()
        val state = SimpleBasePlayer.State.Builder()
            .setAvailableCommands(Player.Commands.EMPTY)
            .setPlaylist(listOf(stopped))
            .setContentPositionMs(190_000)
            .setContentBufferedPositionMs(SimpleBasePlayer.PositionSupplier.getConstant(240_000))
            .setTotalBufferedDurationMs(SimpleBasePlayer.PositionSupplier.getConstant(50_000))
            .build()

        val pending = state.buildUpon().apply { showUnloadedTrack(track, position = 12.5, duration = 180.0) }.build()
        val window = pending.timeline.getWindow(0, Timeline.Window())

        assertEquals("b", window.mediaItem.mediaId)
        assertEquals("Next song", pending.currentMetadata.title)
        assertEquals(180_000L, window.durationMs)
        assertEquals(12_500L, pending.contentPositionMsSupplier.get())
        assertEquals(12_500L, pending.contentBufferedPositionMsSupplier.get())
        assertEquals(0L, pending.totalBufferedDurationMsSupplier.get())
    }
}
