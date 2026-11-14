package com.nichokas.sdat.core

import org.json.JSONArray
import org.json.JSONObject
import uniffi.tonearm_ffi.Track

/** A saved track (iOS Downloads.Entry): its file and cover inside the downloads folder, and what it plays at. */
data class DownloadEntry(
    val track: Track,
    val file: String,
    val cover: String?,
    val quality: String,
    val size: Long,
    /** Epoch milliseconds. */
    val date: Long,
    val gainDb: Double? = null,
    val peak: Double? = null,
    val albumGainDb: Double? = null,
    val albumPeak: Double? = null,
    val formats: List<String> = emptyList(),
    /** HLS CODECS, e.g. "flac", "mp4a.40.2". */
    val codec: String? = null,
    /** Bits per second, from EXT-X-STREAM-INF. */
    val bandwidth: Int? = null,
    val sampleRate: Int? = null,
    /** FLAC only. */
    val bitDepth: Int? = null,
    val albumArtist: String? = null,
    val albumYear: String? = null,
    val trackNumber: Int? = null,
) {
    /** "HI-RES FLAC 24-bit/96 kHz", "FLAC 16-bit/44.1 kHz", "AAC 320 kbps"; null when unknown. */
    val qualityText: String?
        get() {
            val khz = sampleRate?.let { r ->
                val k = r / 1000.0
                if (k == Math.rint(k)) "${k.toInt()} kHz" else "%.1f kHz".format(java.util.Locale.ROOT, k)
            }
            val lower = codec?.lowercase().orEmpty()
            val lossless = when {
                "flac" in lower || formats.any { it.startsWith("FLAC") } -> "FLAC"
                "alac" in lower -> "ALAC"
                else -> null
            }
            if (lossless != null) {
                val hires = (bitDepth ?: 16) > 16 || (sampleRate ?: 44100) > 48000
                val spec = listOfNotNull(bitDepth?.let { "$it-bit" }, khz).joinToString("/")
                val head = if (hires) "HI-RES $lossless" else lossless
                return if (spec.isEmpty()) head else "$head $spec"
            }
            if (codec != null || formats.isNotEmpty()) {
                return "AAC" + (bandwidth?.let { " ${it / 1000} kbps" } ?: "") + (khz?.let { " · $it" } ?: "")
            }
            return null
        }
}

/** The downloads index file (iOS index.json), keyed by track id. Tracks use [TrackJson], like the Discover cache. */
object DownloadIndex {
    private const val FORMAT = 1

    fun encode(entries: Map<String, DownloadEntry>): String {
        val all = JSONObject()
        entries.forEach { (id, e) -> all.put(id, encode(e)) }
        return JSONObject().put("format", FORMAT).put("entries", all).toString()
    }

    /** Empty for an unreadable file or another format; a malformed entry is dropped alone. */
    fun decode(text: String): Map<String, DownloadEntry> = runCatching {
        val root = JSONObject(text)
        if (root.optInt("format") != FORMAT) return emptyMap()
        val all = root.getJSONObject("entries")
        all.keys().asSequence().mapNotNull { id -> runCatching { id to decode(all.getJSONObject(id)) }.getOrNull() }.toMap()
    }.getOrDefault(emptyMap())

    private fun encode(e: DownloadEntry): JSONObject = JSONObject()
        .put("track", TrackJson.encode(e.track))
        .put("file", e.file)
        .put("cover", e.cover)
        .put("quality", e.quality)
        .put("size", e.size)
        .put("date", e.date)
        .put("gainDb", e.gainDb)
        .put("peak", e.peak)
        .put("albumGainDb", e.albumGainDb)
        .put("albumPeak", e.albumPeak)
        .put("formats", JSONArray(e.formats))
        .put("codec", e.codec)
        .put("bandwidth", e.bandwidth)
        .put("sampleRate", e.sampleRate)
        .put("bitDepth", e.bitDepth)
        .put("albumArtist", e.albumArtist)
        .put("albumYear", e.albumYear)
        .put("trackNumber", e.trackNumber)

