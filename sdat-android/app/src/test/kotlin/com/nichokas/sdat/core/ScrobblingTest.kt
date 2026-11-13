package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.ScrobbleTrack
import uniffi.tonearm_ffi.Track

class ScrobblingTest {
    private fun track(secs: UInt) = Track(
        id = "42", title = "Song", version = "Live", durationSecs = secs, explicit = false, streamable = true,
        artists = listOf(ArtistRef("1", "First", null), ArtistRef("2", "Second", null)), album = AlbumRef("a", "Album", null, null),
    )

    // tonearm-core's should_scrobble: half the track or 4 minutes, never 30 s or less.
    private fun listen(secs: UInt) = Listen(track(secs), startedAt = 0) { listened, duration ->
        duration > 30u && (listened * 2u >= duration.toULong() || listened >= 240u)
    }

    /** Plays [secs] seconds in 0.25 s ticks from [from]; returns how many ticks made the play count. */
    private fun Listen.play(from: Double, secs: Double, playing: Boolean = true): Int {
        var at = from
        var counted = 0
        repeat((secs / 0.25).toInt()) {
            if (heard(at, at + 0.25, playing)) counted++
            at += 0.25
        }
        return counted
    }

    @Test fun countsOnceAtHalfTheTrack() {
        val l = listen(200u)
        assertEquals(0, l.play(0.0, 99.75))
        assertEquals(1, l.play(99.75, 0.25))
        assertEquals(0, l.play(100.0, 100.0))
    }

    @Test fun countsAfterFourMinutesOfALongTrack() {
        val l = listen(3600u)
        assertEquals(0, l.play(0.0, 239.75))
        assertEquals(1, l.play(239.75, 0.25))
    }

    @Test fun seeksAndPausesAddNothing() {
        val l = listen(200u)
        assertFalse(l.heard(0.0, 150.0, playing = true))
        assertFalse(l.heard(150.0, 10.0, playing = true))
        assertEquals(0, l.play(10.0, 200.0, playing = false))
        assertEquals(0, l.play(10.0, 99.0))
        assertTrue(l.heard(109.0, 110.5, playing = true))
    }

    @Test fun shortTracksNeverCount() {
        assertEquals(0, listen(30u).play(0.0, 30.0))
    }

    @Test fun sendsTheFirstArtistLikeIos() {
        assertEquals(
            ScrobbleTrack("Song (Live)", "First", "Album", "https://tidal.com/track/42", 200u),
            scrobbleTrack(track(200u)),
        )
    }
}
