package com.nichokas.sdat.core

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PagedTest {
    @Test fun appendsPagesUntilThePagerRunsDry() = runBlocking {
        val pages = ArrayDeque(listOf(listOf(1, 2), listOf(3)))
        val loader = PagedLoader(this) { pages.removeFirstOrNull() }
        loader.more()?.join()
        loader.more()?.join()
        assertFalse(loader.list.done)
        loader.more()?.join()
        assertEquals(listOf(1, 2, 3), loader.list.items)
        assertTrue(loader.list.done)
        assertNull(loader.more())
    }

    @Test fun anEmptyPageEndsTheList() = runBlocking {
        val loader = PagedLoader(this) { emptyList<Int>() }
        loader.more()?.join()
        assertTrue(loader.list.isEmpty)
    }

    @Test fun neverLoadsTwoPagesAtOnce() = runBlocking {
        var calls = 0
        val gate = CompletableDeferred<List<Int>>()
        val loader = PagedLoader(this) { calls++; gate.await() }
        val first = loader.more()
        assertNull(loader.more())
        gate.complete(listOf(1))
        first?.join()
        assertEquals(1, calls)
        assertEquals(listOf(1), loader.list.items)
    }

    @Test fun anErrorStopsLoadingUntilRetried() = runBlocking {
        var fail = true
        val loader = PagedLoader(this) { if (fail) throw IllegalStateException() else listOf(1) }
        loader.more()?.join()
        assertEquals("Something went wrong. Please try again.", loader.list.error)
        assertNull(loader.more())
        fail = false
        loader.retry()?.join()
        assertNull(loader.list.error)
        assertEquals(listOf(1), loader.list.items)
    }

    @Test fun asksForMoreWithinTheLastFiveRows() {
        val list = PagedList(items = (1..20).toList())
        assertFalse(list.wantsMore(14))
        assertTrue(list.wantsMore(15))
        assertFalse(list.copy(done = true).wantsMore(19))
        assertFalse(list.started().wantsMore(19))
    }

    @Test fun closeBeforeMoreNeverCallsNext() = runBlocking {
        var calls = 0
        var closes = 0
        val loader = PagedLoader(this, pager = AutoCloseable { closes++ }) { calls++; listOf(1) }
        loader.close()
        loader.close()
        assertNull(loader.more())
        assertNull(loader.retry())
        assertEquals(0, calls)
        assertEquals(1, closes)
        assertFalse(loader.list.loading)
        assertNull(loader.list.error)
    }

    @Test fun closeWhileMoreIsQueuedNeverCallsNext() = runBlocking {
        var calls = 0
        val loader = PagedLoader(this) { calls++; listOf(1) }
        val job = loader.more()
        loader.close()
        job?.join()
        assertEquals(0, calls)
        assertFalse(loader.list.loading)
        assertNull(loader.list.error)
    }

    @Test fun closeWhileInFlightDropsLateResultAndError() = runBlocking {
        val resultGate = CompletableDeferred<List<Int>>()
        val okLoader = PagedLoader(this) { resultGate.await() }
        val okJob = okLoader.more()
        yield()
        okLoader.close()
        resultGate.complete(listOf(1, 2))
        okJob?.join()
        assertEquals(emptyList<Int>(), okLoader.list.items)
        assertFalse(okLoader.list.loading)
        assertNull(okLoader.list.error)

        val failGate = CompletableDeferred<List<Int>>()
        val failLoader = PagedLoader(this) { failGate.await() }
        val failJob = failLoader.more()
        yield()
        failLoader.close()
        failGate.completeExceptionally(IllegalStateException("closed"))
        failJob?.join()
        assertFalse(failLoader.list.loading)
        assertNull(failLoader.list.error)
    }
}
