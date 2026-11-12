package com.nichokas.sdat.core

import kotlin.random.Random
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PlayQueueTest {
    private fun queue() = PlayQueue<String>(Random(7)) { it }
    private val abcd = listOf("a", "b", "c", "d")

    @Test fun discoverStartsAtTheTapAndWrapsThePool() {
        val q = queue()
        assertEquals("c", q.discover("c", abcd))
        assertEquals(listOf("c", "d", "a", "b"), q.tracks)
        assertEquals(0, q.index)
        assertEquals("c", q.current)
        assertEquals(PlayQueue.DISCOVER, q.source)
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
        q.select(350)
        q.append(listOf("t401"))
        assertEquals(PlayQueue.MAX_PLAYED_HISTORY, q.index)
        assertEquals("t350", q.current)
        assertFalse(q.append(listOf("t0", "t10")))
    }

    @Test fun refillsWithFiveOrFewerLeft() {
        val q = queue()
        q.discover("a", (0 until 10).map { "$it" })
        q.select(4)
        assertFalse(q.needsRefill)
        q.select(5)
        assertTrue(q.needsRefill)
    }

    @Test fun playStartsAtTheTappedTrack() {
        val q = queue()
        assertEquals("c", q.play(abcd, startAt = 2, from = "Album"))
        assertEquals(abcd, q.tracks)
        assertEquals(listOf("d"), q.upNext)
        assertEquals("Album", q.source)
    }

    @Test fun playOutsideTheListChangesNothing() {
        val q = queue()
        q.play(abcd)
        assertNull(q.play(abcd, startAt = 4))
        assertNull(q.play(emptyList()))
        assertEquals("a", q.current)
    }

    @Test fun shuffledPlaysEveryTrackOnceAndStaysOn() {
        val q = queue()
        val first = q.playShuffled(abcd)
        assertTrue(q.shuffle)
        assertEquals(first, q.tracks[0])
        assertEquals(abcd.toSet(), q.tracks.toSet())
        assertEquals(4, q.tracks.size)
        assertEquals("b", q.play(abcd, startAt = 1))
        assertEquals("b", q.tracks[0])
        assertEquals(abcd.toSet(), q.tracks.toSet())
    }

    @Test fun shuffleOffRestoresTheOrderAtTheCurrentTrack() {
        val q = queue()
        q.play(abcd, startAt = 1)
        q.setShuffle(true, reorder = true)
        assertEquals(listOf("a", "b"), q.tracks.take(2))
        assertEquals(setOf("c", "d"), q.upNext.toSet())
        q.advance(manual = true)
        val now = q.current!!
        q.setShuffle(false, reorder = true)
        assertEquals(abcd, q.tracks)
        assertEquals(abcd.indexOf(now), q.index)
    }

    @Test fun shuffleLeavesAnEndlessQueueInOrder() {
        val q = queue()
        q.startInfinite(abcd)
        q.setShuffle(true, reorder = false)
        assertTrue(q.shuffle)
        assertEquals(abcd, q.tracks)
    }

    @Test fun infiniteRefillKeepsGoingPastTheFirstBatch() {
        val q = queue()
        assertEquals("a", q.startInfinite(abcd))
        assertEquals(PlayQueue.DISCOVER, q.source)
        repeat(3) { q.advance(manual = false) }
        assertNull(q.advance(manual = false))
        assertTrue(q.append(listOf("c", "e")))
        assertEquals("e", q.advance(manual = false))
        assertNull(q.startInfinite(emptyList()))
    }

    @Test fun playNextGoesBeforeQueuedTracksAndTheBaseQueue() {
        val q = queue()
        q.play(abcd)
        q.addToQueue("x")
        q.addToQueue("y")
        q.playNext("z")
        assertEquals(listOf("z", "x", "y"), q.userQueue)
        assertEquals("z", q.advance(manual = true))
        assertEquals(0, q.index)
        assertEquals("x", q.advance(manual = true))
        assertEquals("y", q.advance(manual = true))
        assertEquals("b", q.advance(manual = true))
        assertEquals(1, q.index)
    }

    @Test fun nextStopsAtTheEndAndWrapsOnRepeatAll() {
        val q = queue()
        q.play(abcd, startAt = 3)
        assertNull(q.advance(manual = true))
        assertEquals("d", q.current)
        q.cycleRepeat()
        assertEquals(Repeat.All, q.repeat)
        assertEquals("a", q.advance(manual = true))
        assertEquals(0, q.index)
    }

    @Test fun repeatOneLoopsOnlyOnTheNaturalEnd() {
        val q = queue()
        q.play(abcd)
        q.cycleRepeat()
        q.cycleRepeat()
        assertEquals(Repeat.One, q.repeat)
        assertEquals("a", q.advance(manual = false))
        assertEquals("b", q.advance(manual = true))
        q.cycleRepeat()
        assertEquals(Repeat.Off, q.repeat)
    }

    @Test fun previousSelectsOnlyInsideTheQueue() {
        val q = queue()
        q.play(abcd, startAt = 1)
        assertEquals("a", q.select(0))
        assertNull(q.select(-1))
        assertEquals("a", q.current)
    }

    @Test fun jumpsAndRemovals() {
        val q = queue()
        q.play(abcd)
        q.addToQueue("x")
        q.addToQueue("y")
        q.addToQueue("z")
        assertEquals("y", q.jumpToUser(1))
        assertEquals(listOf("z"), q.userQueue)
        assertEquals(0, q.index)
        q.removeUser(0)
        assertEquals(emptyList<String>(), q.userQueue)
        q.removeUpNext(0)
        assertEquals(listOf("c", "d"), q.upNext)
        assertEquals("d", q.jumpToUpNext(1))
        assertEquals(2, q.index)
    }

    @Test fun clearKeepsShuffleAndRepeat() {
        val q = queue()
        q.playShuffled(abcd)
        q.cycleRepeat()
        q.addToQueue("x")
        q.clear()
        assertNull(q.current)
        assertEquals(emptyList<String>(), q.tracks + q.userQueue)
        assertTrue(q.shuffle)
        assertEquals(Repeat.All, q.repeat)
    }
}
