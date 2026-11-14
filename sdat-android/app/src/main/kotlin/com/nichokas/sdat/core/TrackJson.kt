package com.nichokas.sdat.core

import org.json.JSONArray
import org.json.JSONObject
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Track

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
