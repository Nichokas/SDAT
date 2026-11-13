package com.nichokas.sdat.core

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.runBlocking
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
}
