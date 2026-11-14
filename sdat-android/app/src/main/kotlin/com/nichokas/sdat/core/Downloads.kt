package com.nichokas.sdat.core

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import uniffi.tonearm_ffi.AlbumDetail
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.album
import uniffi.tonearm_ffi.stream

/** A downloaded track as the player opens it: a local HLS playlist over the saved file. */
data class LocalStream(val playlist: File, val quality: Quality)

/**
 * Offline tracks (iOS Downloads). TIDAL only hands out HLS, so a download saves the fMP4 media playlist's init segment
 * and fragments back to back in one file, plus a local playlist that indexes them; encrypted or MPEG-TS streams are
 * refused. Files live in no_backup/Downloads with `index.json`; the queue survives a restart in `pending.json`.
 * State is read and written on the main thread only.
 */
object Downloads {
    private lateinit var dir: File
    private lateinit var prefs: Preferences
    private lateinit var connectivity: ConnectivityManager
    private lateinit var onActive: () -> Unit
    private val scope = MainScope()
    // One file write at a time, in call order, so the last snapshot always lands last.
    private val writes = Dispatchers.IO.limitedParallelism(1)

    var entries by mutableStateOf<Map<String, DownloadEntry>>(emptyMap())
        private set
    var queue by mutableStateOf(DownloadQueue())
        private set
    private val coverMap by derivedStateOf {
        entries.values.flatMap { e ->
            val cover = e.cover ?: return@flatMap emptyList()
            listOfNotNull(e.track.album?.coverUrl, e.track.album?.coverUrlLarge).map { it to File(dir, cover) }
        }.toMap()
    }
    private var worker: Job? = null
    private var fetching: Deferred<DownloadEntry>? = null
    private val albumMeta = mutableMapOf<String, AlbumDetail>()

    /** Set once the foreground service ran out of time: the queue on disk must outlive the teardown. */
    private var frozen = false

    /** [onActive] keeps the process alive while the queue runs. */
    fun init(context: Context, prefs: Preferences, onActive: () -> Unit) {
        dir = File(context.noBackupFilesDir, "Downloads")
        this.prefs = prefs
        this.onActive = onActive
        connectivity = context.getSystemService(ConnectivityManager::class.java)
        val saved = runCatching { DownloadIndex.decode(File(dir, INDEX).readText()) }.getOrDefault(emptyMap())
        entries = saved.filterValues { File(dir, it.file).isFile }
    }

    val totalSize: Long get() = entries.values.sumOf { it.size }

    /** Newest first. */
    val tracks: List<Track> get() = entries.values.sortedByDescending { it.date }.map { it.track }

    fun state(id: String) = queue.state(id, entries.containsKey(id))

    fun isDownloaded(id: String) = entries.containsKey(id)

    fun coverFile(remote: String?): File? = remote?.let { coverMap[it] }

    fun tracksInAlbum(albumId: String) = offlineAlbum(entries.values, albumId)

    fun albumInfo(albumId: String): Pair<String?, String?> =
        entries.values.firstOrNull { it.track.album?.id == albumId }.let { it?.albumArtist to it?.albumYear }

    fun localStream(id: String): LocalStream? {
        val e = entries[id] ?: return null
        val playlist = File(dir, playlistName(id))
        if (!playlist.isFile || File(dir, e.file).length() == 0L) return null
        return LocalStream(playlist, qualityOf(e.quality) ?: Quality.HI_FI)
    }

    /** An album page loaded online: its artist, year and track numbers fill in the album's saved entries. */
    fun note(d: AlbumDetail) {
        val artist = d.album.artistLine
        val year = year(d.album.releaseDate)
        var next = entries
        d.tracks.forEachIndexed { i, t ->
            val e = next[t.id] ?: return@forEachIndexed
            val filled = e.copy(albumArtist = artist, albumYear = year, trackNumber = i + 1)
            if (filled != e) next = next + (t.id to filled)
        }
        if (next !== entries) {
            entries = next
            saveIndex()
        }
    }

    /** Queues [list]; downloaded, queued and unstreamable tracks are skipped. */
    fun download(list: List<Track>) {
        val next = queue.enqueue(list, ::isDownloaded)
        if (next === queue) return
        frozen = false
        queue = next
        savePending()
        pump()
        onActive()
    }

