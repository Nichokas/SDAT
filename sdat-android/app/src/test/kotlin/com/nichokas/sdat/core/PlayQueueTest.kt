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

    @Test fun appendSkipsPlayNextTracksWaitingOrPlaying() {
        val q = queue()
        q.discover("a", listOf("a", "b"))
        q.addToQueue("x")
        q.playNext("y")
        assertFalse(q.append(listOf("x", "y")))
        assertEquals("y", q.advance(manual = true))
        assertTrue(q.append(listOf("y", "z")))
        assertEquals(listOf("a", "b", "z"), q.tracks)
    }

    @Test fun appendSkipsPlayNextTracksAlreadyPlayed() {
        val q = queue()
        q.discover("a", listOf("a", "b"))
        q.addToQueue("x")
        assertEquals("x", q.advance(manual = true))
        assertEquals("b", q.advance(manual = true))
        assertFalse(q.append(listOf("x")))
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

    @Test fun previousFromAPlayNextTrackReturnsToTheBaseTrackItFollowed() {
        val q = queue()
        q.play(listOf("a", "b", "c"), startAt = 1)
        q.playNext("x")
        assertEquals("x", q.advance(manual = true))
        assertEquals("b", q.previous())
        assertEquals("a", q.previous())
        assertNull(q.previous())
        assertEquals("a", q.current)
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

    @Test fun aRemovedTrackStaysRemovedWhenShuffleTurnsOff() {
        val q = queue()
        q.play(abcd)
        q.removeUpNext(0)
        q.setShuffle(true, reorder = true)
        val gone = q.upNext.first()
        q.removeUpNext(0)
        q.setShuffle(false, reorder = true)
        assertEquals(listOf("a", "c", "d") - gone, q.tracks)
    }

    @Test fun shuffleOffReturnsToTheSameOccurrenceOfARepeatedTrack() {
        val q = queue()
        q.play(listOf("a", "b", "a", "c"), startAt = 2)
        q.setShuffle(true, reorder = true)
        q.setShuffle(false, reorder = true)
        assertEquals(2, q.index)
        assertEquals(listOf("c"), q.upNext)
    }

    @Test fun removingARepeatedTrackRemovesThatOccurrence() {
        val q = queue()
        q.setShuffle(true, reorder = true)
        q.play(listOf("x", "a", "x", "b"))
        q.removeUpNext(q.upNext.indexOf("x"))
        q.setShuffle(false, reorder = true)
        assertEquals(listOf("x", "a", "b"), q.tracks)
        assertEquals(0, q.index)
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

    @Test fun preloadsWhatANaturalEndPlaysWithoutTakingIt() {
        val q = queue()
        q.play(abcd, startAt = 2)
        assertEquals("d", q.preloadable)
        q.addToQueue("x")
        assertEquals("x", q.preloadable)
        assertEquals(listOf("x"), q.userQueue)
        assertEquals(q.preloadable, q.advance(manual = false))
        assertEquals("d", q.preloadable)
        assertEquals(q.preloadable, q.advance(manual = false))
    }

    @Test fun preloadsNothingAtTheEndOrOnRepeatOneAndWrapsOnRepeatAll() {
        val q = queue()
        q.play(abcd, startAt = 3)
        assertNull(q.preloadable)
        q.cycleRepeat()
        assertEquals("a", q.preloadable)
        q.cycleRepeat()
        assertNull(q.preloadable)
        assertEquals("d", q.advance(manual = false))
        assertNull(queue().preloadable)
    }

    @Test fun savedQueueRestoresTheSameQueues() {
        val q = queue()
        q.play(abcd, startAt = 1, from = "Album")
        q.setShuffle(true, reorder = true)
        q.cycleRepeat()
        q.addToQueue("x")
        val saved = q.saved(id = 3, endless = false)
        val restored = queue()
        restored.restore(saved, SavedState(3, q.current!!, q.index, 42.0, q.repeat, q.shuffle))
        assertEquals("b", restored.current)
        assertEquals(q.upNext, restored.upNext)
        assertEquals(listOf("x"), restored.userQueue)
        assertEquals("Album", restored.source)
        assertEquals(Repeat.All, restored.repeat)
        restored.setShuffle(false, reorder = true)
        assertEquals(abcd, restored.tracks)
        assertEquals(1, restored.index)
    }

    @Test fun aLongShuffledQueueSavesItsOrderAroundTheCurrentTrack() {
        val q = queue()
        val list = (0 until 3000).map { "t$it" }
        q.play(list, startAt = 2500)
        q.setShuffle(true, reorder = true)
        val saved = q.saved(id = 1, endless = false)
        val restored = queue()
        restored.restore(saved, SavedState(1, "t2500", q.index, 0.0, Repeat.Off, shuffle = true))
        restored.setShuffle(false, reorder = true)
        assertEquals("t2501", restored.upNext.first())
    }

    @Test fun everySavedShuffledTrackCanBeUnshuffled() {
        val q = queue()
        q.play((0 until 5000).map { "t$it" })
        q.setShuffle(true, reorder = true)
        val saved = q.saved(id = 1, endless = false)
        val restored = queue()
        restored.restore(saved, SavedState(1, "t0", 0, 0.0, Repeat.Off, shuffle = true))
        restored.jumpToUpNext(1500)
        val now = restored.current
        restored.setShuffle(false, reorder = true)
        assertEquals(now, restored.tracks[restored.index!!])
        assertEquals(restored.tracks.sortedBy { it.drop(1).toInt() }, restored.tracks)
    }

    @Test fun restoreWithAnIndexOutsideItsWindowKeepsJustTheTrack() {
        val q = queue()
        q.play(abcd)
        q.addToQueue("x")
        val saved = q.saved(id = 1, endless = false)
        val restored = queue()
        restored.restore(saved, SavedState(1, "c", 9, 0.0, Repeat.Off, shuffle = false))
        assertEquals(listOf("c"), restored.tracks)
        assertEquals(0, restored.index)
        assertEquals(emptyList<String>(), restored.userQueue)
    }

    @Test fun restoreWithoutItsQueueKeepsJustTheTrack() {
        val q = queue()
        q.restore(null, SavedState(1, "c", 7, 0.0, Repeat.Off, shuffle = false))
        assertEquals(listOf("c"), q.tracks)
        assertEquals(0, q.index)
        assertNull(q.advance(manual = false))
    }

    @Test fun savesAWindowAroundALongEndlessQueue() {
        val q = queue()
        q.discover("t0", emptyList())
        q.append((1..299).map { "t$it" })
        q.select(120)
        val saved = q.saved(id = 1, endless = true)
        assertEquals(70, saved.offset)
        assertEquals(listOf("t70", "t299"), listOf(saved.original.first(), saved.original.last()))
        assertEquals(saved.original.indices.toList(), saved.order)
        val restored = queue()
        restored.restore(saved, SavedState(1, "t120", 120, 0.0, Repeat.Off, shuffle = false))
        assertEquals("t121", restored.upNext.first())
        // The restored window is the dedupe history, as iOS seeds recentIDs from it.
        assertFalse(restored.append(listOf("t80")))
    }

    @Test fun windowMatchesIos() {
        assertEquals(0 until 10, savedWindow(10, 5, endless = false))
        assertEquals(0 until 2000, savedWindow(5000, 100, endless = false))
        assertEquals(500 until 2500, savedWindow(5000, 1000, endless = false))
        assertEquals(3000 until 5000, savedWindow(5000, 4900, endless = false))
        assertEquals(950 until 1201, savedWindow(5000, 1000, endless = true))
        assertEquals(0 until 0, savedWindow(0, null, endless = false))
    }
}
