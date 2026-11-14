package com.nichokas.sdat.core

import android.os.SystemClock
import java.io.File
import java.util.concurrent.Executors
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.tonearm_ffi.Track

/** iOS persist: the queue saved across restarts through [PlaybackStore], on one background thread so the writes stay
 * in order. Called on [scope]'s thread. */
class QueueSaver(
    dir: File,
    private val scope: CoroutineScope,
    private val io: CoroutineDispatcher = Executors.newSingleThreadExecutor().asCoroutineDispatcher(),
    private val clock: () -> Long = SystemClock::elapsedRealtime,
) {
    private val store = PlaybackStore(dir)
    /** The queue file on disk; a state is saved only against it. Only touched on [io]. */
    private var diskQueueId: Int? = null
    private var queueId = 0
    /** What the last queue file handed to the store was cut from. */
    private var cut: QueueCut? = null
    private var lastSave = 0L

    /** Five seconds since the last save, for saving the position while playing. */
    val due get() = clock() - lastSave > 5000

    /** The saved state and its queue. The next save rewrites the queue file: a restored index no longer counts from
     * its offset. */
    suspend fun load(): Pair<SavedState<Track>, SavedQueue<Track>?>? {
        val saved = withContext(io) { store.load() }
        saved?.let { queueId = it.first.queueId }
        return saved
    }

    /** The small state on every call, the queue file only when it no longer matches [queue]; nothing playing clears
     * both. */
    fun save(queue: PlayQueue<Track>, endless: Boolean, position: Double) {
        lastSave = clock()
        val current = queue.current
        if (current == null) {
            cut = null
            scope.launch(io) { store.clear() }
            return
        }
        if (cut?.matches(queue, endless) != true) {
            val saved = queue.saved(++queueId, endless)
            val written = QueueCut(queue, endless, saved.offset until saved.offset + saved.order.size)
            cut = written
            scope.launch(io) {
                if (store.saveQueue(saved)) {
                    diskQueueId = saved.id
                } else {
                    // The next save tries again; until then states stay with the queue file still on disk.
                    scope.launch { if (cut === written) cut = null }
                }
            }
        }
        val state = SavedState(queueId, current, queue.index, position, queue.repeat, queue.shuffle)
        scope.launch(io) { if (state.queueId == diskQueueId) store.saveState(state) }
    }
}

/** PlayQueue replaces a list whenever it changes it, so the same list objects mean the same queue file. [window] is
 * the base-queue indices the file holds. */
private class QueueCut(queue: PlayQueue<Track>, val endless: Boolean, val window: IntRange) {
    val tracks = queue.tracks
    val original = queue.original
    val userQueue = queue.userQueue
    val source = queue.source

    fun matches(queue: PlayQueue<Track>, endless: Boolean): Boolean {
        val index = queue.index
        return tracks === queue.tracks && original === queue.original && userQueue === queue.userQueue &&
            source == queue.source && this.endless == endless && (index == null || index in window)
    }
}
