package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NavigationTest {
    private val NavState.routes get() = stack.map { it.route }

    @Test
    fun startsAtTheDiscoverRoot() {
        val nav = NavState()
        assertEquals(AppTab.Discover, nav.tab)
        assertNull(nav.top)
        assertFalse(nav.inDetail)
    }

    @Test
    fun pushAddsToTheCurrentTab() {
        val nav = NavState().push(Route.Album("1")).push(Route.Artist("2"))
        assertEquals(listOf(Route.Album("1"), Route.Artist("2")), nav.routes)
        assertEquals(Route.Artist("2"), nav.top)
        assertTrue(nav.inDetail)
    }

    @Test
    fun popRemovesTheTopPage() {
        val nav = NavState().push(Route.Album("1")).push(Route.Artist("2")).pop()
        assertEquals(listOf(Route.Album("1")), nav.routes)
    }

    @Test
    fun popAtTheRootChangesNothing() {
        val nav = NavState(tab = AppTab.Search)
        assertEquals(nav, nav.pop())
    }

    @Test
    fun switchingTabsKeepsEachStack() {
        val nav = NavState()
            .push(Route.Album("1"))
            .select(AppTab.Collection)
            .push(Route.CollectionTracks)
        assertEquals(listOf(Route.CollectionTracks), nav.routes)
        val back = nav.select(AppTab.Discover)
        assertEquals(listOf(Route.Album("1")), back.routes)
        assertEquals(listOf(Route.CollectionTracks), back.select(AppTab.Collection).routes)
    }

    @Test
    fun reselectingTheActiveTabReturnsToItsRoot() {
        val nav = NavState()
            .push(Route.Album("1"))
            .select(AppTab.Search)
            .push(Route.Artist("2"))
            .push(Route.Album("3"))
            .select(AppTab.Search)
        assertEquals(AppTab.Search, nav.tab)
        assertFalse(nav.inDetail)
        assertEquals(listOf(Route.Album("1")), nav.select(AppTab.Discover).routes)
    }

    @Test
    fun theSameRoutePushedTwiceGetsDistinctDestinations() {
        val stack = NavState().push(Route.Album("1")).push(Route.Album("1")).stack
        assertEquals(2, stack.size)
        assertNotEquals(stack[0].id, stack[1].id)
    }

    @Test
    fun destinationsStayDistinctAcrossTabs() {
        val nav = NavState().push(Route.Album("1")).select(AppTab.Search).push(Route.Album("1"))
        assertNotEquals(nav.stack.single().id, nav.select(AppTab.Discover).stack.single().id)
    }

    @Test
    fun settingsOnTopIsInSettings() {
        val nav = NavState().push(Route.Settings)
        assertTrue(nav.inSettings)
        assertFalse(nav.push(Route.Downloads).inSettings)
        assertFalse(nav.pop().inSettings)
    }
}
