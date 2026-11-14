package com.nichokas.sdat.core

import androidx.media3.common.Player
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.Stream

class PlaybackTest {
    @Test fun retriesAfterFailureBeforeAnythingElse() {
        assertEquals(Tap.Retry, tapAction(unprepared = true, ended = true, playWhenReady = true))
    }

    @Test fun replaysFromTheStartOnceEnded() {
        assertEquals(Tap.Replay, tapAction(unprepared = false, ended = true, playWhenReady = false))
    }

    @Test fun pausesWhileBufferingAndResumesWhenPaused() {
        assertEquals(Tap.Pause, tapAction(unprepared = false, ended = false, playWhenReady = true))
        assertEquals(Tap.Resume, tapAction(unprepared = false, ended = false, playWhenReady = false))
    }

    @Test fun previousRestartsPastThreeSecondsOrOnTheFirstTrack() {
        assertTrue(restartsOnPrevious(position = 3.5, index = 2))
        assertTrue(restartsOnPrevious(position = 0.0, index = 0))
        assertFalse(restartsOnPrevious(position = 3.0, index = 2))
    }

    @Test fun progressIsPositionOverDurationCappedAtOne() {
        assertEquals(0.25, progress(30.0, 120.0), 1e-9)
        assertEquals(1.0, progress(130.0, 120.0), 1e-9)
        assertEquals(0.0, progress(30.0, 0.0), 1e-9)
    }

    @Test fun seekStaysInsideTheTrack() {
        assertEquals(0.0, clampSeek(-4.0, 120.0), 1e-9)
        assertEquals(120.0, clampSeek(130.0, 120.0), 1e-9)
        assertEquals(42.0, clampSeek(42.0, 120.0), 1e-9)
        assertEquals(42.0, clampSeek(42.0, 0.0), 1e-9)
    }

    @Test fun controllerSeeksToAnotherItemGoThroughTheQueue() {
        assertEquals(Seek.Next, seekAction(Player.COMMAND_SEEK_TO_NEXT, 0, 0))
        assertEquals(Seek.Previous, seekAction(Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM, 0, 0))
        assertEquals(Seek.InCurrent, seekAction(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM, 0, 0))
        assertEquals(Seek.InCurrent, seekAction(Player.COMMAND_SEEK_TO_DEFAULT_POSITION, 0, 0))
        assertEquals(Seek.Next, seekAction(Player.COMMAND_SEEK_TO_MEDIA_ITEM, 1, 0))
        assertEquals(Seek.Ignore, seekAction(Player.COMMAND_SEEK_TO_MEDIA_ITEM, 3, 0))
    }

    @Test fun badgeShowsTheDeliveredQuality() {
        fun stream(asked: Quality, vararg formats: String) = Stream("u", asked, formats.toList(), null, null)
        assertEquals(Quality.HI_FI, stream(Quality.CD, "FLAC", "FLAC_HIRES").actualQuality)
        assertEquals(Quality.CD, stream(Quality.HI_FI, "FLAC").actualQuality)
        assertEquals(Quality.AAC, stream(Quality.HI_FI, "AACLC").actualQuality)
        assertEquals(Quality.CD, stream(Quality.CD).actualQuality)
        assertEquals("Hi-Res Lossless", Quality.HI_FI.badgeText)
    }
}
