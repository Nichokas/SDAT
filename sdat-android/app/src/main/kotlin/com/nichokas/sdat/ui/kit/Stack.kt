package com.nichokas.sdat.ui.kit

import androidx.compose.animation.ContentTransform
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.runtime.Composable
import androidx.navigation3.runtime.NavEntry
import androidx.navigation3.ui.NavDisplay

// UIKit's navigation push curve.
private val pushEasing = CubicBezierEasing(0.2f, 0.9f, 0.3f, 1f)
private const val PUSH_MS = 420

private fun push() = ContentTransform(
    slideInHorizontally(tween(PUSH_MS, easing = pushEasing)) { it },
    slideOutHorizontally(tween(PUSH_MS, easing = pushEasing)) { -it / 3 },
)

// The leaving page stays on top while it slides away.
private fun pop() = ContentTransform(
    slideInHorizontally(tween(PUSH_MS, easing = pushEasing)) { -it / 3 },
    slideOutHorizontally(tween(PUSH_MS, easing = pushEasing)) { it },
    targetContentZIndex = -1f,
)

/** A back stack with iOS push and pop, also driven by Android's predictive back gesture. */
@Composable
fun <T : Any> Stack(backStack: List<T>, onBack: () -> Unit, entryProvider: (T) -> NavEntry<T>) {
    NavDisplay(
        backStack = backStack,
        onBack = onBack,
        transitionSpec = { push() },
        popTransitionSpec = { pop() },
        predictivePopTransitionSpec = { pop() },
        entryProvider = entryProvider,
    )
}
