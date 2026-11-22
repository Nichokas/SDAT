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

    @Test fun refetchesTheFirstFailureOfEachTrack() {
        assertTrue(refetchDue(null, "a", now = 5_000))
        assertTrue(refetchDue("b" to 5_000L, "a", now = 5_000))
    }

    @Test fun showsAFailureRightAfterARefetchOfTheSameTrack() {
        assertFalse(refetchDue("a" to 5_000L, "a", now = 5_000 + RETRY_WINDOW_MS - 1))
    }

    @Test fun refetchesTheSameTrackAgainOnceTheWindowPassed() {
        assertTrue(refetchDue("a" to 5_000L, "a", now = 5_000 + RETRY_WINDOW_MS))
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

    @Test fun sessionCommandsStripUnguardedExoPlayerMutations() {
        val blocked = setOf(
            Player.COMMAND_SET_REPEAT_MODE,
            Player.COMMAND_SET_SHUFFLE_MODE,
            Player.COMMAND_SET_SPEED_AND_PITCH,
            Player.COMMAND_SET_MEDIA_ITEM,
            Player.COMMAND_CHANGE_MEDIA_ITEMS,
            Player.COMMAND_RELEASE,
        )
        val kept = setOf(
            Player.COMMAND_PLAY_PAUSE,
            Player.COMMAND_PREPARE,
            Player.COMMAND_STOP,
            Player.COMMAND_GET_CURRENT_MEDIA_ITEM,
        )
        val base = kept + blocked

        val loaded = sessionCommands(base, hasTrack = true, emptyTimeline = false)
        assertTrue(loaded.containsAll(kept))
        assertTrue(Player.COMMAND_SEEK_TO_NEXT in loaded)
        assertTrue(Player.COMMAND_SEEK_TO_PREVIOUS in loaded)
        assertFalse(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM in loaded)
        blocked.forEach { assertFalse(it in loaded) }

        val restored = sessionCommands(base, hasTrack = true, emptyTimeline = true)
        assertTrue(restored.containsAll(kept))
        assertTrue(Player.COMMAND_SEEK_TO_NEXT in restored)
        assertTrue(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM in restored)
        blocked.forEach { assertFalse(it in restored) }

        val empty = sessionCommands(base, hasTrack = false, emptyTimeline = true)
        assertEquals(kept, empty)
        blocked.forEach { assertFalse(it in empty) }
    }

    @Test fun nonDiscoverHandoffWaitsForUpcomingUnderRepeatOffAndRepeatAll() {
        assertEquals(
            Advance.Wait,
            advanceAction(auto = true, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = true, repeat = Repeat.All, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.All, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Queue,
            advanceAction(auto = true, repeat = Repeat.One, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.One, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Queue,
            advanceAction(auto = true, repeat = Repeat.All, hasNext = true, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
    }

    @Test fun advanceStopsWrapsOrRefillsOnceUpcomingIsSettled() {
        assertEquals(
            Advance.Stop,
            advanceAction(auto = true, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Queue,
            advanceAction(auto = true, repeat = Repeat.All, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = true, repeat = Repeat.Off, hasNext = false, hasInfinite = true, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = true, awaitingUpNext = false),
        )
    }
}
