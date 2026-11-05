package com.nichokas.sdat.ui

import uniffi.tonearm_ffi.Track

/** Discover's tiles: tracks with a cover, at most one per track, album and cover image (as on iOS). */
class CoverPool {
    private val ids = HashSet<String>()
    private val albums = HashSet<String>()
    private val covers = HashSet<String>()
    val tracks = mutableListOf<Track>()

    fun add(batch: List<Track>) {
        for (t in batch) {
            val album = t.album ?: continue
            val cover = album.coverUrl ?: continue
            if (t.id in ids || album.id in albums || cover in covers) continue
            ids += t.id
            albums += album.id
            covers += cover
            tracks += t
        }
    }
}
