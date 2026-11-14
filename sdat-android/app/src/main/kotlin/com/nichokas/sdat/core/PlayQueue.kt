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
    /** The base queue in playing order: [order] applied to [original]. */
    var tracks by mutableStateOf(emptyList<T>())
        private set
    var index by mutableStateOf<Int?>(null)
        private set
    /** Differs from [index]'s track while a user-queue track plays. */
    var current by mutableStateOf<T?>(null)
        private set
    var userQueue by mutableStateOf(emptyList<T>())
        private set
    private var playingUser = false
    var shuffle by mutableStateOf(false)
        private set
    var repeat by mutableStateOf(Repeat.Off)
        private set
    var source by mutableStateOf<String?>(null)
        private set

    /** The base queue in its unshuffled order, restored when shuffle is turned off. */
    var original = emptyList<T>()
        private set
    /** Each playing position's place in [original], so a track queued twice maps back to its own occurrence. */
    private var order = emptyList<Int>()
    private val recentIDs = HashSet<String>()
    private val recentOrder = ArrayDeque<String>()

    val upNext get() = index?.let { tracks.drop(it + 1) } ?: tracks
    val needsRefill get() = index?.let { tracks.size - it <= 5 } ?: false

    /** What a natural end plays next, without taking it: [advance]'s pick, except that repeat-one only loops the
     * current track, so there is nothing to preload. */
    val preloadable: T?
        get() = when {
            repeat == Repeat.One -> null
            userQueue.isNotEmpty() -> userQueue.first()
            else -> index?.let { i -> tracks.getOrNull(i + 1) ?: tracks.firstOrNull()?.takeIf { repeat == Repeat.All } }
        }

    /** With shuffle on, the started track first and the rest shuffled. */
    fun play(list: List<T>, startAt: Int = 0, from: String? = null): T? {
        if (startAt !in list.indices) return null
        clearRecent()
        val rest = list.indices.filter { it != startAt }
        setBase(list, if (shuffle) listOf(startAt) + rest.shuffled(random) else list.indices.toList())
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
        setBase(list, list.indices.toList())
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
            playingUser = true
            remember(listOf(id(it)))
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
        playingUser = false
        return tracks[i].also { current = it }
    }

    /** The track before the current one: the base track a Play Next track followed, else the one before it. */
    fun previous(): T? {
        val i = index ?: return null
        return if (playingUser) select(i) else select(i - 1)
    }

    fun jumpToUpNext(offset: Int): T? = index?.let { select(it + 1 + offset) }

    /** Plays that user-queue track, dropping the ones before it. */
    fun jumpToUser(offset: Int): T? {
        val t = userQueue.getOrNull(offset) ?: return null
        userQueue = userQueue.drop(offset + 1)
        current = t
        playingUser = true
        remember(listOf(id(t)))
        return t
    }

    fun removeUpNext(offset: Int) {
        val at = (index ?: return) + 1 + offset
        if (at in order.indices) removePlaying(setOf(at))
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
            setBase(original, order.take(i + 1) + order.drop(i + 1).shuffled(random))
        } else {
            index = order[i]
            setBase(original, original.indices.toList())
        }
    }

    fun cycleRepeat() {
        repeat = when (repeat) {
            Repeat.Off -> Repeat.All
            Repeat.All -> Repeat.One
            Repeat.One -> Repeat.Off
        }
    }

    /** Appends the batch's tracks not queued (Play Next included) or recently played; returns whether any were new. */
    fun append(batch: List<T>): Boolean {
        remember(tracks.map(id))
        // Play Next tracks still waiting sit outside the base queue; played ones are remembered when they start.
        val ids = HashSet(recentIDs).apply { userQueue.forEach { add(id(it)) } }
        val fresh = batch.filter { ids.add(id(it)) }
        if (fresh.isEmpty()) return false
        setBase(original + fresh, order + (original.size until original.size + fresh.size))
        remember(fresh.map(id))
        trimPlayedHistory()
        return true
    }

    /** The queues cut to a window around the playing track, as iOS saves them; the unshuffled order keeps just the
     * window's tracks, so every saved track can be unshuffled. */
    fun saved(id: Int, endless: Boolean): SavedQueue<T> {
        val w = savedWindow(tracks.size, index, endless)
        val kept = order.slice(w).sorted()
        val rank = kept.withIndex().associate { (r, p) -> p to r }
        return SavedQueue(id, w.first, kept.map(original::get), order.slice(w).map(rank::getValue), userQueue.take(MAX_SAVED), endless, source)
    }

    /** iOS restore: without its queue, or with one whose window lacks the saved index, the saved track is the whole
     * queue. */
    fun restore(saved: SavedQueue<T>?, state: SavedState<T>) {
        clear()
        val at = saved?.let { q -> state.index?.minus(q.offset)?.takeIf { it in q.order.indices } }
        if (saved == null || at == null) {
            setBase(listOf(state.current), listOf(0))
            index = 0
        } else {
            setBase(saved.original, saved.order)
            userQueue = saved.userQueue
            source = saved.source
            index = at
        }
        current = state.current
        // The saved state doesn't say which queue the track came from; a base track of the same id is the likely one.
        playingUser = tracks.getOrNull(index ?: -1)?.let(id) != id(state.current)
        remember(tracks.map(id))
        repeat = state.repeat
        shuffle = state.shuffle
    }

    /** Forgets the queues; shuffle and repeat stay, like iOS. */
    fun clear() {
        clearRecent()
        setBase(emptyList(), emptyList())
        userQueue = emptyList()
        index = null
        current = null
        playingUser = false
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
        removePlaying((0 until drop).toSet())
        index = i - drop
    }

    private fun setBase(list: List<T>, playing: List<Int>) {
        original = list
        order = playing
        tracks = playing.map(list::get)
    }

    /** Drops these playing positions from both orders. */
    private fun removePlaying(at: Set<Int>) {
        val gone = at.map(order::get).toSet()
        val moved = IntArray(original.size)
        var kept = 0
        for (p in original.indices) if (p !in gone) moved[p] = kept++
        setBase(original.filterIndexed { p, _ -> p !in gone }, order.filterIndexed { j, _ -> j !in at }.map { moved[it] })
    }

    companion object {
        const val MAX_PLAYED_HISTORY = 300
        const val MAX_RECENT_IDS = 2000
        const val DISCOVER = "Discover"
        const val MAX_SAVED = 2000
    }
}

private const val ENDLESS_BEHIND = 50
private const val ENDLESS_AHEAD = 200

/** iOS PlaybackStore.window: the part of a base queue of [count] tracks worth saving around [index]. An endless
 * queue keeps a little history and a few batches ahead; any other queue up to [PlayQueue.MAX_SAVED] tracks. */
internal fun savedWindow(count: Int, index: Int?, endless: Boolean): IntRange {
    val i = (index ?: 0).coerceIn(0, maxOf(count - 1, 0))
    val behind = if (endless) ENDLESS_BEHIND else PlayQueue.MAX_SAVED / 4
    val ahead = if (endless) ENDLESS_AHEAD else PlayQueue.MAX_SAVED - behind - 1
    var lo = maxOf(0, i - behind)
    var hi = minOf(count, i + ahead + 1)
    if (!endless && hi - lo < PlayQueue.MAX_SAVED) {
        lo = maxOf(0, minOf(lo, hi - PlayQueue.MAX_SAVED))
        hi = minOf(count, maxOf(hi, lo + PlayQueue.MAX_SAVED))
    }
    return lo until hi
}
