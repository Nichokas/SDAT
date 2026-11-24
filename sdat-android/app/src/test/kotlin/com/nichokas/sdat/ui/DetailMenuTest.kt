package com.nichokas.sdat.ui

import android.app.Application
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.performClick
import com.nichokas.sdat.ui.kit.MenuAction
import com.nichokas.sdat.ui.kit.SdatTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class DetailMenuTest {
    @get:Rule val compose = createComposeRule()

    @Test fun moreButtonAnnouncesWhetherItsMenuIsOpen() {
        compose.setContent { SdatTheme { DetailMenu("Album", listOf(MenuAction("Share"))) } }
        val more = compose.onNodeWithContentDescription("More")
        more.assert(SemanticsMatcher.expectValue(SemanticsProperties.StateDescription, "Collapsed"))
        more.performClick()
        more.assert(SemanticsMatcher.expectValue(SemanticsProperties.StateDescription, "Expanded"))
    }
}
