package com.nichokas.sdat.ui.kit

import android.app.Application
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedback
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class TabBarTest {
    @get:Rule val compose = createComposeRule()

    private val tabs = listOf(TabItem(0, "Discover", "magnifyingglass"), TabItem(1, "Search", "magnifyingglass"))

    @Test fun expandedBarSelectsTheTappedTab() {
        var selected = -1
        val progress = mutableFloatStateOf(0f)
        compose.setContent {
            SdatTheme { TabBar(tabs, 0, { selected = it }, {}, progress, 54.dp, Modifier.width(300.dp)) }
        }
        compose.onNodeWithText("Search").performTouchInput { click() }
        assertEquals(1, selected)
    }

    @Test fun horizontalDragTicksOnCrossingAndSelectsOnlyOnRelease() {
        val selected = mutableIntStateOf(0)
        val ticks = mutableListOf<HapticFeedbackType>()
        val feedback = object : HapticFeedback {
            override fun performHapticFeedback(hapticFeedbackType: HapticFeedbackType) {
                ticks.add(hapticFeedbackType)
            }
        }
        val progress = mutableFloatStateOf(0f)
        compose.setContent {
            CompositionLocalProvider(LocalHapticFeedback provides feedback) {
                SdatTheme {
                    TabBar(tabs, selected.intValue, { selected.intValue = it }, {}, progress, 54.dp, Modifier.width(300.dp))
                }
            }
        }
        val discover = compose.onNodeWithText("Discover")
        val search = compose.onNodeWithText("Search")
        val dx = search.fetchSemanticsNode().boundsInRoot.center.x - discover.fetchSemanticsNode().boundsInRoot.center.x
        discover.performTouchInput {
            down(center)
            moveBy(Offset(dx, 0f))
        }
        discover.assertIsSelected()
        compose.runOnIdle { assertEquals(listOf(HapticFeedbackType.SegmentTick), ticks) }
        discover.performTouchInput { cancel() }
        discover.assertIsSelected()
        discover.performTouchInput {
            down(center)
            moveBy(Offset(dx, 0f))
            up()
        }
        search.assertIsSelected()
    }

    @Test fun compactBarExpandsOnTap() {
        var expanded = false
        val progress = mutableFloatStateOf(1f)
        compose.setContent {
            SdatTheme { TabBar(tabs, 0, {}, { expanded = true }, progress, 54.dp, Modifier.width(300.dp)) }
        }
        compose.onNodeWithContentDescription("Discover").performClick()
        assertTrue(expanded)
    }
}
