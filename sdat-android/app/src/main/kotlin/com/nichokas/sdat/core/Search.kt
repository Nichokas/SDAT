package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.delay

/** What a query edit asks the search screen to do. */
sealed interface QueryPlan {
    /** The field is empty: back to Explore. */
    data object Clear : QueryPlan

    /** Nothing to do: a one-letter query, or the query already shown. */
    data object Keep : QueryPlan

    data class Run(val query: String) : QueryPlan
}

/**
 * The iOS search screen's state. [shown] is the query the results belong to, and the one the filters page through.
 * [generation] numbers searches, so only the latest may write its result. Generic over the result so tests need no
 * FFI records.
 */
data class SearchState<R>(
    val query: String = "",
    val shown: String = "",
    val result: R? = null,
    val error: String? = null,
    val searching: Boolean = false,
    val generation: Int = 0,
) {
    private val trimmed get() = query.trim()

    fun plan(): QueryPlan {
        val q = trimmed
        return when {
            q.isEmpty() -> QueryPlan.Clear
            // One-letter queries return junk.
            q.length < 2 -> QueryPlan.Keep
            // Coming back to the screen must not search again.
            q == shown && (result != null || error != null) -> QueryPlan.Keep
            else -> QueryPlan.Run(q)
        }
    }

    fun cleared() = SearchState<R>(query = query, generation = generation)

    /** The first search shows its query at once, over skeletons; later ones keep the old results until theirs arrive. */
    fun started(q: String) = copy(
        generation = generation + 1,
        searching = true,
        shown = if (result == null) q else shown,
    )

    fun finished(q: String, generation: Int, result: R) =
        if (current(q, generation)) copy(shown = q, result = result, error = null, searching = false) else this

    fun failed(q: String, generation: Int, message: String) =
        if (current(q, generation)) copy(shown = q, error = message, searching = false) else this

    private fun current(q: String, generation: Int) = generation == this.generation && trimmed == q
}

/** Debounces query edits into searches; [settle] runs in an effect keyed on the query, so a new edit cancels it. */
class SearchModel<R>(private val debounceMs: Long = 300, private val search: suspend (String) -> R) {
    var state by mutableStateOf(SearchState<R>())
        private set

    fun edit(query: String) { state = state.copy(query = query) }

    suspend fun settle() {
        when (val plan = state.plan()) {
            QueryPlan.Clear -> state = state.cleared()
            // A search this edit cancelled will never finish, so nothing is searching any more.
            QueryPlan.Keep -> state = state.copy(searching = false)
            is QueryPlan.Run -> {
                delay(debounceMs)
                state = state.started(plan.query)
                val generation = state.generation
                // The FFI call may finish after a newer edit; the generation check drops its result.
                try {
                    val result = search(plan.query)
                    state = state.finished(plan.query, generation, result)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    state = state.failed(plan.query, generation, friendlyMessage(e))
                }
            }
        }
    }
}

/** iOS shows the first artist as the top result only when its name and the query contain one another. */
fun isTopArtist(name: String, query: String) =
    name.isNotEmpty() && (name.contains(query, ignoreCase = true) || query.contains(name, ignoreCase = true))
