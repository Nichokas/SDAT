package com.nichokas.sdat.core

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Track

class DiscoverCacheTest {
    @get:Rule val dir = TemporaryFolder()

    private val full = Track(
        id = "1", title = "Song", version = "(Remastered)", durationSecs = 4_000_000_000u, explicit = true, streamable = false,
        artists = listOf(ArtistRef("a1", "First", null), ArtistRef("a2", "Second", null)),
        album = AlbumRef("al", "Album", "https://c/80x80.jpg", "https://c/1280x1280.jpg"), bpm = 121.5,
    )
    private val bare = Track(
        id = "2", title = "Bare", version = "", durationSecs = 0u, explicit = false, streamable = true,
        artists = emptyList(), album = null,
    )

    private fun cache() = DiscoverCache(File(dir.root, "discover.json"))

    @Test fun roundTripsThroughTheFile() {
        val c = cache()
        c.save(listOf(full, bare, full.copy(id = "3", album = AlbumRef("al2", "Other", null, null), bpm = null)))
        assertEquals(listOf(full, bare, full.copy(id = "3", album = AlbumRef("al2", "Other", null, null), bpm = null)), c.load())
    }

    @Test fun aNewSaveReplacesTheOldPool() {
        val c = cache()
        c.save(listOf(full))
        c.save(listOf(bare))
        assertEquals(listOf(bare), c.load())
        assertEquals(listOf("discover.json"), dir.root.list()!!.toList())
    }

    @Test fun dropsArtistPicturesAsOnIos() {
        val c = cache()
        c.save(listOf(bare.copy(artists = listOf(ArtistRef("a", "A", "https://p/1.jpg")))))
        assertEquals(listOf(ArtistRef("a", "A", null)), c.load().single().artists)
    }

    @Test fun noFileIsEmpty() = assertEquals(emptyList<Track>(), cache().load())

    @Test fun clearedIsEmpty() {
        val c = cache()
        c.save(listOf(full))
        c.clear()
        assertEquals(emptyList<Track>(), c.load())
    }

    @Test fun corruptFileIsEmpty() {
        File(dir.root, "discover.json").writeText(DiscoverCache.encode(listOf(full)).dropLast(20))
        assertEquals(emptyList<Track>(), cache().load())
    }

    @Test fun malformedTrackIsEmpty() {
        File(dir.root, "discover.json").writeText("""{"format":1,"tracks":[{"id":"1"}]}""")
        assertEquals(emptyList<Track>(), cache().load())
    }

    @Test fun iosStyleBareArrayIsEmpty() {
        File(dir.root, "discover.json").writeText("""[{"id":"1","title":"t","version":"","dur":1,"explicit":false,"streamable":true,"artists":[]}]""")
        assertEquals(emptyList<Track>(), cache().load())
    }

    @Test fun otherFormatIsEmpty() {
        File(dir.root, "discover.json").writeText(DiscoverCache.encode(listOf(full)).replace("\"format\":1", "\"format\":2"))
        assertEquals(emptyList<Track>(), cache().load())
    }
}
