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
    private var settleJob: Job? = null

    var nav by mutableStateOf(NavState())
        private set
    val pages = PageStore()
    var bar by mutableStateOf(BarCollapse())
        private set
    var showPlayer by mutableStateOf(false)
    var toast by mutableStateOf<String?>(null)
        private set

    fun push(route: Route) = navigate(nav.push(route))

    fun pop() = navigate(nav.pop())

    fun restore(saved: NavState) = navigate(saved)

    fun select(tab: AppTab) {
        if (tab != nav.tab && bar.collapsed) setCollapsed(false)
        navigate(nav.select(tab))
    }

    private fun navigate(next: NavState) {
        nav = next
        pages.keep(next)
    }

    fun setCollapsed(on: Boolean) = updateBar { bar.set(on, it) }

    /** Called by a scrolling tab root with how far its content has moved up, in dp. */
    fun rootScrolled(scrollDp: Float) = updateBar { bar.scrolled(scrollDp, it) }

    private fun updateBar(next: (nowMs: Long) -> BarCollapse) {
        val now = SystemClock.uptimeMillis()
        val lock = bar.lockedUntilMs
        bar = next(now)
        val until = bar.lockedUntilMs
        if (until == lock) return
        settleJob?.cancel()
        settleJob = scope.launch { delay(until - now); updateBar { bar.settle(it) } }
    }

    /** The object outlives sign-out, so the next account must not land on the last one's pages. */
    fun reset() {
        nav = NavState()
        settleJob?.cancel()
        pages.clear()
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
