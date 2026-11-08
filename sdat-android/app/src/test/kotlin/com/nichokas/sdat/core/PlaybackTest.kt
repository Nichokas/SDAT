package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlaybackTest {
    @Test fun retriesAfterFailureBeforeAnythingElse() {
        assertEquals(Tap.Retry, tapAction(failed = true, ended = true, playWhenReady = true))
    }

    @Test fun replaysFromTheStartOnceEnded() {
        assertEquals(Tap.Replay, tapAction(failed = false, ended = true, playWhenReady = false))
    }

    @Test fun pausesWhileBufferingAndResumesWhenPaused() {
        assertEquals(Tap.Pause, tapAction(failed = false, ended = false, playWhenReady = true))
        assertEquals(Tap.Resume, tapAction(failed = false, ended = false, playWhenReady = false))
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
}
