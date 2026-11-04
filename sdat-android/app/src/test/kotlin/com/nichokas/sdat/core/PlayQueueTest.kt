package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlayQueueTest {
    private fun queue() = PlayQueue<String> { it }

    @Test fun discoverStartsAtTheTapAndWrapsThePool() {
        val q = queue()
        q.discover("c", listOf("a", "b", "c", "d"))
        assertEquals(listOf("c", "d", "a", "b"), q.tracks)
        assertEquals(0, q.index)
    }

    @Test fun discoverPlaysEachTrackOnce() {
        val q = queue()
        q.discover("b", listOf("a", "b", "a", "c", "b"))
        assertEquals(listOf("b", "a", "c"), q.tracks)
    }

    // Same as iOS's `firstIndex ?? 0`: the pool's first track is skipped too.
    @Test fun discoverOutsideThePoolFallsBackToItsFirstIndex() {
        val q = queue()
        q.discover("x", listOf("a", "b"))
        assertEquals(listOf("x", "b"), q.tracks)
    }

    @Test fun appendSkipsQueuedAndRepeatedTracks() {
        val q = queue()
        q.discover("a", listOf("a", "b"))
        assertTrue(q.append(listOf("b", "c", "c", "a", "d")))
        assertEquals(listOf("a", "b", "c", "d"), q.tracks)
        assertFalse(q.append(listOf("a", "d")))
    }

    @Test fun appendRemembersTracksTrimmedFromHistory() {
        val q = queue()
        q.discover("t0", emptyList())
        q.append((1..400).map { "t$it" })
        q.index = 350
        q.append(listOf("t401"))
        assertEquals(PlayQueue.MAX_PLAYED_HISTORY, q.index)
        assertEquals("t350", q.current)
        assertFalse(q.append(listOf("t0", "t10")))
    }

    @Test fun refillsWithFiveOrFewerLeft() {
        val q = queue()
        q.discover("a", (0 until 10).map { "$it" })
        q.index = 4
        assertFalse(q.needsRefill)
        q.index = 5
        assertTrue(q.needsRefill)
    }
}
