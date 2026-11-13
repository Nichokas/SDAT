package com.nichokas.sdat.core

import android.content.Context
import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Track

/** The last Discover pool on disk, so the plane shows instantly on the next launch (iOS discover.json). */
class DiscoverCache(private val file: File) {
    /** Empty when there is no pool yet, or the file is unreadable or from another format. */
    fun load(): List<Track> = runCatching { decode(file.readText()) }.getOrDefault(emptyList())

    fun save(tracks: List<Track>) {
        runCatching {
            // Written aside and renamed, so a crash mid-write never leaves half a file.
            val tmp = File.createTempFile("discover", ".tmp", file.parentFile)
            tmp.writeText(encode(tracks))
            if (!tmp.renameTo(file)) tmp.delete()
        }
    }

    fun clear() {
        file.delete()
    }

    companion object {
        private const val FORMAT = 1

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

/** A track as JSON with the fields the iOS DiscoverCache.T keeps; artist pictures are dropped as there. */
object TrackJson {
    fun encode(t: Track): JSONObject = JSONObject()
        .put("id", t.id)
        .put("title", t.title)
        .put("version", t.version)
        .put("dur", t.durationSecs.toLong())
        .put("explicit", t.explicit)
        .put("streamable", t.streamable)
        .put("artists", JSONArray(t.artists.map { JSONArray(listOf(it.id, it.name)) }))
        .put("albumId", t.album?.id)
        .put("albumTitle", t.album?.title)
        .put("cover", t.album?.coverUrl)
        .put("coverLarge", t.album?.coverUrlLarge)
        .put("bpm", t.bpm)

    /** Throws on a malformed object. */
    fun decode(o: JSONObject): Track {
        val artists = o.getJSONArray("artists")
        return Track(
            id = o.getString("id"),
            title = o.getString("title"),
            version = o.getString("version"),
            durationSecs = o.getLong("dur").toUInt(),
            explicit = o.getBoolean("explicit"),
            streamable = o.getBoolean("streamable"),
            artists = List(artists.length()) {
                val a = artists.getJSONArray(it)
                ArtistRef(a.getString(0), a.optString(1), null)
            },
            album = o.string("albumId")?.let { AlbumRef(it, o.string("albumTitle") ?: "", o.string("cover"), o.string("coverLarge")) },
            bpm = if (o.isNull("bpm")) null else o.getDouble("bpm"),
        )
    }

    private fun JSONObject.string(key: String): String? = if (isNull(key)) null else getString(key)
}
