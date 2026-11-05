package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
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
}
