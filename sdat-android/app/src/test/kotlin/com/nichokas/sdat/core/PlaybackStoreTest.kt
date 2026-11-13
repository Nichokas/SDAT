package com.nichokas.sdat.core

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Track

class PlaybackStoreTest {
    @get:Rule val dir = TemporaryFolder()

    private val full = Track(
        id = "1", title = "Song", version = "(Live)", durationSecs = 4_000_000_000u, explicit = true, streamable = true,
        artists = listOf(ArtistRef("a1", "Ünïcødé", "https://p"), ArtistRef("a2", "", null)),
        album = AlbumRef("al", "Album", "https://c", null), bpm = 121.5,
    )
    private val bare = Track(id = "2", title = "", version = "", durationSecs = 0u, explicit = false, streamable = false, artists = emptyList(), album = null)
    private val queue = SavedQueue(7, 40, listOf(full, bare), listOf(bare, full), listOf(bare), endless = true, source = "Discover")
    private val state = SavedState(7, full, 41, 93.25, Repeat.One, shuffle = true)

    private fun store() = PlaybackStore(dir.root)

    @Test fun roundTripsTheQueueAndTheState() {
        store().saveQueue(queue)
        store().saveState(state)
        assertEquals(state to queue, store().load())
    }

    @Test fun roundTripsEmptyOptionals() {
        val q = queue.copy(tracks = emptyList(), endless = false, source = null)
        store().saveQueue(q)
        store().saveState(state.copy(current = bare, index = null))
        assertEquals(state.copy(current = bare, index = null) to q, store().load())
    }

    @Test fun aStateNeverPairsWithAnotherQueue() {
        store().saveQueue(queue)
        store().saveState(state.copy(queueId = 8))
        assertEquals(state.copy(queueId = 8) to null, store().load())
    }

    @Test fun anUnreadableStateDropsBothFiles() {
        store().saveQueue(queue)
        store().saveState(state)
        val file = File(dir.root, "playback-state")
        file.writeBytes(file.readBytes().copyOf(20))
        assertNull(store().load())
        assertEquals(emptyList<String>(), dir.root.list()!!.toList())
    }

    @Test fun clearForgetsEverything() {
        store().saveQueue(queue)
        store().saveState(state)
        store().clear()
        assertNull(store().load())
    }
}
