package com.nichokas.sdat.core

import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.tonearm_ffi.Track

class QueueSaverTest {
    @get:Rule val dir = TemporaryFolder()

    private var now = 10_000L
    private val queue = PlayQueue<Track> { it.id }
    private val saver by lazy { saver() }

    private fun track(id: String) =
        Track(id = id, title = id, version = "", durationSecs = 100u, explicit = false, streamable = true, artists = emptyList(), album = null)

    private fun tracks(n: Int) = (0 until n).map { track("$it") }

    // Unconfined runs each write before save returns, in the order it was queued.
    private fun saver() = QueueSaver(dir.root, CoroutineScope(Dispatchers.Unconfined), Dispatchers.Unconfined) { now }

    /** What the next start reads back. */
    private fun restart() = runBlocking { saver().load() }

    private fun savedQueueId() = restart()!!.first.queueId

    @Test fun theNextStartGetsTheQueueAndTheStateBack() {
        val list = tracks(3)
        queue.play(list, startAt = 1, from = "Album")
        queue.addToQueue(track("u"))
        saver.save(queue, endless = false, position = 12.5)
        val (state, saved) = restart()!!
        assertEquals(SavedState(state.queueId, list[1], 1, 12.5, Repeat.Off, false), state)
        assertEquals(SavedQueue(state.queueId, 0, list, listOf(0, 1, 2), listOf(track("u")), false, "Album"), saved)
    }

    @Test fun theQueueFileIsRewrittenOnlyWhenTheQueueChanges() {
        queue.play(tracks(3))
        saver.save(queue, endless = false, position = 0.0)
        val first = savedQueueId()

        queue.select(2)
        saver.save(queue, endless = false, position = 4.0)
        assertEquals(first, savedQueueId())
        assertEquals(4.0, restart()!!.first.position, 1e-9)

        queue.addToQueue(track("u"))
        saver.save(queue, endless = false, position = 0.0)
        val queued = savedQueueId()
        assertNotEquals(first, queued)
        assertEquals(listOf(track("u")), restart()!!.second!!.userQueue)

        saver.save(queue, endless = true, position = 0.0)
        assertNotEquals(queued, savedQueueId())
        assertTrue(restart()!!.second!!.endless)

        queue.play(listOf(track("x")))
        saver.save(queue, endless = true, position = 0.0)
        assertEquals(listOf(track("x")), restart()!!.second!!.original)
    }

    @Test fun theQueueFileIsRewrittenWhenThePlayingIndexLeavesItsWindow() {
        queue.play(tracks(300))
        saver.save(queue, endless = true, position = 0.0)
        val first = savedQueueId()

        queue.select(150)
        saver.save(queue, endless = true, position = 0.0)
        assertEquals(first, savedQueueId())

        queue.select(250)
        saver.save(queue, endless = true, position = 0.0)
        val (state, saved) = restart()!!
        assertNotEquals(first, state.queueId)
        assertEquals(250, state.index)
        assertEquals(200, saved!!.offset)
    }

    @Test fun aQueueThatCannotBeWrittenKeepsStatesWithThePreviousOneAndIsRetried() {
        queue.play(tracks(3))
        saver.save(queue, endless = false, position = 1.0)
        val before = restart()

        // Not empty, so the store's cleanup of a failed write cannot remove it.
        val blocker = File(dir.root, "playback-queue.tmp").apply { mkdir(); resolve("x").createNewFile() }
        queue.addToQueue(track("u"))
        saver.save(queue, endless = false, position = 2.0)
        saver.save(queue, endless = false, position = 3.0)
        assertEquals(before, restart())

        blocker.deleteRecursively()
        saver.save(queue, endless = false, position = 4.0)
        val (state, saved) = restart()!!
        assertEquals(4.0, state.position, 1e-9)
        assertEquals(listOf(track("u")), saved!!.userQueue)
    }

    @Test fun nothingPlayingClearsTheSave() {
        queue.play(tracks(2))
        saver.save(queue, endless = false, position = 0.0)
        queue.clear()
        saver.save(queue, endless = false, position = 0.0)
        assertNull(restart())
    }

    @Test fun aSaveIsDueFiveSecondsAfterTheLastOne() {
        queue.play(tracks(1))
        saver.save(queue, endless = false, position = 0.0)
        now += 5000
        assertFalse(saver.due)
        now += 1
        assertTrue(saver.due)
    }
}
