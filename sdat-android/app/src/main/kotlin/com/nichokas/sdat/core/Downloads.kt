package com.nichokas.sdat.core

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.cancelChildren
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
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
    /** The worker and the restore of the saved queue, which [removeAll] and [pause] cancel together. */
    private val work = MainScope()
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
    private var fetching: Job? = null
    private var restoring: Job? = null
    private val dropped = mutableSetOf<String>()
    private val gate = Mutex()
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
        if (frozen) {
            frozen = false
            resumePending()
        }
        val next = queue.enqueue(list, ::isDownloaded)
        if (next === queue) return
        queue = next
        savePending()
        pump()
        onActive()
    }

    fun cancel(id: String) {
        // Otherwise a restore still reading the saved queue would be cancelled and the saved queue overwritten.
        if (!queue.isQueued(id)) return
        val wasCurrent = queue.current?.id == id
        queue = queue.cancel(id)
        if (wasCurrent) fetching?.cancel()
        // Otherwise a restore still reading the saved queue would bring the track back.
        if (restoring?.isActive == true) dropped += id
        savePending()
    }

    fun cancelAll() = queue.active.forEach { cancel(it.id) }

    fun remove(id: String) {
        cancel(id)
        queue = queue.forget(id)
        val e = entries[id] ?: return
        entries = entries - id
        // The track being fetched may already have taken this album's cover.
        val coverUnused = e.cover != null && entries.values.none { it.cover == e.cover } && queue.current?.album?.id != e.track.album?.id
        scope.launch(writes) {
            discard(id)
            if (coverUnused) File(dir, e.cover).delete()
        }
        saveIndex()
    }

    fun remove(list: List<Track>) = list.forEach { remove(it.id) }

    /**
     * Cancels the queue and deletes every file: sign-out and erase. The deletion is queued behind the writes already
     * made, and the cancelled worker writes nothing more (see [save]), so no file of the old queue can come back.
     */
    fun removeAll() {
        work.coroutineContext.cancelChildren()
        queue = DownloadQueue()
        entries = emptyMap()
        albumMeta.clear()
        scope.launch(writes) { dir.deleteRecursively() }
    }

    /** The foreground service ran out of time: stop, keeping the queue on disk for the next launch. */
    fun pause() {
        savePending()
        frozen = true
        queue = queue.cancelAll()
        work.coroutineContext.cancelChildren()
    }

    /** Requeues downloads that a timeout, a kill or a crash interrupted. Call from the foreground. */
    fun resumePending() {
        restoring = work.launch {
            val saved = withContext(writes) {
                runCatching {
                    val list = JSONArray(File(dir, PENDING).readText())
                    List(list.length()) { TrackJson.decode(list.getJSONObject(it)) }
                }.getOrDefault(emptyList())
            }
            val keep = saved.filterNot { it.id in dropped }
            dropped.clear()
            restoring = null
            queue = queue.enqueue(keep, ::isDownloaded)
            // Also rewrites a saved queue whose tracks were all skipped as downloaded.
            savePending()
            pump()
            if (queue.isActive) onActive()
        }
    }

    private fun pump() {
        if (worker?.isActive == true) return
        worker = work.launch {
            // A cancelled worker may still be in a read, and cleans up before its scope returns and frees the lock; the
            // lock keeps the next worker waiting even when the one queued in between is cancelled too.
            gate.withLock { drain() }
        }
    }

    private suspend fun drain() = coroutineScope {
        while (true) {
            queue = queue.startNext()
            val t = queue.current ?: break
            var error: String? = null
            // A child, so cancelling the worker cancels it; cancelling it alone (cancel(id)) moves on to the next.
            val job = launch {
                try {
                    commit(t, fetch(t))
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    error = friendlyMessage(e)
                }
            }
            fetching = job
            job.join()
            queue = queue.finish(t.id, error)
            savePending()
        }
    }

    /** Runs in the fetch's own job right after it returns, with no suspension a cancellation could slip into. */
    private fun commit(t: Track, e: DownloadEntry) {
        if (queue.current?.id != t.id) {
            scope.launch(writes) { discard(t.id) }
            return
        }
        entries = entries + (t.id to e)
        val snapshot = entries
        scope.launch(writes) {
            if (atomicWrite(File(dir, INDEX), DownloadIndex.encode(snapshot))) return@launch
            // Without the index the copy is gone after a restart, so it is not counted as saved now either.
            withContext(Dispatchers.Main) {
                if (entries[t.id] !== e) return@withContext
                entries = entries - t.id
                queue = queue.fail(t.id, DownloadFailure.storage.message.orEmpty())
                saveIndex()
                launch(writes) { discard(t.id) }
            }
        }
    }

    /** Leaves no file of [t] behind unless it returns. */
    private suspend fun fetch(t: Track): DownloadEntry {
        // Always the top tier the account and track allow, whatever the streaming setting.
        val s = stream(t.id, Quality.HI_FI)
        var (text, base) = manifest(s.url)
        var variant: Hls.Variant? = null
        if (Hls.isMaster(text)) {
            variant = Hls.bestVariant(text, base) ?: throw DownloadFailure.unsupported("master playlist")
            val (data, url) = request(variant.uri)
            text = data.decodeToString()
            base = url
        }
        val media = Hls.media(text, base)
        val parts = listOf(media.map) + media.segments.map { it.uri }
        val sizes = LongArray(parts.size)
        var spec: Pair<Int?, Int?> = null to null
        val name = "${t.id}.mp4"
        val tmp = File(dir, partName(t.id))
        try {
            save { dir.mkdirs(); tmp.delete() }
            for ((i, uri) in parts.withIndex()) {
                val data = get(uri)
                if (i == 0) spec = Hls.audioSpec(data)
                save { tmp.appendBytes(data) }
                sizes[i] = data.size.toLong()
                queue = queue.progress(t.id, (i + 1.0) / parts.size)
            }
            save {
                val playlist = Hls.localPlaylist(name, sizes[0], media.segments.mapIndexed { i, seg -> seg to sizes[i + 1] })
                File(dir, playlistName(t.id)).writeText(playlist)
                val dst = File(dir, name)
                dst.delete()
                if (!tmp.renameTo(dst)) throw IOException("rename failed")
            }
            val a = t.album
            val cover = a?.let { saveCover(it.id, it.coverUrlLarge ?: it.coverUrl) }
            val detail = a?.let { albumMeta[it.id] ?: orNull { album(it.id) }?.also { d -> albumMeta[it.id] = d } }
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
        } catch (e: Throwable) {
            withContext(NonCancellable + writes) { discard(t.id) }
            throw e
        }
    }

    /**
     * A file write of the running download, in order with every other write. Once its download is cancelled it does
     * not run, which is what lets [removeAll] delete the folder without waiting for the worker.
     */
    private suspend fun <T> save(block: () -> T): T = withContext(writes) {
        ensureActive()
        try {
            block()
        } catch (e: IOException) {
            throw DownloadFailure.storage
        }
    }

    private fun discard(id: String) {
        File(dir, partName(id)).delete()
        File(dir, "$id.mp4").delete()
        File(dir, playlistName(id)).delete()
    }

    /** [block]'s value, or null when it fails; a cancellation still cancels. */
    private inline fun <T> orNull(block: () -> T): T? = try {
        block()
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        null
    }

    /** The album's cover file name, fetching it once per album; null when it can't be had. */
    private suspend fun saveCover(albumId: String, url: String?): String? {
        val name = "cover-$albumId.jpg"
        val file = File(dir, name)
        if (withContext(Dispatchers.IO) { file.isFile }) return name
        val data = orNull { get(URI(url ?: return null)) } ?: return null
        return orNull {
            save {
                val tmp = File(dir, "$name.part")
                try {
                    tmp.writeBytes(data)
                    if (!tmp.renameTo(file)) throw IOException("rename failed")
                } finally {
                    tmp.delete()
                }
                name
            }
        }
    }

    private suspend fun manifest(url: String): Pair<String, URI?> {
        Hls.inline(url)?.let { return it to null }
        val uri = runCatching { URI(url) }.getOrElse { throw DownloadFailure.unsupported("manifest URL") }
        val (data, final) = request(uri)
        return data.decodeToString() to final
    }

    private suspend fun get(uri: URI): ByteArray = request(uri).first

    /**
     * The body and the URL it came from after redirects, which relative playlist entries resolve against. Three
     * attempts, 1 s then 2 s apart; each one first checks the cellular setting.
     */
    private suspend fun request(uri: URI): Pair<ByteArray, URI> {
        var last: Exception = IOException("No attempt made")
        repeat(3) { attempt ->
            if (!prefs.downloadOverCellular && onCellular()) throw DownloadFailure.cellular
            try {
                return withContext(Dispatchers.IO) {
                    val c = uri.toURL().openConnection() as HttpURLConnection
                    c.connectTimeout = TIMEOUT_MS
                    c.readTimeout = TIMEOUT_MS
                    try {
                        if (c.responseCode !in 200..299) throw IOException("HTTP ${c.responseCode}")
                        // A blocking read ignores cancellation, so it is checked between chunks.
                        val body = ByteArrayOutputStream()
                        c.inputStream.use { input ->
                            val buf = ByteArray(64 * 1024)
                            while (true) {
                                ensureActive()
                                val n = input.read(buf)
                                if (n < 0) break
                                body.write(buf, 0, n)
                            }
                        }
                        body.toByteArray() to c.url.toURI()
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

    /** Skipped while a restore runs: the file still holds the saved queue, and the restore writes the merged one. */
    private fun savePending() {
        if (frozen || restoring?.isActive == true) return
        val list = queue.active
        scope.launch(writes) {
            val file = File(dir, PENDING)
            if (list.isEmpty()) file.delete() else atomicWrite(file, JSONArray(list.map(TrackJson::encode)).toString())
        }
    }

    private fun atomicWrite(file: File, text: String): Boolean {
        val tmp = File(dir, file.name + ".tmp")
        return try {
            dir.mkdirs()
            tmp.writeText(text)
            tmp.renameTo(file).also { if (!it) tmp.delete() }
        } catch (_: IOException) {
            tmp.delete()
            false
        }
    }

    private fun playlistName(id: String) = "$id.m3u8"

    private fun partName(id: String) = "$id.part"

    private const val INDEX = "index.json"
    private const val PENDING = "pending.json"
    private const val TIMEOUT_MS = 30_000
}
