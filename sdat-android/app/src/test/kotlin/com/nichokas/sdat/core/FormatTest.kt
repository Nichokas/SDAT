package com.nichokas.sdat.core

import java.io.IOException
import java.time.LocalDate
import java.time.ZoneOffset
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.Track

class FormatTest {
    private fun track(version: String) = Track(
        "1", "Song", version, 200u, false, true,
        listOf(ArtistRef("a", "A", null), ArtistRef("b", "B", null)),
        AlbumRef("x", "Album", "small", null),
    )

    @Test fun trackLines() {
        assertEquals("Song", track("").displayTitle)
        assertEquals("Song (Live)", track("Live").displayTitle)
        assertEquals("A, B", track("").artistLine)
        assertEquals("small", track("").coverUrlLarge)
    }

    @Test fun time() {
        assertEquals("3:05", Fmt.time(185.9))
        assertEquals("0:00", Fmt.time(-1.0))
        assertEquals("0:00", Fmt.time(Double.NaN))
    }

    @Test fun summary() {
        assertEquals("1 song · 3 min", Fmt.summary(1u, 200u))
        assertEquals("12 songs · 1 h 5 min", Fmt.summary(12u, 3900u))
    }

    @Test fun updated() {
        val today = LocalDate.of(2026, 11, 4)
        val noon = today.atTime(12, 0).toEpochSecond(ZoneOffset.UTC)
        assertEquals(null, Fmt.updated(null, ZoneOffset.UTC, today))
        assertEquals("Updated today", Fmt.updated(noon, ZoneOffset.UTC, today))
        assertEquals("Updated yesterday", Fmt.updated(noon - 86_400, ZoneOffset.UTC, today))
    }

    @Test fun friendlyMessages() {
        assertEquals("TIDAL refused the request. Try signing in again.", friendlyMessage(FfiException.Http(401u, "")))
        assertEquals("TIDAL is having trouble right now. Try again later.", friendlyMessage(FfiException.Http(503u, "")))
        assertEquals("TIDAL couldn't handle this request (418).", friendlyMessage(FfiException.Http(418u, "")))
        assertEquals("Last.fm: bad", friendlyMessage(FfiException.LastFm(4, "bad")))
        assertEquals("Can't reach TIDAL. Check your connection.", friendlyMessage(IOException()))
        assertEquals("Something went wrong. Please try again.", friendlyMessage(FfiException.Other("x")))
    }
}
