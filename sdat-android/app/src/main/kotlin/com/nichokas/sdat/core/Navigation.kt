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

    /** Strings only, so it fits the saved instance state and survives process death; ids stay, as they key saved pages. */
    fun saved(): List<String> = buildList {
        add(tab.name)
        add(nextId.toString())
        for ((t, stack) in stacks) for (d in stack) {
            add(t.name)
            add(d.id.toString())
            addAll(d.route.fields())
        }
    }

    companion object {
        private const val HEADER = 2
        private const val ENTRY = 5

        /** A list saved by another app version may not parse; it then falls back to the Discover root. */
        fun restore(saved: List<String>): NavState = runCatching {
            val stacks = saved.drop(HEADER).chunked(ENTRY)
                .groupBy({ AppTab.valueOf(it[0]) }, { Destination(it[1].toLong(), route(it[2], it[3], it[4])) })
            NavState(AppTab.valueOf(saved[0]), stacks, saved[1].toLong())
        }.getOrDefault(NavState())
    }
}

private fun Route.fields(): List<String> = when (this) {
    is Route.Album -> listOf("album", id, "")
    is Route.Playlist -> listOf("playlist", id, "")
    is Route.Artist -> listOf("artist", id, "")
    is Route.Explore -> listOf("explore", path, title)
    Route.CollectionTracks -> listOf("collectionTracks", "", "")
    Route.CollectionAlbums -> listOf("collectionAlbums", "", "")
    Route.CollectionArtists -> listOf("collectionArtists", "", "")
    Route.CollectionPlaylists -> listOf("collectionPlaylists", "", "")
    Route.Downloads -> listOf("downloads", "", "")
    Route.Settings -> listOf("settings", "", "")
}

private fun route(kind: String, a: String, b: String): Route = when (kind) {
    "album" -> Route.Album(a)
    "playlist" -> Route.Playlist(a)
    "artist" -> Route.Artist(a)
    "explore" -> Route.Explore(a, b)
    "collectionTracks" -> Route.CollectionTracks
    "collectionAlbums" -> Route.CollectionAlbums
    "collectionArtists" -> Route.CollectionArtists
    "collectionPlaylists" -> Route.CollectionPlaylists
    "downloads" -> Route.Downloads
    "settings" -> Route.Settings
    else -> error("Unknown saved route $kind")
}