    private fun decode(o: JSONObject): DownloadEntry {
        fun str(k: String) = if (o.isNull(k)) null else o.getString(k)
        fun dbl(k: String) = if (o.isNull(k)) null else o.getDouble(k)
        fun int(k: String) = if (o.isNull(k)) null else o.getInt(k)
        val formats = o.optJSONArray("formats")
        return DownloadEntry(
            track = TrackJson.decode(o.getJSONObject("track")),
            file = o.getString("file"),
            cover = str("cover"),
            quality = o.getString("quality"),
            size = o.getLong("size"),
            date = o.getLong("date"),
            gainDb = dbl("gainDb"),
            peak = dbl("peak"),
            albumGainDb = dbl("albumGainDb"),
            albumPeak = dbl("albumPeak"),
            formats = formats?.let { a -> List(a.length()) { a.getString(it) } }.orEmpty(),
            codec = str("codec"),
            bandwidth = int("bandwidth"),
            sampleRate = int("sampleRate"),
            bitDepth = int("bitDepth"),
            albumArtist = str("albumArtist"),
            albumYear = str("albumYear"),
            trackNumber = int("trackNumber"),
        )
    }
}

sealed interface DownloadState {
    data object None : DownloadState
    data object Queued : DownloadState
    data class Downloading(val fraction: Double) : DownloadState
    data object Done : DownloadState
    data class Failed(val message: String) : DownloadState
}

/**
 * The download queue: tracks [waiting] in order, the one being fetched, and the last failure per track.
 * [batchTotal] counts tracks added since the queue was last empty, less cancelled ones, for whole-batch progress.
 */
data class DownloadQueue(
    val waiting: List<Track> = emptyList(),
    val current: Track? = null,
    val fraction: Double = 0.0,
    val failures: Map<String, String> = emptyMap(),
    val batchTotal: Int = 0,
) {
    /** Being fetched first, then waiting. */
    val active: List<Track> get() = listOfNotNull(current) + waiting
    val isActive: Boolean get() = current != null || waiting.isNotEmpty()
    val batchDone: Int get() = maxOf(0, batchTotal - active.size)
    val batchProgress: Double
        get() = if (batchTotal == 0) 0.0 else minOf(1.0, (batchDone + if (current != null) fraction else 0.0) / batchTotal)

    fun isQueued(id: String) = current?.id == id || waiting.any { it.id == id }

    fun state(id: String, downloaded: Boolean): DownloadState = when {
        downloaded -> DownloadState.Done
        current?.id == id -> DownloadState.Downloading(fraction)
        waiting.any { it.id == id } -> DownloadState.Queued
        else -> failures[id]?.let(DownloadState::Failed) ?: DownloadState.None
    }

    /** Queues [tracks], skipping unstreamable, [downloaded] and already queued ones; a retry clears its failure. */
    fun enqueue(tracks: List<Track>, downloaded: (String) -> Boolean): DownloadQueue {
        val ids = active.mapTo(mutableSetOf()) { it.id }
        val added = tracks.filter { it.streamable && !downloaded(it.id) && ids.add(it.id) }
        if (added.isEmpty()) return this
        return copy(waiting = waiting + added, failures = failures - added.map { it.id }.toSet(), batchTotal = batchTotal + added.size)
    }

    /** Moves the first waiting track to [current] when nothing is being fetched. */
    fun startNext(): DownloadQueue =
        if (current != null || waiting.isEmpty()) this else copy(current = waiting.first(), waiting = waiting.drop(1), fraction = 0.0)

    fun progress(id: String, fraction: Double) = if (current?.id == id) copy(fraction = fraction) else this

    /** The fetch of [id] ended, with [error] when it failed; ignored when [id] was cancelled meanwhile. */
    fun finish(id: String, error: String?): DownloadQueue {
        if (current?.id != id) return this
        return copy(current = null, fraction = 0.0, failures = if (error != null) failures + (id to error) else failures).settled()
    }

    fun cancel(id: String): DownloadQueue {
        if (!isQueued(id)) return this
        return copy(
            waiting = waiting.filter { it.id != id },
            current = current?.takeIf { it.id != id },
            fraction = if (current?.id == id) 0.0 else fraction,
            batchTotal = batchTotal - 1,
        ).settled()
    }

    fun cancelAll() = DownloadQueue(failures = failures)

    fun forget(id: String) = copy(failures = failures - id)

    fun fail(id: String, error: String) = copy(failures = failures + (id to error))

    private fun settled() = if (isActive) this else copy(batchTotal = 0)
}

/** An album's downloaded tracks in album order when known, else in the order they were saved. */
fun offlineAlbum(entries: Collection<DownloadEntry>, albumId: String): List<Track> =
    entries.filter { it.track.album?.id == albumId }
        .sortedWith(compareBy({ it.trackNumber ?: Int.MAX_VALUE }, { it.date }))
        .map { it.track }
