package com.nichokas.sdat.core

import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.File
import java.io.IOException
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.ArtistRef
import uniffi.tonearm_ffi.Track

/** The queues as saved: a window of the base queue starting at [offset], its unshuffled order and the user queue.
 * [id] pairs it with a [SavedState], so a state never meets a stale queue. */
data class SavedQueue<T>(
    val id: Int,
    val offset: Int,
    val tracks: List<T>,
    val original: List<T>,
    val userQueue: List<T>,
    val endless: Boolean,
    val source: String?,
)

/** The small part, rewritten every few seconds while playing. [index] counts in the whole base queue. */
data class SavedState<T>(
    val queueId: Int,
    val current: T,
    val index: Int?,
    val position: Double,
    val repeat: Repeat,
    val shuffle: Boolean,
)

/** iOS PlaybackStore: what survives a restart, as two files in [dir] so the frequent position writes stay small.
 * Blocking; Playback calls it on one background thread, which keeps the writes in order. */
class PlaybackStore(private val dir: File) {
    private val queueFile = File(dir, "playback-queue")
    private val stateFile = File(dir, "playback-state")

    fun saveQueue(q: SavedQueue<Track>) = write(queueFile) {
        writeInt(q.id)
        writeInt(q.offset)
        writeTracks(q.tracks)
        writeTracks(q.original)
        writeTracks(q.userQueue)
        writeBoolean(q.endless)
        writeOptional(q.source, this::writeUTF)
    }

    fun saveState(s: SavedState<Track>) = write(stateFile) {
        writeInt(s.queueId)
        writeTrack(s.current)
        writeOptional(s.index, this::writeInt)
        writeDouble(s.position)
        writeUTF(s.repeat.name)
        writeBoolean(s.shuffle)
    }

    fun clear() {
        queueFile.delete()
        stateFile.delete()
    }

    /** The saved state, with its queue unless that is missing or belongs to another state. An unreadable state drops
     * both files. */
    fun load(): Pair<SavedState<Track>, SavedQueue<Track>?>? {
        val state = read(stateFile) {
            SavedState(readInt(), readTrack(), readOptional(this::readInt), readDouble(), Repeat.valueOf(readUTF()), readBoolean())
        } ?: return null.also { clear() }
        val queue = read(queueFile) {
            SavedQueue(readInt(), readInt(), readTracks(), readTracks(), readTracks(), readBoolean(), readOptional(this::readUTF))
        }
        return state to queue?.takeIf { it.id == state.queueId }
    }

    private fun write(file: File, body: DataOutputStream.() -> Unit) {
        val tmp = File(dir, file.name + ".tmp")
        try {
            DataOutputStream(tmp.outputStream().buffered()).use {
                it.writeInt(FORMAT)
                it.body()
            }
            tmp.renameTo(file)
        } catch (_: IOException) {
            tmp.delete()
        }
    }

    private fun <R> read(file: File, body: DataInputStream.() -> R): R? = try {
        DataInputStream(file.inputStream().buffered()).use { if (it.readInt() == FORMAT) it.body() else null }
    } catch (_: IOException) {
        null
    } catch (_: IllegalArgumentException) {
        null
    }

    private companion object {
        const val FORMAT = 1
    }
}

private fun <V : Any> DataOutputStream.writeOptional(v: V?, write: (V) -> Unit) {
    writeBoolean(v != null)
    if (v != null) write(v)
}

private fun <V> DataInputStream.readOptional(read: () -> V): V? = if (readBoolean()) read() else null

private fun DataOutputStream.writeTracks(list: List<Track>) {
    writeInt(list.size)
    list.forEach { writeTrack(it) }
}

// Grown as read, so a corrupt count runs out of input instead of allocating it.
private fun DataInputStream.readTracks(): List<Track> = buildList { repeat(readInt()) { add(readTrack()) } }

private fun DataOutputStream.writeTrack(t: Track) {
    writeUTF(t.id)
    writeUTF(t.title)
    writeUTF(t.version)
    writeInt(t.durationSecs.toInt())
    writeBoolean(t.explicit)
    writeBoolean(t.streamable)
    writeInt(t.artists.size)
    t.artists.forEach {
        writeUTF(it.id)
        writeUTF(it.name)
        writeOptional(it.pictureUrl, this::writeUTF)
    }
    writeOptional(t.album) {
        writeUTF(it.id)
        writeUTF(it.title)
        writeOptional(it.coverUrl, this::writeUTF)
        writeOptional(it.coverUrlLarge, this::writeUTF)
    }
    writeOptional(t.bpm, this::writeDouble)
}

private fun DataInputStream.readTrack() = Track(
    id = readUTF(),
    title = readUTF(),
    version = readUTF(),
    durationSecs = readInt().toUInt(),
    explicit = readBoolean(),
    streamable = readBoolean(),
    artists = buildList { repeat(readInt()) { add(ArtistRef(readUTF(), readUTF(), readOptional(this@readTrack::readUTF))) } },
    album = readOptional { AlbumRef(readUTF(), readUTF(), readOptional(this::readUTF), readOptional(this::readUTF)) },
    bpm = readOptional(this::readDouble),
)
