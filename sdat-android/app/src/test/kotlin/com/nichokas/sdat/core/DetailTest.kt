package com.nichokas.sdat.core

import java.io.IOException
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.tonearm_ffi.FavouriteKind

class DetailTest {
    @Test fun subtitleLeavesOutMissingParts() {
        assertEquals("Daft Punk · 2001", joinDot("Daft Punk", year("2001-03-12")))
        assertEquals("Daft Punk", joinDot("Daft Punk", year(null)))
        assertEquals("Playlist", joinDot(null, "Playlist"))
    }

    @Test fun mosaicNeedsFourDistinctCovers() {
        assertEquals(listOf("a", "b", "c", "d"), mosaicCovers(listOf("a", null, "a", "b", "c", "b", "d", "e")))
        assertEquals(emptyList<String>(), mosaicCovers(listOf("a", "b", "a", "c", null)))
    }

    @Test fun ownPlaylistsHaveNoFavourite() {
        assertNull(playlistFavourite(isMix = false, own = true))
        assertEquals(FavouriteKind.MIX, playlistFavourite(isMix = true, own = false))
        assertEquals(FavouriteKind.PLAYLIST, playlistFavourite(isMix = false, own = false))
    }

    private fun pages(vararg pages: List<Int>?): suspend () -> List<Int>? {
        val queue = pages.toMutableList()
        return { if (queue.isEmpty()) null else queue.removeAt(0) }
    }

    private fun drain(loaded: Int, cap: Int, next: suspend () -> List<Int>?): List<List<Int>> {
        val got = mutableListOf<List<Int>>()
        runBlocking { pageThrough(loaded, cap, next) { got += it } }
        return got
    }

    @Test fun pagingStopsAtAnEmptyOrMissingPage() {
        assertEquals(listOf(listOf(1, 2), listOf(3)), drain(0, 100, pages(listOf(1, 2), listOf(3), emptyList(), listOf(4))))
        assertEquals(listOf(listOf(1)), drain(0, 100, pages(listOf(1), null, listOf(2))))
    }

    @Test fun pagingStopsAtTheCapCountingWhatIsLoaded() {
        assertEquals(listOf(listOf(1, 2)), drain(3, 5, pages(listOf(1, 2), listOf(3))))
        assertEquals(emptyList<List<Int>>(), drain(5, 5, pages(listOf(1))))
    }

    @Test fun aFailedPageEndsPagingWithWhatArrived() {
        var calls = 0
        val next: suspend () -> List<Int>? = { if (calls++ == 0) listOf(1) else throw IOException("offline") }
        assertEquals(listOf(listOf(1)), drain(0, 100, next))
    }
}
