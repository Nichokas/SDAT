package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.random.Random

enum class Repeat { Off, All, One }

/** The iOS Player's queues without the engine: the base queue, the user queue (Play Next, Add to Queue) that plays
 * before the rest of it, shuffle and repeat, and the endless-queue dedupe (recentIDs/recentOrder).
 * Operations that change the playing track return it; the caller starts it. */
class PlayQueue<T>(private val random: Random = Random.Default, private val id: (T) -> String) {
    var tracks by mutableStateOf(emptyList<T>())
        private set
    var index by mutableStateOf<Int?>(null)
        private set
    /** Differs from [index]'s track while a user-queue track plays. */
    var current by mutableStateOf<T?>(null)
        private set
    var userQueue by mutableStateOf(emptyList<T>())
        private set
    var shuffle by mutableStateOf(false)
        private set
    var repeat by mutableStateOf(Repeat.Off)
        private set
    var source by mutableStateOf<String?>(null)
        private set

    /** The base queue in its unshuffled order, restored when shuffle is turned off. */
    private var original = emptyList<T>()
    private val recentIDs = HashSet<String>()
    private val recentOrder = ArrayDeque<String>()

    val upNext get() = index?.let { tracks.drop(it + 1) } ?: tracks
    val needsRefill get() = index?.let { tracks.size - it <= 5 } ?: false

    /** With shuffle on, the started track first and the rest shuffled. */
    fun play(list: List<T>, startAt: Int = 0, from: String? = null): T? {
        if (startAt !in list.indices) return null
        clearRecent()
        original = list
        tracks = if (shuffle) listOf(list[startAt]) + (list.take(startAt) + list.drop(startAt + 1)).shuffled(random) else list
        source = from
        return select(if (shuffle) 0 else startAt)
    }

    /** Shuffle buttons: turns shuffle on and starts at a random track. */
    fun playShuffled(list: List<T>, from: String? = null): T? {
        if (list.isEmpty()) return null
        shuffle = true
        return play(list, random.nextInt(list.size), from)
    }

    /** Tapped track first, then the rest of the pool from the tapped one onwards, wrapping, each track once. */
    fun discover(track: T, pool: List<T>): T {
        clearRecent()
        val seen = hashSetOf(id(track))
        val i = pool.indexOfFirst { id(it) == id(track) }.coerceAtLeast(0)
        startEndless(listOf(track) + (pool.drop(i + 1) + pool.take(i)).filter { seen.add(id(it)) })
        return track
    }

    /** The first batch of Discover's endless shuffle. */
    fun startInfinite(batch: List<T>): T? {
        if (batch.isEmpty()) return null
        clearRecent()
        startEndless(batch)
        return current
    }

    private fun startEndless(list: List<T>) {
        original = list
        tracks = list
        source = DISCOVER
        select(0)
    }

    fun playNext(t: T) {
        userQueue = listOf(t) + userQueue
    }

    fun addToQueue(t: T) {
        userQueue = userQueue + t
    }

    /** What plays after the current track. [manual] (Next) skips past a repeat-one track, which only loops on its
     * natural end. Null at the end of the queue, changing nothing. */
    fun advance(manual: Boolean): T? {
        val c = current
        if (repeat == Repeat.One && !manual && c != null) return c
        userQueue.firstOrNull()?.let {
            userQueue = userQueue.drop(1)
            current = it
            return it
        }
        val i = index ?: return null
        return when {
            i + 1 < tracks.size -> select(i + 1)
            repeat == Repeat.All && tracks.isNotEmpty() -> select(0)
            else -> null
        }
    }

    fun select(i: Int): T? {
        if (i !in tracks.indices) return null
        index = i
        return tracks[i].also { current = it }
    }

    fun jumpToUpNext(offset: Int): T? = index?.let { select(it + 1 + offset) }

    /** Plays that user-queue track, dropping the ones before it. */
    fun jumpToUser(offset: Int): T? {
        val t = userQueue.getOrNull(offset) ?: return null
        userQueue = userQueue.drop(offset + 1)
        current = t
        return t
    }

    fun removeUpNext(offset: Int) {
        val at = (index ?: return) + 1 + offset
        tracks = tracks.filterIndexed { j, _ -> j != at }
    }

    fun removeUser(offset: Int) {
        userQueue = userQueue.filterIndexed { j, _ -> j != offset }
    }

    /** [reorder] is false for an endless queue: its order is the source's, and shuffle applies to the next list. */
    fun setShuffle(on: Boolean, reorder: Boolean) {
        if (on == shuffle) return
        shuffle = on
        val i = index ?: return
        if (!reorder) return
        if (on) {
            tracks = tracks.take(i + 1) + tracks.drop(i + 1).shuffled(random)
        } else {
            val at = original.indexOfFirst { id(it) == id(tracks[i]) }
            if (at < 0) return
            tracks = original
            index = at
        }
    }

    fun cycleRepeat() {
        repeat = when (repeat) {
            Repeat.Off -> Repeat.All
            Repeat.All -> Repeat.One
            Repeat.One -> Repeat.Off
        }
    }

    /** Appends the batch's tracks not queued or recently played; returns whether any were new. */
    fun append(batch: List<T>): Boolean {
        remember(tracks.map(id))
        val ids = HashSet(recentIDs)
        val fresh = batch.filter { ids.add(id(it)) }
        if (fresh.isEmpty()) return false
        val mirrored = original == tracks
        tracks = tracks + fresh
        original = if (mirrored) tracks else original + fresh
        remember(fresh.map(id))
        trimPlayedHistory()
        return true
    }

    /** Forgets the queues; shuffle and repeat stay, like iOS. */
    fun clear() {
        clearRecent()
        tracks = emptyList()
        original = emptyList()
        userQueue = emptyList()
        index = null
        current = null
        source = null
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
        val mirrored = original == tracks
        tracks = tracks.drop(drop)
        if (mirrored) original = tracks
        index = i - drop
    }

    companion object {
        const val MAX_PLAYED_HISTORY = 300
        const val MAX_RECENT_IDS = 2000
        const val DISCOVER = "Discover"
    }
}
