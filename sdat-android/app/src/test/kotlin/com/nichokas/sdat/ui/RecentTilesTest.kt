package com.nichokas.sdat.ui

import com.nichokas.sdat.core.LocalRecent
import com.nichokas.sdat.core.Route
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.tonearm_ffi.RecentItem
import uniffi.tonearm_ffi.RecentKind

class RecentTilesTest {
    private fun remote(kind: RecentKind, id: String) = RecentItem(kind, id, "R$id", "", null)
    private fun local(kind: LocalRecent.Kind, id: String) = LocalRecent(kind, id, "L$id", "", null)

    @Test fun tidalHistoryComesFirstThenDeviceOnlyEntries() {
        val tiles = recentTiles(
            listOf(remote(RecentKind.ALBUM, "a"), remote(RecentKind.ARTIST, "r"), remote(RecentKind.MIX, "m")),
            listOf(local(LocalRecent.Kind.Album, "a"), local(LocalRecent.Kind.Playlist, "p"), local(LocalRecent.Kind.Mix, "m")),
        )
        assertEquals(
            listOf(Route.Album("a"), Route.Artist("r"), Route.Playlist("m"), Route.Playlist("p")),
            tiles.map { it.route },
        )
        assertEquals(listOf("Ra", "Rr", "Rm", "Lp"), tiles.map { it.title })
        assertEquals(listOf(false, true, false, false), tiles.map { it.round })
    }

    @Test fun withoutTidalHistoryTheDeviceListShows() {
        val tiles = recentTiles(emptyList(), listOf(local(LocalRecent.Kind.Mix, "m"), local(LocalRecent.Kind.Album, "a")))
        assertEquals(listOf(Route.Playlist("m"), Route.Album("a")), tiles.map { it.route })
    }
}
