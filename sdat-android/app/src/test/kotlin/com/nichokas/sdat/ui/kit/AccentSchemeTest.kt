package com.nichokas.sdat.ui.kit

import android.app.Application
import com.nichokas.sdat.core.Accent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class AccentSchemeTest {
    private val context = RuntimeEnvironment.getApplication()

    @Test fun eachAccentGivesItsOwnPrimaryInBothModes() {
        for (dark in listOf(false, true)) {
            val primaries = Accent.entries.map { colorScheme(it, dark, context).primary }
            assertEquals(Accent.entries.size, primaries.toSet().size)
        }
    }

    @Test fun presetsFollowLightAndDarkMode() {
        for (accent in Accent.entries.filter { it != Accent.System }) {
            assertNotEquals(colorScheme(accent, false, context).surface, colorScheme(accent, true, context).surface)
        }
    }
}
