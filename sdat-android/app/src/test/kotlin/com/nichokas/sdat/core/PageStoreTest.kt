package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class PageStoreTest {
    private class Model : AutoCloseable {
        var closed = false
        override fun close() { closed = true }
    }

    private val search = PageKey.Root(AppTab.Search)
    private fun PageStore.model(page: PageKey, slot: Long = 1, inputs: List<Any?> = emptyList()) = get(page, slot, inputs) { Model() }

    @Test fun aPageFindsItsValueWhenItComposesAgain() {
        val store = PageStore()
        val first = store.model(search, inputs = listOf("daft"))
        assertSame(first, store.model(search, inputs = listOf("daft")))
    }

    @Test fun callSitesAndPagesKeepApart() {
        val store = PageStore()
        val nav = NavState(tab = AppTab.Search).push(Route.Album("1"))
        store.keep(nav)
        val album = PageKey.Pushed(nav.stack.single().id)
        val a = store.model(search, slot = 1)
        assertNotSame(a, store.model(search, slot = 2))
        assertNotSame(a, store.model(album, slot = 1))
    }

    @Test fun newInputsReplaceAndCloseTheOldValue() {
        val store = PageStore()
        val daft = store.model(search, inputs = listOf("daft"))
        val punk = store.model(search, inputs = listOf("punk"))
        assertNotSame(daft, punk)
        assertTrue(daft.closed)
        assertSame(punk, store.model(search, inputs = listOf("punk")))
    }

    @Test fun poppingAPageDropsAndClosesItsValues() {
        val store = PageStore()
        val nav = NavState().push(Route.Playlist("1"))
        store.keep(nav)
        val page = PageKey.Pushed(nav.stack.single().id)
        val playlist = store.model(page)
        val root = store.model(PageKey.Root(AppTab.Discover))
        store.keep(nav.pop())
        assertTrue(playlist.closed)
        assertFalse(root.closed)
        assertSame(root, store.model(PageKey.Root(AppTab.Discover)))
    }

    @Test fun pagesOnOtherTabsSurviveATabSwitch() {
        val store = PageStore()
        val nav = NavState().push(Route.Album("1"))
        store.keep(nav)
        val page = PageKey.Pushed(nav.stack.single().id)
        val album = store.model(page)
        store.keep(nav.select(AppTab.Search))
        assertSame(album, store.model(page))
    }

    @Test fun aPoppedPageAnimatingOutIsNotKept() {
        val store = PageStore()
        val nav = NavState().push(Route.Album("1"))
        val page = PageKey.Pushed(nav.stack.single().id)
        store.keep(nav)
        store.keep(nav.pop())
        assertNotSame(store.model(page), store.model(page))
    }

    @Test fun signOutClearsTabRootsToo() {
        val store = PageStore()
        val results = store.model(search)
        store.clear()
        assertTrue(results.closed)
        assertNotSame(results, store.model(search))
    }

    @Test fun navListsEveryRootAndPushedPage() {
        val nav = NavState().push(Route.Album("1")).select(AppTab.Search).push(Route.Artist("2"))
        assertEquals(
            AppTab.entries.map(PageKey::Root).toSet() + setOf(PageKey.Pushed(0), PageKey.Pushed(1)),
            nav.pages,
        )
    }
}