    fun cancel(id: String) {
        val wasCurrent = queue.current?.id == id
        queue = queue.cancel(id)
        if (wasCurrent) fetching?.cancel()
        savePending()
    }

    fun cancelAll() = queue.active.forEach { cancel(it.id) }

    fun remove(id: String) {
        cancel(id)
        queue = queue.forget(id)
        val e = entries[id] ?: return
        entries = entries - id
        val coverUnused = e.cover != null && entries.values.none { it.cover == e.cover }
        scope.launch(writes) {
            File(dir, e.file).delete()
            File(dir, playlistName(id)).delete()
            if (coverUnused) File(dir, e.cover).delete()
        }
        saveIndex()
    }

    fun remove(list: List<Track>) = list.forEach { remove(it.id) }

    /** Cancels the queue, waits for the worker to stop, then deletes every file: sign-out and erase. */
    fun removeAll() {
        queue = DownloadQueue()
        entries = emptyMap()
        albumMeta.clear()
        scope.launch {
            worker?.cancelAndJoin()
            fetching?.cancelAndJoin()
            withContext(writes) { dir.deleteRecursively() }
        }
    }

    /** The foreground service ran out of time: stop, keeping the queue on disk for the next launch. */
    fun pause() {
        savePending()
        frozen = true
        queue = queue.cancelAll()
        worker?.cancel()
        fetching?.cancel()
    }

    /** Requeues downloads that a timeout, a kill or a crash interrupted. Call from the foreground. */
    fun resumePending() {
        scope.launch {
            val saved = withContext(writes) {
                runCatching {
                    val list = JSONArray(File(dir, PENDING).readText())
                    List(list.length()) { TrackJson.decode(list.getJSONObject(it)) }
                }.getOrDefault(emptyList())
            }
            if (saved.isNotEmpty()) download(saved)
        }
    }

    private fun pump() {
        if (worker?.isActive == true) return
        worker = scope.launch {
            while (true) {
                queue = queue.startNext()
                val t = queue.current ?: break
                val job = scope.async { fetch(t) }
                fetching = job
                val error = try {
                    val e = job.await()
                    if (queue.current?.id == t.id) {
                        entries = entries + (t.id to e)
                        saveIndex()
                    } else {
                        withContext(writes) { File(dir, e.file).delete(); File(dir, playlistName(t.id)).delete() }
                    }
                    null
                } catch (e: CancellationException) {
                    // The track was cancelled: carry on, unless the worker itself was.
                    ensureActive()
                    null
                } catch (e: Exception) {
                    friendlyMessage(e)
                } finally {
                    job.cancel()
                }
                queue = queue.finish(t.id, error)
                savePending()
            }
        }
    }

