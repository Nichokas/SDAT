package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.referentialEqualityPolicy
import androidx.compose.runtime.setValue
import kotlin.random.Random

enum class Repeat { Off, All, One }

internal enum class QueueSection { User, UpNext }

internal class QueueSlice<T>(val section: QueueSection, private val base: List<T>, private val start: Int = 0) {
    val tracks: List<T> = base.subList(start, base.size)

    // Equal track values can still represent different queue occurrences.
    override fun equals(other: Any?): Boolean =
        other is QueueSlice<*> && section == other.section && base === other.base && start == other.start

    override fun hashCode(): Int = 31 * (31 * section.hashCode() + System.identityHashCode(base)) + start
}

/** The iOS Player's queues without the engine: the base queue, the user queue (Play Next, Add to Queue) that plays
 * before the rest of it, shuffle and repeat, and the endless-queue dedupe (recentIDs/recentOrder).
 * Operations that change the playing track return it; the caller starts it. */
class PlayQueue<T>(private val random: Random = Random.Default, private val id: (T) -> String) {
    /** The base queue in playing order: [order] applied to [original]. */
    var tracks by mutableStateOf(emptyList<T>(), referentialEqualityPolicy())
        private set
    var index by mutableStateOf<Int?>(null)
        private set
    /** [index]'s track; a user-queue track joins the base queue when it starts. */
    var current by mutableStateOf<T?>(null)
        private set
    var userQueue by mutableStateOf(emptyList<T>(), referentialEqualityPolicy())
        private set
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

    private val futureStart get() = index?.let { it.coerceIn(-1, tracks.lastIndex) + 1 } ?: 0
    val upNext get() = tracks.let { it.subList(futureStart, it.size) }
    internal val upcomingSlice get() = QueueSlice(QueueSection.UpNext, tracks, futureStart)
    internal val userSlice get() = QueueSlice(QueueSection.User, userQueue)
    /** The base queue before the current track: repeat all plays it again after the last. */
    val played get() = index?.let { tracks.take(it) } ?: emptyList()
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

    /** Play buttons: turns shuffle off and starts at the first track. */
    fun playInOrder(list: List<T>, from: String? = null): T? {
        if (list.isEmpty()) return null
        shuffle = false
        return play(list, 0, from)
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

    fun playNext(list: List<T>) {
        userQueue = list + userQueue
    }

    fun insertUser(t: T, at: Int) {
        userQueue = userQueue.toMutableList().apply { add(at.coerceIn(0, size), t) }
    }

    fun addToQueue(list: List<T>) {
        userQueue = userQueue + list
    }

    /** What plays after the current track. [manual] (Next) skips past a repeat-one track, which only loops on its
     * natural end. Null at the end of the queue, changing nothing. */
    fun advance(manual: Boolean): T? {
        val c = current
        if (repeat == Repeat.One && !manual && c != null) return c
        userQueue.firstOrNull()?.let {
            userQueue = userQueue.drop(1)
            return join(it)
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

    fun previous(): T? = index?.let { select(it - 1) }

    fun jumpToUpNext(offset: Int): T? = index?.let { select(it + 1 + offset) }

    /** Plays that user-queue track, dropping the ones before it. */
    fun jumpToUser(offset: Int): T? {
        val t = userQueue.getOrNull(offset) ?: return null
        userQueue = userQueue.drop(offset + 1)
        return join(t)
    }

    /** Starts a user-queue track right after the current one in the base queue, in [original] too, so Previous and
     * Next find it again and turning shuffle off keeps it there. */
    private fun join(t: T): T {
        val at = index?.plus(1) ?: 0
        val o = order.getOrNull(at - 1)?.plus(1) ?: 0
        setBase(
            original.toMutableList().apply { add(o, t) },
            order.map { if (it >= o) it + 1 else it }.toMutableList().apply { add(at, o) },
        )
        return tracks[at].also { index = at; current = it }
    }

    internal fun move(slice: QueueSlice<T>, from: Int, to: Int): Boolean {
        if (!accepts(slice, from) || to !in slice.tracks.indices || from == to) return false
        when (slice.section) {
            QueueSection.User -> userQueue = userQueue.moving(from, to)
            QueueSection.UpNext -> {
                val start = futureStart
                val moved = order.moving(start + from, start + to)
                // Discover keeps the two orders aligned even when shuffle is enabled.
                if (order.indices.all { order[it] == it }) {
                    setBase(moved.map(original::get), order)
                } else {
                    setBase(original, moved)
                }
            }
        }
        return true
    }

    internal fun remove(slice: QueueSlice<T>, offset: Int): Boolean {
        if (!accepts(slice, offset)) return false
        when (slice.section) {
            QueueSection.User -> userQueue = userQueue.toMutableList().apply { removeAt(offset) }
            QueueSection.UpNext -> removePlaying(setOf(futureStart + offset))
        }
        return true
    }

    private fun accepts(slice: QueueSlice<T>, offset: Int): Boolean {
        val active = when (slice.section) {
            QueueSection.User -> userSlice
            QueueSection.UpNext -> upcomingSlice
        }
        return slice == active && offset in active.tracks.indices
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
        // Play Next tracks still waiting sit outside the base queue; played ones are in it.
        val ids = HashSet(recentIDs).apply { userQueue.forEach { add(id(it)) } }
        val fresh = batch.filter { ids.add(id(it)) }
        if (fresh.isEmpty()) return false
        setBase(original + fresh, order + (original.size until original.size + fresh.size))
        remember(fresh.map(id))
        trimPlayedHistory()
        return true
    }

    /** Appends to the base queue as given: unlike [append], a track already queued or played stays in. */
    fun extend(list: List<T>) {
        setBase(original + list, order + (original.size until original.size + list.size))
        remember(list.map(id))
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
        // Saved by a version that kept a playing Play Next track out of the base queue.
        if (tracks.getOrNull(index ?: -1)?.let(id) != id(state.current)) join(state.current) else current = state.current
        remember(tracks.map(id) + id(state.current))
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

private fun <T> List<T>.moving(from: Int, to: Int): List<T> =
    toMutableList().apply { add(to, removeAt(from)) }

private const val ENDLESS_BEHIND = 50
private const val ENDLESS_AHEAD = 200

/** Playback taken over from another device, as a saved queue for [PlayQueue.restore]: [current] alone, unshuffled;
 * what follows it arrives later. [discover]: Discover's endless queue. */
internal fun <T> resumed(current: T, position: Double, repeat: Repeat, discover: Boolean = false): Pair<SavedQueue<T>, SavedState<T>> =
    SavedQueue(0, 0, listOf(current), listOf(0), emptyList(), endless = discover, source = PlayQueue.DISCOVER.takeIf { discover }) to
        SavedState(0, current, 0, position, repeat, shuffle = false)

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
