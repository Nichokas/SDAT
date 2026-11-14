package com.nichokas.sdat.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.currentCompositeKeyHashCode
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.PageKey

/** The back-stack page being composed; the tab stack provides it to each entry. */
val LocalPage = staticCompositionLocalOf<PageKey> { error("No page provided") }

/**
 * Like `remember`, but the value lives as long as the page is on a back stack: popping back to the page, or switching
 * back to its tab, finds its results and loaded pages instead of fetching them again.
 */
@Composable
fun <T : Any> rememberRetained(vararg inputs: Any?, init: () -> T): T {
    val page = LocalPage.current
    // The same key rememberSaveable uses, so a call site finds its value when the page composes again.
    val slot = currentCompositeKeyHashCode
    return remember(page, slot, *inputs) { Chrome.pages.get(page, slot, inputs.toList(), init) }
}
