package com.nichokas.sdat.core

import android.app.Application
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class AccentPreferenceTest {
    private val context get() = RuntimeEnvironment.getApplication()

    @Test fun followsTheWallpaperUntilChosen() {
        assertEquals(Accent.System, Preferences(context).accent)
    }

    @Test fun aChosenAccentOutlivesTheProcess() {
        Preferences(context).accent = Accent.Rose
        assertEquals(Accent.Rose, Preferences(context).accent)
    }

    @Test fun clearingSettingsGoesBackToTheWallpaper() {
        val prefs = Preferences(context)
        prefs.accent = Accent.Green
        prefs.clear()
        assertEquals(Accent.System, prefs.accent)
    }
}
