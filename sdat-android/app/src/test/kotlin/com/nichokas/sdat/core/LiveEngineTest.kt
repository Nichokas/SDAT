package com.nichokas.sdat.core

import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.Track

class LiveEngineTest {
    @Test fun guestParksAtTheHostPositionIfTheHostPausesBeforeAlignment() = runBlocking {
        val port = TestPort()
        val engine = LiveEngine(port, this) { 10L }
        engine.liveFollowing = true
        engine.liveTarget = { HostAt(4_000, true) }
        engine.prepareAlignment(engine.shouldAlign(true))
        engine.liveTarget = { HostAt(4_200, false) }

        engine.startAlignment()
        yield()

        assertEquals(4_200L, port.positionMs)
        assertFalse(port.requested)
        assertFalse(engine.aligning)
        assertEquals(10L, engine.liveSettledAt)
    }

    @Test fun hostUsesTheRequestedPositionWhileTheNewItemLoads() = runBlocking {
        val port = TestPort()
        val engine = LiveEngine(port, this) { 0L }
        engine.loadingFrom(3_000)

        assertEquals(3_000L, engine.livePublishedPositionMs)
        assertFalse(engine.livePublishedPlaying)

        engine.clearLoadPosition()
        assertEquals(port.logicalPositionMs, engine.livePublishedPositionMs)
        assertTrue(engine.livePublishedPlaying)
    }

    private class TestPort : LivePlaybackPort {
        override val currentId = "new"
        override val engineId = "old"
        override var requested = true
        override val advancing = false
        private var playing = true
        override val logicalPlaying get() = playing
        override val loading = false
        override val fetching = false
        override val hasError = false
        override val suppressed = false
        override val idle = false
        override val ended = false
        override val ready = true
        override val failed = false
        override var positionMs = 12_000L
        override var logicalPositionMs = 2_000L
        override val durationMs = 180_000L
        override val bufferedMs = 20_000L

        override fun playEngine() { requested = true }
        override fun pauseEngine() { requested = false }
        override fun seekEngine(ms: Long) { positionMs = ms }
        override fun setLogicalPosition(ms: Long) { logicalPositionMs = ms }
        override fun setLogicalPlaying(playing: Boolean) { this.playing = playing }
        override fun requestPause() = pauseEngine()
        override fun seek(ms: Long) = seekEngine(ms)
        override fun toggle() { requested = !requested }
        override fun load(track: Track, atMs: Long, playing: Boolean) = Unit
        override fun followingChanged() = Unit
    }
}
