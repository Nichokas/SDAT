package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch

/** An infinite list over a tonearm pager: the items so far, whether the pager ran dry, and the error that stopped it. */
data class PagedList<T>(
    val items: List<T> = emptyList(),
    val done: Boolean = false,
    val loading: Boolean = false,
    val error: String? = null,
) {
    /** Never two loads at once, never past the end, and never over an error until it is retried. */
    val canLoad: Boolean get() = !loading && !done && error == null
    val isEmpty: Boolean get() = done && items.isEmpty()

    /** As on iOS, a row among the last five coming on screen asks for the next page. */
    fun wantsMore(visibleIndex: Int) = canLoad && visibleIndex >= items.size - 5

    fun started() = copy(loading = true)

    /** The pager reports its end with null or an empty page. */
    fun loaded(page: List<T>?) = if (page.isNullOrEmpty()) copy(loading = false, done = true) else copy(items = items + page, loading = false)

    fun failed(message: String) = copy(loading = false, error = message)

    fun retried() = copy(error = null)
}

// Loads outlive the row that asked: cancelling a pager's call midway could lose the page it already advanced past.
private val loadScope = MainScope()

/** Drives a [PagedList] from a pager's `nextPage`. */
class PagedLoader<T>(private val scope: CoroutineScope = loadScope, private val next: suspend () -> List<T>?) {
    var list by mutableStateOf(PagedList<T>())
        private set

    /** Loads the next page unless one is loading, the end was reached, or an error waits for [retry]. */
    fun more(): Job? {
        if (!list.canLoad) return null
        list = list.started()
        return scope.launch {
            try {
                val page = next()
                list = list.loaded(page)
            } catch (e: CancellationException) {
                list = list.copy(loading = false)
                throw e
            } catch (e: Exception) {
                list = list.failed(friendlyMessage(e))
            }
        }
    }

    fun retry(): Job? {
        list = list.retried()
        return more()
    }
}
