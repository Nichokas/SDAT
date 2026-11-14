package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.File
import java.io.IOException
import java.util.concurrent.Executors

/** An album or playlist opened on this device. */
data class LocalRecent(val kind: Kind, val id: String, val title: String, val subtitle: String, val coverUrl: String?) {
    enum class Kind { Album, Playlist, Mix }
}

const val RECENTS_CAP = 100

/** [item] first, without its earlier entry, keeping the newest [RECENTS_CAP]. */
fun List<LocalRecent>.opened(item: LocalRecent): List<LocalRecent> =
    (listOf(item) + filterNot { it.kind == item.kind && it.id == item.id }).take(RECENTS_CAP)

/** The recents file. Blocking. */
class RecentsStore(private val file: File) {
    fun save(items: List<LocalRecent>) {
        val tmp = File(file.path + ".tmp")
        try {
            DataOutputStream(tmp.outputStream().buffered()).use { out ->
                out.writeInt(FORMAT)
                out.writeInt(items.size)
                items.forEach {
                    out.writeUTF(it.kind.name)
                    out.writeUTF(it.id)
                    out.writeUTF(it.title)
                    out.writeUTF(it.subtitle)
                    out.writeBoolean(it.coverUrl != null)
                    it.coverUrl?.let(out::writeUTF)
                }
            }
            if (!tmp.renameTo(file)) tmp.delete()
        } catch (_: IOException) {
            tmp.delete()
        }
    }

    fun delete() = file.delete()

    /** Empty when the file is missing or unreadable. */
    fun load(): List<LocalRecent> = try {
        DataInputStream(file.inputStream().buffered()).use { input ->
            if (input.readInt() != FORMAT) return emptyList()
            val count = input.readInt()
            if (count !in 0..RECENTS_CAP) return emptyList()
            buildList {
                repeat(count) {
                    add(
                        LocalRecent(
                            LocalRecent.Kind.valueOf(input.readUTF()),
                            input.readUTF(),
                            input.readUTF(),
                            input.readUTF(),
                            if (input.readBoolean()) input.readUTF() else null,
                        ),
                    )
                }
            }
        }
    } catch (_: IOException) {
        emptyList()
    } catch (_: IllegalArgumentException) {
        emptyList()
    }

    private companion object {
        const val FORMAT = 1
    }
}

/** iOS Recents: recently opened albums and playlists, kept on device because TIDAL's history misses some. */
object Recents {
    private var store: RecentsStore? = null

    // One thread keeps the writes in order.
    private val writer = Executors.newSingleThreadExecutor()

    // Held while a save checks it is still current and writes, so a clear can't land in between.
    private val lock = Any()

    @Volatile private var generation = 0

    var items by mutableStateOf(emptyList<LocalRecent>())
        private set

    fun open(file: File) {
        store = RecentsStore(file).also { items = it.load() }
    }

    fun add(item: LocalRecent) {
        val next = items.opened(item)
        items = next
        val s = store ?: return
        val gen = generation
        writer.execute { synchronized(lock) { if (gen == generation) s.save(next) } }
    }

    /**
     * Deletes rather than writes an empty list, which a full disk could refuse, before returning: a process killed
     * right after sign-out must not reload the old account's history. Queued saves are dropped rather than waited for.
     */
    fun clear() {
        items = emptyList()
        synchronized(lock) {
            generation++
            store?.delete()
        }
    }
}
