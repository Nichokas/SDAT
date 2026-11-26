package com.nichokas.sdat.ui

import android.app.Application
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasScrollToIndexAction
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.ui.kit.SdatTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class PinnedSearchTest {
    @get:Rule val compose = createComposeRule()

    @Test fun resultsScrollUnderThePinnedSearchField() {
        compose.setContent {
            SdatTheme {
                SdatPage(
                    "Search",
                    pinnedContent = {
                        BasicTextField("Album", {}, Modifier.fillMaxWidth().height(48.dp).testTag("search"))
                    },
                ) {
                    items(60) { Text("Result $it", Modifier.fillMaxWidth().height(56.dp)) }
                }
            }
        }

        val search = compose.onNodeWithTag("search").assertIsDisplayed()
        val start = search.fetchSemanticsNode().positionInRoot
        compose.onNode(hasScrollToIndexAction()).performScrollToIndex(40)
        compose.onNodeWithText("Result 40").assertIsDisplayed()
        search.assertIsDisplayed()
        assertEquals(start, search.fetchSemanticsNode().positionInRoot)
    }
}
