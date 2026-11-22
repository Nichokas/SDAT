package com.nichokas.sdat.ui

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.Track

class MoreTopTracksTest {
    private fun track(id: String) = Track(
        id = id,
        title = "Song $id",
        version = "",
        durationSecs = 180u,
        explicit = false,
        streamable = true,
        artists = emptyList(),
        album = null,
    )

    private class FakePager(
        private val fetch: suspend (UInt) -> List<Track>,
        private val done: () -> Boolean = { false },
    ) : TopTracksSource {
        var closed = false
            private set

        override suspend fun next(limit: UInt) = fetch(limit)

        override fun isDone() = done()

        override fun close() {
            closed = true
        }
    }

    @Test fun resetCancelsInFlightLoadClosesPagerAndKeepsNewerLoadActive() = runBlocking {
        val gate1 = CompletableDeferred<List<Track>>()
        val gate2 = CompletableDeferred<List<Track>>()
        val opened = mutableListOf<Pair<List<String>, FakePager>>()
        val more = MoreTopTracks(
            artistId = "artist-1",
            scope = this,
            open = { _, shown ->
                val gate = if (opened.isEmpty()) gate1 else gate2
                FakePager(fetch = { gate.await() }).also { opened += shown to it }
            },
            onError = {},
        )

        val first = more.load(listOf(track("old-1")))!!
        yield()
        assertTrue(more.loading)
        assertEquals(1, opened.size)
        assertEquals(listOf("old-1"), opened[0].first)

        more.reset()
        assertTrue(opened[0].second.closed)
        assertFalse(more.loading)
        assertTrue(more.tracks.isEmpty())
        assertTrue(more.canLoad)

        val second = more.load(listOf(track("new-1")))!!
        yield()
        assertTrue(more.loading)

        gate1.complete(listOf(track("stale")))
        first.join()
        assertTrue(first.isCancelled)
        assertTrue(more.loading)
        assertTrue(more.tracks.isEmpty())

        gate2.complete(listOf(track("fresh")))
        second.join()
        assertFalse(more.loading)
        assertEquals(listOf("fresh"), more.tracks.map { it.id })
        assertEquals(listOf("new-1"), opened[1].first)
        assertFalse(opened[1].second.closed)

        more.close()
        assertTrue(opened[1].second.closed)
    }

    @Test fun resetBeforeQueuedLoadStartsSkipsOpeningStalePager() = runBlocking {
        var openCalls = 0
        val more = MoreTopTracks(
            artistId = "artist-1",
            scope = this,
            open = { _, _ ->
                openCalls++
                FakePager(fetch = { emptyList() })
            },
            onError = {},
        )

        val job = more.load(listOf(track("stale-1")))!!
        more.reset()
        job.join()

        assertEquals(0, openCalls)
        assertFalse(more.loading)
        assertTrue(more.tracks.isEmpty())
    }

    @Test fun closeCancelsInFlightLoadClosesPagerAndBlocksFurtherLoads() = runBlocking {
        val gate = CompletableDeferred<List<Track>>()
        var calls = 0
        val pagers = mutableListOf<FakePager>()
        val more = MoreTopTracks(
            artistId = "artist-1",
            scope = this,
            open = { _, _ ->
                FakePager(
                    fetch = {
                        calls++
                        if (calls == 1) listOf(track("a")) else gate.await()
                    },
                ).also(pagers::add)
            },
            onError = {},
        )

        more.load(emptyList())!!.join()
        assertEquals(listOf("a"), more.tracks.map { it.id })
        assertFalse(pagers.single().closed)

        val second = more.load(more.tracks)!!
        yield()
        assertTrue(more.loading)

        more.close()
        second.join()
        assertTrue(second.isCancelled)
        assertTrue(pagers.single().closed)
        assertFalse(more.loading)
        assertNull(more.load(more.tracks))
        assertEquals(1, pagers.size)
    }

    @Test fun errorFlashesMessageAndAllowsRetryUntilDone() = runBlocking {
        var fail = true
        val errors = mutableListOf<String>()
        val pager = FakePager(
            fetch = {
                if (fail) throw IllegalStateException("boom")
                listOf(track("x"))
            },
            done = { !fail },
        )
        val more = MoreTopTracks(
            artistId = "artist-1",
            scope = this,
            open = { _, _ -> pager },
            onError = errors::add,
        )

        more.load(emptyList())!!.join()
        assertEquals(1, errors.size)
        assertFalse(more.loading)
        assertTrue(more.canLoad)

        fail = false
        more.load(emptyList())!!.join()
        assertEquals(listOf("x"), more.tracks.map { it.id })
        assertFalse(more.canLoad)
        assertNull(more.load(more.tracks))

        more.close()
        assertTrue(pager.closed)
    }
}
