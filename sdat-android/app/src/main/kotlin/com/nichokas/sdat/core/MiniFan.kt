package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.sin

/**
 * Press-and-hold menu of the mini player, ported from iOS. The player sits in the bottom-right corner, so it turns
 * into a close button under the finger and the controls fan out over the quarter above and to its left. Sliding onto a
 * control and lifting runs it; lifting on the close button leaves the fan open for taps. Lengths are in dp.
 */
object MiniFan {
    /** A shorter hold toggled playback by accident while tapping to open the player. */
    const val HOLD_MS = 400L
    const val RADIUS = 92f
    /** How far the finger must slide from where it pressed before it leaves the close button. */
    const val DEAD_ZONE = 32f

    /** [angle] in degrees from straight up, counterclockwise toward the left. */
    enum class Item(val angle: Float, val radius: Float, val size: Float) {
        Close(0f, 0f, 56f),
        Toggle(0f, RADIUS, 56f),
        Previous(45f, RADIUS, 48f),
        Next(90f, RADIUS, 48f),
        ;

        val x get() = -sin(Math.toRadians(angle.toDouble())).toFloat() * radius
        val y get() = -cos(Math.toRadians(angle.toDouble())).toFloat() * radius
    }

    sealed interface State {
        data object Closed : State
        /** The finger is still down; [hover] is what lifting it would pick, null for nothing. */
        data class Held(val hover: Item?) : State
        data object Open : State
    }

    var state by mutableStateOf<State>(State.Closed)
        private set

    /** Centre of the mini player in root coordinates, in px; read when the fan opens. */
    var anchor = Offset.Zero

    /** What a finger [dx], [dy] away from where it pressed points at. */
    fun hit(dx: Float, dy: Float): Item? {
        if (hypot(dx, dy) < DEAD_ZONE) return Item.Close
        val angle = Math.toDegrees(atan2(-dx, -dy).toDouble()).toFloat()
        // Past the fan's quarter by more than half a gap: the finger is pointing away from every control.
        if (angle < -30f || angle > 120f) return null
        return Item.entries.filter { it != Item.Close }.minBy { abs(it.angle - angle) }
    }

    fun hold() {
        state = State.Held(Item.Close)
    }

    /** Returns true when the finger has just moved onto a different item. */
    fun move(dx: Float, dy: Float): Boolean {
        val held = state as? State.Held ?: return false
        val hover = hit(dx, dy)
        state = State.Held(hover)
        return hover != null && hover != held.hover
    }

    /** Ends a hold and returns the control to run, if the finger lifted on one. */
    fun release(): Item? {
        val hover = (state as? State.Held)?.hover
        state = if (hover == Item.Close) State.Open else State.Closed
        return hover?.takeIf { it != Item.Close }
    }

    fun close() {
        state = State.Closed
    }
}
