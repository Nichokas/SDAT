package com.nichokas.sdat.core

/** One page of a tab's back stack: the tab's root, or a pushed [Destination]. */
sealed interface PageKey {
    data class Root(val tab: AppTab) : PageKey
    data class Pushed(val id: Long) : PageKey
}

/**
 * What each page keeps while it is on a back stack, as a SwiftUI navigation stack keeps a page's state: a pushed page
 * takes the one below out of composition, and popping back must find its loaded data, not fetch it again. A page's
 * values go once it leaves every stack, and [AutoCloseable] ones are closed then, so their loads stop.
 */
class PageStore {
    private class Slot(val inputs: List<Any?>, val value: Any)

    private val pages = HashMap<PageKey, HashMap<Long, Slot>>()
    private var live: Set<PageKey> = NavState().pages

    /** The value [page] keeps in [slot], made again by [init] when [inputs] changed. */
    fun <T : Any> get(page: PageKey, slot: Long, inputs: List<Any?>, init: () -> T): T {
        // A popped page still composes while it animates out; what it asks for then is not kept.
        if (page !in live) return init()
        val slots = pages.getOrPut(page) { HashMap() }
        slots[slot]?.let { kept ->
            @Suppress("UNCHECKED_CAST")
            if (kept.inputs == inputs) return kept.value as T
            close(kept)
        }
        return init().also { slots[slot] = Slot(inputs, it) }
    }

    /** Drops the pages no longer on any of [nav]'s stacks. */
    fun keep(nav: NavState) {
        live = nav.pages
        val gone = pages.keys - live
        gone.forEach { page -> pages.remove(page)?.values?.forEach(::close) }
    }

    /** Sign-out: the next account starts with nothing, tab roots included. */
    fun clear() {
        pages.values.forEach { it.values.forEach(::close) }
        pages.clear()
        live = NavState().pages
    }

    private fun close(slot: Slot) = (slot.value as? AutoCloseable)?.close()
}
