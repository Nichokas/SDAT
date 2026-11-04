package com.nichokas.sdat.ui

import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.Track

class CoverPoolTest {
    private fun track(id: String, album: String?, cover: String? = "c-$album") = Track(
        id = id, title = id, version = "", durationSecs = 0u, explicit = false, streamable = true,
        artists = emptyList(), album = album?.let { AlbumRef(it, it, cover, null) },
    )

    @Test fun skipsTracksWithoutCover() {
        val pool = CoverPool()
        pool.add(listOf(track("1", null), track("2", "a", cover = null), track("3", "b")))
        assertEquals(listOf("3"), pool.tracks.map { it.id })
    }

    @Test fun keepsOneTilePerTrackAlbumAndCover() {
        val pool = CoverPool()
        pool.add(listOf(track("1", "a"), track("1", "b"), track("2", "a"), track("3", "c", cover = "c-a")))
        pool.add(listOf(track("4", "d"), track("4", "e")))
        assertEquals(listOf("1", "4"), pool.tracks.map { it.id })
    }

    @Test fun aRejectedTrackDoesNotReserveItsAlbum() {
        val pool = CoverPool()
        pool.add(listOf(track("1", "a"), track("2", "b", cover = "c-a"), track("3", "b")))
        assertEquals(listOf("1", "3"), pool.tracks.map { it.id })
    }
}
