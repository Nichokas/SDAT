package com.nichokas.sdat.core

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class RecentsTest {
    @get:Rule val dir = TemporaryFolder()

    private fun item(id: String, kind: LocalRecent.Kind = LocalRecent.Kind.Album, title: String = "T$id") =
        LocalRecent(kind, id, title, "Artist", "https://c/$id")

    @Test fun theNewestOpenedComesFirst() {
        val list = emptyList<LocalRecent>().opened(item("1")).opened(item("2"))
        assertEquals(listOf("2", "1"), list.map { it.id })
    }

    @Test fun reopeningMovesTheEntryToTheFrontWithItsNewDetails() {
        val list = listOf(item("2"), item("1")).opened(item("1", title = "Renamed"))
        assertEquals(listOf(item("1", title = "Renamed"), item("2")), list)
    }

    @Test fun anAlbumAndAPlaylistWithTheSameIdAreDifferentEntries() {
        val list = listOf(item("1")).opened(item("1", LocalRecent.Kind.Playlist))
        assertEquals(2, list.size)
    }

    @Test fun keepsTheNewestHundred() {
        val list = (1..150).fold(emptyList<LocalRecent>()) { acc, i -> acc.opened(item("$i")) }
        assertEquals(RECENTS_CAP, list.size)
        assertEquals("150", list.first().id)
        assertEquals("51", list.last().id)
    }

    @Test fun roundTripsThroughTheFile() {
        val file = File(dir.root, "recents")
        val items = listOf(item("1", LocalRecent.Kind.Mix), LocalRecent(LocalRecent.Kind.Playlist, "p", "Ünïcødé", "", null))
        RecentsStore(file).save(items)
        assertEquals(items, RecentsStore(file).load())
    }

    @Test fun aMissingOrCorruptFileLoadsEmpty() {
        val file = File(dir.root, "recents")
        assertEquals(emptyList<LocalRecent>(), RecentsStore(file).load())
        // Right format and count, then a kind that doesn't exist.
        file.writeBytes(byteArrayOf(0, 0, 0, 1, 0, 0, 0, 5, 0, 3) + "Bad".toByteArray())
        assertEquals(emptyList<LocalRecent>(), RecentsStore(file).load())
    }

    @Test fun signOutLeavesNothingForTheNextLaunch() {
        val file = File(dir.root, "recents")
        Recents.open(file)
        Recents.add(item("1"))
        Recents.clear()
        assertEquals(emptyList<LocalRecent>(), Recents.items)
        assertEquals(emptyList<LocalRecent>(), RecentsStore(file).load())
    }
}
