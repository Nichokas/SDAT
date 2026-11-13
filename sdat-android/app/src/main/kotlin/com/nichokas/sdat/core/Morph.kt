package com.nichokas.sdat.core

import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow

/**
 * Geometry and fade schedule of the bottom-chrome morph (tab bar to circle, mini-player circle to capsule), ported from iOS.
 * Everything derives from one linear clock `t` (0 expanded, 1 collapsed) and the direction, so both widths come from one
 * budget and never overlap. Widths are in dp.
 */
object Morph {
    const val DURATION_MS = 400
    const val ROUND = 54f
    const val CIRCLE = 62f
    const val GAP = 12f
    private const val SHRINK_SHARE = 0.42f
    private const val GROW_SHARE = 0.8f
    private const val GROW_EASE = 0.22f

    /** Shape fractions: tab 0 is the full bar and 1 the circle; mini 0 is the circle and 1 the capsule. */
    data class Frame(val tabWidth: Float, val miniWidth: Float, val tabFraction: Float, val miniFraction: Float)

    /** Widest the tab bar gets beside the circle mini-player, for a chrome [width] inside the margins. */
    fun tabMax(width: Float) = max(width - GAP, ROUND + CIRCLE) - CIRCLE

    // The shrinking shape snaps away while the growing one spreads and is capped at the space already freed.
    fun frame(width: Float, t: Float, collapsing: Boolean): Frame {
        val tabMax = tabMax(width)
        val span = tabMax - ROUND
        val c = if (collapsing) clamp01(t) else 1 - clamp01(t)
        val shrink = snap(c)
        val grow = spread(c)
        val tabF: Float
        val miniF: Float
        if (collapsing) {
            tabF = shrink
            miniF = min(grow, tabF)
        } else {
            miniF = 1 - shrink
            tabF = max(1 - grow, miniF)
        }
        return Frame(tabMax - span * tabF, CIRCLE + span * miniF, tabF, miniF)
    }

    // Trapezoidal velocity: short ease-in, long constant middle, short ease-out, done at GROW_SHARE of the clock.
    private fun spread(x: Float): Float {
        val u = clamp01(x / GROW_SHARE)
        val a = GROW_EASE
        val v = 1 / (1 - a)
        return when {
            u < a -> 0.5f * v * u * u / a
            u > 1 - a -> (1 - u).let { r -> 1 - 0.5f * v * r * r / a }
            else -> v * (u - a / 2)
        }
    }

    // Smoothstep on a warped clock: fast start, soft brake, done at SHRINK_SHARE of the clock.
    private fun snap(x: Float): Float {
        val u = clamp01(x / SHRINK_SHARE).pow(0.7f)
        return u * u * (3 - 2 * u)
    }

    // Outgoing content is gone exactly when incoming content starts, so both are never visible.
    fun outAlpha(f: Float) = clamp01((0.5f - f) / 0.12f)
    fun inAlpha(f: Float) = clamp01((f - 0.5f) / 0.12f)
    fun outScale(f: Float) = 1 - 0.4f * clamp01((f - 0.38f) / 0.12f)
    fun inScale(f: Float) = 0.6f + 0.4f * clamp01((f - 0.5f) / 0.12f)

    private fun clamp01(x: Float) = x.coerceIn(0f, 1f)
}

/**
 * Whether a scrolled tab root has collapsed the bottom chrome. A flip locks out scroll updates until its morph has run,
 * as on iOS, so a scroll back cannot cut it short.
 */
data class BarCollapse(val collapsed: Boolean = false, val collapsing: Boolean = true, val lockedUntilMs: Long = 0) {
    fun set(on: Boolean, nowMs: Long) = BarCollapse(on, on, nowMs + Morph.DURATION_MS + 20)

    /** [scrollDp] is how far the list's content has moved up. */
    fun scrolled(scrollDp: Float, nowMs: Long): BarCollapse {
        if (nowMs < lockedUntilMs) return this
        val on = scrollDp >= COLLAPSE_AT_DP
        return if (on == collapsed) this else set(on, nowMs)
    }

    companion object {
        // iOS maps 20...80 pt of scroll to progress 0...1 and flips at the midpoint.
        const val COLLAPSE_AT_DP = 50f
    }
}

/**
 * A tab root's list position, observed instead of its scroll distance: the distance is the same everywhere past the
 * first item, and a deduplicating flow of it would drop every later scroll there.
 */
data class ListScroll(val index: Int, val offsetPx: Int) {
    // Past the first item the exact distance no longer matters: the chrome is collapsed either way.
    fun dp(density: Float) = if (index > 0) Float.MAX_VALUE else offsetPx / density
}
