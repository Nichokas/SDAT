package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

sealed interface Route {
    data class Album(val id: String) : Route
    data class Artist(val id: String) : Route
}

/** Navigation and transient feedback shared by every screen. */
object Chrome {
    private val scope = MainScope()
    private var toastJob: Job? = null

    val path = mutableStateListOf<Route>()
    var showPlayer by mutableStateOf(false)
    var toast by mutableStateOf<String?>(null)
        private set

    fun push(r: Route) = path.add(r)

    fun flash(text: String) {
        toast = text
        toastJob?.cancel()
        toastJob = scope.launch { delay(1600); if (toast == text) toast = null }
    }
}
