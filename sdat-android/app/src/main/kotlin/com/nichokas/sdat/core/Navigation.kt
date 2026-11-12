package com.nichokas.sdat.core

enum class AppTab { Discover, Search, Collection }

sealed interface Route {
    data class Album(val id: String) : Route
    data class Playlist(val id: String) : Route
    data class Artist(val id: String) : Route
    data class Explore(val path: String, val title: String) : Route
    data object CollectionTracks : Route
    data object CollectionAlbums : Route
    data object CollectionArtists : Route
    data object CollectionPlaylists : Route
    data object Downloads : Route
    data object Settings : Route
}

/** A pushed page. The id keeps two pushes of the same route apart, so each keeps its own saved state. */
data class Destination(val id: Long, val route: Route)

/** The selected tab and one back stack per tab; a tab's root is implicit, so an empty stack means the root. */
data class NavState(
    val tab: AppTab = AppTab.Discover,
    private val stacks: Map<AppTab, List<Destination>> = emptyMap(),
    private val nextId: Long = 0,
) {
    val stack: List<Destination> get() = stacks[tab].orEmpty()
    val top: Route? get() = stack.lastOrNull()?.route
    val inDetail: Boolean get() = stack.isNotEmpty()
    val inSettings: Boolean get() = top == Route.Settings

    fun push(route: Route) = copy(stacks = stacks + (tab to stack + Destination(nextId, route)), nextId = nextId + 1)

    fun pop() = if (inDetail) copy(stacks = stacks + (tab to stack.dropLast(1))) else this

    /** Re-selecting the current tab returns it to its root, as on iOS; another tab keeps its stack. */
    fun select(tab: AppTab) = if (tab == this.tab) copy(stacks = stacks - tab) else copy(tab = tab)
}
