package com.nichokas.sdat.core

import android.content.Context
import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import uniffi.tonearm_ffi.Track

/** The last Discover pool on disk, so the plane shows instantly on the next launch (iOS discover.json). */
class DiscoverCache(private val file: File) {
    /** Empty when there is no pool yet, or the file is unreadable or from another format. */
    fun load(): List<Track> = runCatching { decode(file.readText()) }.getOrDefault(emptyList())

    /** Drops the pool when the cache was cleared after [epoch], so a pool fetched for one account never lands after its sign-out. */
    fun save(tracks: List<Track>, epoch: Long) {
        runCatching {
            // Written aside and renamed, so a crash mid-write never leaves half a file.
            val text = encode(tracks)
            val tmp = File.createTempFile("discover", ".tmp", file.parentFile)
            var moved = false
            try {
                tmp.writeText(text)
                moved = synchronized(lock) { epoch == Companion.epoch && tmp.renameTo(file) }
            } finally {
                if (!moved) tmp.delete()
            }
        }
    }

    fun clear() = synchronized(lock) {
        epoch++
        file.delete()
    }

    companion object {
        private const val FORMAT = 1
        private val lock = Any()

        /** Counts clears; a load reads it when it starts and hands it to [save]. */
        @Volatile var epoch = 0L
            private set

        fun of(context: Context) = DiscoverCache(File(context.cacheDir, "discover.json"))

        fun encode(tracks: List<Track>): String =
            JSONObject().put("format", FORMAT).put("tracks", JSONArray(tracks.map(TrackJson::encode))).toString()

        fun decode(text: String): List<Track> = runCatching {
            val root = JSONObject(text)
            if (root.optInt("format") != FORMAT) return emptyList()
            val list = root.getJSONArray("tracks")
            List(list.length()) { TrackJson.decode(list.getJSONObject(it)) }
        }.getOrDefault(emptyList())
    }
}
