package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.HandoffDevice
import uniffi.tonearm_ffi.HandoffElsewhere
import uniffi.tonearm_ffi.HandoffLocal
import uniffi.tonearm_ffi.HandoffRepeat
import uniffi.tonearm_ffi.Track

class HandoffTest {
    private val track = Track("1", "Song", "", 200u, false, true, emptyList(), AlbumRef("x", "Album", null, null))
    private val now = 1_700_000_000_000L

    private fun elsewhere(name: String, playing: Boolean, updatedAtMs: Long) =
        HandoffElsewhere(HandoffDevice("d", name), track, 30_000, playing, updatedAtMs)

    @Test fun reportsPlayNextBeforeTheRestOfTheQueue() {
        assertEquals(
            HandoffLocal("a", listOf("x", "y", "b", "c"), 12_000, true, HandoffRepeat.ALL),
            handoffLocal("a", listOf("x", "y"), listOf("b", "c"), 12_000, playing = true, Repeat.All),
        )
    }

    @Test fun nothingLoadedHasNoFuture() {
        assertEquals(HandoffLocal(null, emptyList(), 0, false, HandoffRepeat.OFF), handoffLocal(null, listOf("x"), listOf("b"), 0, playing = false, Repeat.Off))
    }

    @Test fun repeatRoundTrips() {
        Repeat.entries.forEach { assertEquals(it, it.handoff.local) }
    }

    @Test fun resumedQueueIsTheCurrentTrackThenWhatFollowsInOrder() {
        val q = PlayQueue<String>(kotlin.random.Random(1)) { it }
        q.playShuffled(listOf("p", "q", "r"))
        q.playNext(listOf("x"))
        val (saved, state) = resumed("a", 42.5, Repeat.One)
        q.restore(saved, state)
        assertEquals("a", q.current)
        assertEquals(emptyList<String>(), q.upNext)
        assertEquals(emptyList<String>(), q.userQueue)
        assertEquals(Repeat.One, q.repeat)
        assertEquals(false, q.shuffle)
        assertEquals(42.5, state.position, 0.0)
        q.extend(listOf("b", "a", "c"))
        assertEquals("a", q.current)
        assertEquals(listOf("b", "a", "c"), q.upNext)
    }

    @Test fun upcomingFollowsOnlyTheResumedTrackAlone() {
        assertTrue(upcomingApplies("a", "a", listOf("a")))
        assertFalse(upcomingApplies("a", "b", listOf("a")))
        assertFalse(upcomingApplies("a", null, listOf("a")))
        // Already appended.
        assertFalse(upcomingApplies("a", "a", listOf("a", "b")))
        // A list started at that track meanwhile.
        assertFalse(upcomingApplies("a", "a", listOf("x", "a")))
    }

    @Test fun bannerLine() {
        assertEquals("Playing on iPhone", elsewhereLine(elsewhere("iPhone", playing = true, now - 7_200_000), now))
        assertEquals("Paused on iPhone · 2 h ago", elsewhereLine(elsewhere("iPhone", playing = false, now - 7_200_000), now))
        assertEquals("Paused on another device", elsewhereLine(elsewhere("", playing = false, 0), now))
    }

    @Test fun ago() {
        assertEquals("just now", Fmt.ago(now - 59_000, now))
        assertEquals("just now", Fmt.ago(now + 5_000, now))
        assertEquals("5 min ago", Fmt.ago(now - 5 * 60_000, now))
        assertEquals("23 h ago", Fmt.ago(now - 23 * 3_600_000 - 59 * 60_000, now))
        assertEquals("3 d ago", Fmt.ago(now - 3 * 86_400_000, now))
    }
}
