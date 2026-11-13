package com.nichokas.sdat.core

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SearchTest {
    private class FakeSearch {
        val calls = mutableListOf<String>()
        val gates = mutableMapOf<String, CompletableDeferred<String>>()
        suspend fun run(q: String): String {
            calls += q
            return gates.getOrPut(q) { CompletableDeferred("result:$q") }.await()
        }
    }

    @Test fun searchesTheTrimmedQueryAfterTheDebounce() = runBlocking {
        val fake = FakeSearch()
        val model = SearchModel(debounceMs = 0, search = fake::run)
        model.edit("  daft ")
        model.settle()
        assertEquals(listOf("daft"), fake.calls)
        assertEquals("daft", model.state.shown)
        assertEquals("result:daft", model.state.result)
        assertFalse(model.state.searching)
    }

    @Test fun anEditDuringTheDebounceCancelsTheSearch() = runBlocking {
        val fake = FakeSearch()
        val model = SearchModel(debounceMs = 10_000, search = fake::run)
        model.edit("da")
        val pending = launch { model.settle() }
        yield()
        pending.cancel()
        model.edit("daft")
        assertTrue(fake.calls.isEmpty())
    }

    @Test fun oneLetterDoesNotSearch() = runBlocking {
        val fake = FakeSearch()
        val model = SearchModel(debounceMs = 0, search = fake::run)
        model.edit("d")
        model.settle()
        assertTrue(fake.calls.isEmpty())
        assertEquals("", model.state.shown)
    }

    @Test fun clearingTheFieldDropsResultsAndErrors() = runBlocking {
        val model = SearchModel(debounceMs = 0) { q: String -> if (q == "bad") error("boom") else q }
        model.edit("daft")
        model.settle()
        model.edit("")
        model.settle()
        assertEquals("", model.state.shown)
        assertNull(model.state.result)
        assertNull(model.state.error)
    }

    @Test fun comingBackToASettledQueryDoesNotSearchAgain() = runBlocking {
        val fake = FakeSearch()
        val model = SearchModel(debounceMs = 0, search = fake::run)
        model.edit("daft")
        model.settle()
        model.settle()
        assertEquals(listOf("daft"), fake.calls)
    }

    @Test fun onlyTheLatestSearchWritesItsResult() = runBlocking {
        val fake = FakeSearch()
        fake.gates["daf"] = CompletableDeferred()
        fake.gates["daft"] = CompletableDeferred()
        val model = SearchModel(debounceMs = 0, search = fake::run)
        model.edit("daf")
        // The FFI call cannot be cancelled, so the first search is left running.
        launch { model.settle() }
        yield()
        model.edit("daft")
        val second = launch { model.settle() }
        yield()
        fake.gates.getValue("daft").complete("new")
        second.join()
        fake.gates.getValue("daf").complete("stale")
        yield()
        assertEquals("new", model.state.result)
        assertEquals("daft", model.state.shown)
    }

    @Test fun theFirstSearchShowsItsQueryAtOnceAndLaterOnesKeepTheOldResults() {
        val first = SearchState<String>(query = "daft").started("daft")
        assertEquals("daft", first.shown)
        assertTrue(first.searching)
        val settled = first.finished("daft", first.generation, "a")
        val next = settled.copy(query = "daft punk").started("daft punk")
        assertEquals("daft", next.shown)
        assertEquals("a", next.result)
    }

    @Test fun aFailureShowsTheErrorAndKeepsTheOldResults() = runBlocking {
        val model = SearchModel(debounceMs = 0) { q: String -> if (q == "bad") throw IllegalStateException() else q }
        model.edit("daft")
        model.settle()
        model.edit("bad")
        model.settle()
        assertEquals("bad", model.state.shown)
        assertEquals("daft", model.state.result)
        assertEquals("Something went wrong. Please try again.", model.state.error)
        assertFalse(model.state.searching)
    }

    @Test fun theTopArtistNeedsANameMatch() {
        assertTrue(isTopArtist("Daft Punk", "daft"))
        assertTrue(isTopArtist("Daft Punk", "daft punk discovery"))
        assertFalse(isTopArtist("Justice", "daft"))
        assertFalse(isTopArtist("", "daft"))
    }
}
