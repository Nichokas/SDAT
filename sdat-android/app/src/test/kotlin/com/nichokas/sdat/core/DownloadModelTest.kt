package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Track

class DownloadModelTest {
    private fun track(id: String, album: String = "al", streamable: Boolean = true) = Track(
        id, "Song $id", "", 180u, false, streamable,
        listOf(ArtistRef("a", "Artist", null)),
        AlbumRef(album, "Album", "https://img/80x80.jpg", "https://img/1280x1280.jpg"),
        null,
    )

    private fun entry(id: String, album: String = "al", trackNumber: Int? = null, date: Long = 0) =
        DownloadEntry(track(id, album), "$id.mp4", "cover-$album.jpg", "hifi", 1_000, date, trackNumber = trackNumber)

    @Test fun indexRoundTrip() {
        val full = DownloadEntry(
            track = track("1"),
            file = "1.mp4",
            cover = "cover-al.jpg",
            quality = "hifi",
            size = 41_234_567,
            date = 1_763_000_000_000,
            gainDb = -7.5,
            peak = 0.98,
            albumGainDb = -6.25,
            albumPeak = 1.0,
            formats = listOf("FLAC", "FLAC_HIRES"),
            codec = "flac",
            bandwidth = 2_500_000,
            sampleRate = 96_000,
            bitDepth = 24,
            albumArtist = "Artist",
            albumYear = "2021",
            trackNumber = 3,
        )
        val bare = DownloadEntry(track("2").copy(album = null, bpm = 120.0), "2.mp4", null, "aac", 5, 7)
        val index = mapOf("1" to full, "2" to bare)
        assertEquals(index, DownloadIndex.decode(DownloadIndex.encode(index)))
    }

    @Test fun indexDropsOnlyMalformedEntriesAndRejectsOtherFormats() {
        val text = DownloadIndex.encode(mapOf("1" to entry("1")))
            .replace("\"entries\":{", "\"entries\":{\"bad\":{\"file\":\"x.mp4\"},")
        assertEquals(setOf("1"), DownloadIndex.decode(text).keys)
        assertEquals(emptyMap<String, DownloadEntry>(), DownloadIndex.decode("""{"format":2,"entries":{}}"""))
        assertEquals(emptyMap<String, DownloadEntry>(), DownloadIndex.decode("not json"))
    }

    @Test fun downloadRunsQueuedThenDownloadingThenDone() {
        var downloaded = emptySet<String>()
        var q = DownloadQueue().enqueue(listOf(track("1"), track("2")), downloaded::contains)
        assertEquals(DownloadState.Queued, q.state("1", false))
        q = q.startNext()
        assertEquals(DownloadState.Downloading(0.0), q.state("1", false))
        assertEquals(DownloadState.Queued, q.state("2", false))
        q = q.progress("1", 0.5)
        assertEquals(DownloadState.Downloading(0.5), q.state("1", false))
        assertEquals(0.25, q.batchProgress, 1e-9)
        q = q.finish("1", null)
        downloaded = setOf("1")
        assertEquals(DownloadState.Done, q.state("1", "1" in downloaded))
        assertEquals(1, q.batchDone)
        assertEquals(0.5, q.batchProgress, 1e-9)
        q = q.startNext().finish("2", "Can't reach TIDAL.")
        assertEquals(DownloadState.Failed("Can't reach TIDAL."), q.state("2", false))
        assertFalse(q.isActive)
        assertEquals("an empty queue starts the next batch from zero", 0, q.batchTotal)
    }

    @Test fun enqueueSkipsSavedQueuedAndUnstreamableAndRetryClearsFailure() {
        var q = DownloadQueue(failures = mapOf("4" to "old"))
        q = q.enqueue(listOf(track("1"), track("2"), track("3", streamable = false), track("4")), setOf("2")::contains)
        q = q.enqueue(listOf(track("1")), { false })
        assertEquals(listOf("1", "4"), q.active.map { it.id })
        assertEquals(2, q.batchTotal)
        assertNull(q.failures["4"])
        assertTrue(q.enqueue(emptyList(), { false }) === q)
    }

    @Test fun cancellingTheCurrentTrackIgnoresItsLateResult() {
        var q = DownloadQueue().enqueue(listOf(track("1"), track("2")), { false }).startNext()
        q = q.cancel("1")
        assertEquals(DownloadState.None, q.state("1", false))
        assertEquals(1, q.batchTotal)
        val afterLateFailure = q.finish("1", "boom")
        assertEquals(q, afterLateFailure)
        assertEquals(DownloadState.Downloading(0.0), q.startNext().state("2", false))
        assertEquals(DownloadQueue(failures = mapOf("9" to "x")), DownloadQueue(failures = mapOf("9" to "x")).enqueue(listOf(track("1")), { false }).cancelAll())
    }

    @Test fun offlineAlbumKeepsAlbumOrderThenSaveOrder() {
        val entries = listOf(
            entry("late", date = 5),
            entry("third", trackNumber = 3),
            entry("other", album = "x", trackNumber = 1),
            entry("first", trackNumber = 1),
            entry("early", date = 1),
        )
        assertEquals(listOf("first", "third", "early", "late"), offlineAlbum(entries, "al").map { it.id })
        assertEquals(emptyList<Track>(), offlineAlbum(entries, "none"))
    }

    @Test fun qualityText() {
        val e = entry("1")
        assertEquals("HI-RES FLAC 24-bit/96 kHz", e.copy(codec = "flac", bitDepth = 24, sampleRate = 96_000).qualityText)
        assertEquals("FLAC 16-bit/44.1 kHz", e.copy(formats = listOf("FLAC"), bitDepth = 16, sampleRate = 44_100).qualityText)
        assertEquals("AAC 320 kbps · 44.1 kHz", e.copy(codec = "mp4a.40.2", bandwidth = 320_000, sampleRate = 44_100).qualityText)
        assertNull(e.qualityText)
    }
}
