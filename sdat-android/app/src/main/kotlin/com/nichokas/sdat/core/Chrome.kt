package com.nichokas.sdat.core

import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** Navigation, bottom-chrome and transient feedback state shared by every screen. */
object Chrome {
    private val scope = MainScope()
    private var toastJob: Job? = null

    var nav by mutableStateOf(NavState())
        private set
    var bar by mutableStateOf(BarCollapse())
        private set
    var showPlayer by mutableStateOf(false)
    var toast by mutableStateOf<String?>(null)
        private set

    fun push(route: Route) { nav = nav.push(route) }

    fun pop() { nav = nav.pop() }

    fun select(tab: AppTab) {
        if (tab != nav.tab && bar.collapsed) setCollapsed(false)
        nav = nav.select(tab)
    }

    fun setCollapsed(on: Boolean) { bar = bar.set(on, SystemClock.uptimeMillis()) }

    /** Called by a scrolling tab root with how far its content has moved up, in dp. */
    fun rootScrolled(scrollDp: Float) { bar = bar.scrolled(scrollDp, SystemClock.uptimeMillis()) }

    /** The object outlives sign-out, so the next account must not land on the last one's pages. */
    fun reset() {
        nav = NavState()
        bar = BarCollapse()
        showPlayer = false
        toastJob?.cancel()
        toast = null
    }

    fun flash(text: String) {
        toast = text
        toastJob?.cancel()
        toastJob = scope.launch { delay(1600); if (toast == text) toast = null }
    }
}