    private suspend fun fetch(t: Track): DownloadEntry {
        if (!prefs.downloadOverCellular && onCellular()) throw DownloadFailure.cellular
        // Always the top tier the account and track allow, whatever the streaming setting.
        val s = stream(t.id, Quality.HI_FI)
        var (text, base) = manifest(s.url)
        var variant: Hls.Variant? = null
        if (Hls.isMaster(text)) {
            variant = Hls.bestVariant(text, base) ?: throw DownloadFailure.unsupported("master playlist")
            text = get(variant.uri).decodeToString()
            base = variant.uri
        }
        val media = Hls.media(text, base)
        val parts = listOf(media.map) + media.segments.map { it.uri }
        val sizes = LongArray(parts.size)
        var spec: Pair<Int?, Int?> = null to null
        val name = "${t.id}.mp4"
        val tmp = File(dir, "${t.id}.part")
        try {
            withContext(Dispatchers.IO) { dir.mkdirs(); tmp.delete() }
            for ((i, uri) in parts.withIndex()) {
                val data = get(uri)
                if (i == 0) spec = Hls.audioSpec(data)
                withContext(Dispatchers.IO) { tmp.appendBytes(data) }
                sizes[i] = data.size.toLong()
                queue = queue.progress(t.id, (i + 1.0) / parts.size)
            }
            withContext(Dispatchers.IO) {
                val playlist = Hls.localPlaylist(name, sizes[0], media.segments.mapIndexed { i, seg -> seg to sizes[i + 1] })
                File(dir, playlistName(t.id)).writeText(playlist)
                val dst = File(dir, name)
                dst.delete()
                if (!tmp.renameTo(dst)) throw IOException("Couldn't save the download.")
            }
        } catch (e: Throwable) {
            withContext(Dispatchers.IO) { tmp.delete() }
            throw e
        }
        val a = t.album
        val cover = a?.let { saveCover(it.id, it.coverUrlLarge ?: it.coverUrl) }
        val detail = a?.let { albumMeta[it.id] ?: runCatching { album(it.id) }.getOrNull()?.also { d -> albumMeta[it.id] = d } }
        return DownloadEntry(
            track = t,
            file = name,
            cover = cover,
            quality = s.actualQuality.key,
            size = sizes.sum(),
            date = System.currentTimeMillis(),
            gainDb = s.trackGain?.gainDb,
            peak = s.trackGain?.peak,
            albumGainDb = s.albumGain?.gainDb,
            albumPeak = s.albumGain?.peak,
            formats = s.formats,
            codec = variant?.codecs,
            bandwidth = variant?.bandwidth,
            sampleRate = spec.first,
            bitDepth = spec.second,
            albumArtist = detail?.album?.artistLine,
            albumYear = detail?.album?.releaseDate?.let(::year),
            trackNumber = detail?.tracks?.indexOfFirst { it.id == t.id }?.takeIf { it >= 0 }?.plus(1),
        )
    }

    /** The album's cover file name, fetching it once per album; null when it can't be had. */
    private suspend fun saveCover(albumId: String, url: String?): String? {
        val name = "cover-$albumId.jpg"
        val file = File(dir, name)
        if (withContext(Dispatchers.IO) { file.isFile }) return name
        val data = runCatching { get(URI(url ?: return null)) }.getOrNull() ?: return null
        return withContext(Dispatchers.IO) {
            val tmp = File(dir, "$name.part")
            if (runCatching { tmp.writeBytes(data); tmp.renameTo(file) }.getOrDefault(false)) {
                name
            } else {
                tmp.delete()
                null
            }
        }
    }

    private suspend fun manifest(url: String): Pair<String, URI?> {
        Hls.inline(url)?.let { return it to null }
        val uri = runCatching { URI(url) }.getOrElse { throw DownloadFailure.unsupported("manifest URL") }
        return get(uri).decodeToString() to uri
    }

    /** Three attempts, 1 s then 2 s apart. */
    private suspend fun get(uri: URI): ByteArray {
        var last: Exception = IOException("No attempt made")
        repeat(3) { attempt ->
            try {
                return withContext(Dispatchers.IO) {
                    val c = uri.toURL().openConnection() as HttpURLConnection
                    c.connectTimeout = TIMEOUT_MS
                    c.readTimeout = TIMEOUT_MS
                    try {
                        if (c.responseCode !in 200..299) throw IOException("HTTP ${c.responseCode}")
                        c.inputStream.use { it.readBytes() }
                    } finally {
                        c.disconnect()
                    }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: IOException) {
                last = e
                if (attempt < 2) delay(1000L * (attempt + 1))
            }
        }
        throw last
    }

    private fun onCellular() =
        connectivity.getNetworkCapabilities(connectivity.activeNetwork)?.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) == true

    private fun saveIndex() {
        val snapshot = entries
        scope.launch(writes) { atomicWrite(File(dir, INDEX), DownloadIndex.encode(snapshot)) }
    }

    private fun savePending() {
        if (frozen) return
        val list = queue.active
        scope.launch(writes) {
            val file = File(dir, PENDING)
            if (list.isEmpty()) file.delete() else atomicWrite(file, JSONArray(list.map(TrackJson::encode)).toString())
        }
    }

    private fun atomicWrite(file: File, text: String) {
        runCatching {
            dir.mkdirs()
            val tmp = File(dir, file.name + ".tmp")
            tmp.writeText(text)
            if (!tmp.renameTo(file)) tmp.delete()
        }
    }

    private fun playlistName(id: String) = "$id.m3u8"

    private const val INDEX = "index.json"
    private const val PENDING = "pending.json"
    private const val TIMEOUT_MS = 30_000
}
