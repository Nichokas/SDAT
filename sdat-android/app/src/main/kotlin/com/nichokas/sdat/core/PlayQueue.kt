package com.nichokas.sdat.core

/** The iOS Player's base queue and its endless-queue dedupe (recentIDs/recentOrder), without the engine. */
class PlayQueue<T>(private val id: (T) -> String) {
    var tracks: List<T> = emptyList()
        private set
    var index: Int? = null

    private val recentIDs = HashSet<String>()
    private val recentOrder = ArrayDeque<String>()

    val current get() = index?.let { tracks.getOrNull(it) }
    val next get() = index?.let { tracks.getOrNull(it + 1) }
    val needsRefill get() = index?.let { tracks.size - it <= 5 } ?: false

    /** Tapped track first, then the rest of the pool from the tapped one onwards, wrapping, each track once. */
    fun discover(track: T, pool: List<T>) {
        clearRecent()
        val seen = hashSetOf(id(track))
        val i = pool.indexOfFirst { id(it) == id(track) }.coerceAtLeast(0)
        tracks = listOf(track) + (pool.drop(i + 1) + pool.take(i)).filter { seen.add(id(it)) }
        index = 0
    }

    /** Appends the batch's tracks not queued or recently played; returns whether any were new. */
    fun append(batch: List<T>): Boolean {
        remember(tracks.map(id))
        val ids = HashSet(recentIDs)
        val fresh = batch.filter { ids.add(id(it)) }
        if (fresh.isEmpty()) return false
        tracks = tracks + fresh
        remember(fresh.map(id))
        trimPlayedHistory()
        return true
    }

    fun clear() {
        clearRecent()
        tracks = emptyList()
        index = null
    }

    private fun clearRecent() {
        recentIDs.clear()
        recentOrder.clear()
    }

    private fun remember(ids: List<String>) {
        for (i in ids) if (recentIDs.add(i)) recentOrder.addLast(i)
        while (recentOrder.size > MAX_RECENT_IDS) recentIDs.remove(recentOrder.removeFirst())
    }

    private fun trimPlayedHistory() {
        val i = index ?: return
        if (i <= MAX_PLAYED_HISTORY) return
        val drop = i - MAX_PLAYED_HISTORY
        tracks = tracks.drop(drop)
        index = i - drop
    }

    companion object {
        const val MAX_PLAYED_HISTORY = 300
        const val MAX_RECENT_IDS = 2000
    }
}
