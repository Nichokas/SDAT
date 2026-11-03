package com.nichokas.sdat

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.CompositionLocalProvider
import com.nichokas.sdat.ui.LocalAppFonts
import com.nichokas.sdat.ui.RootView
import com.nichokas.sdat.ui.loadAppFonts
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.glasmioTypography
import io.github.neoruaa.glasmio.icons.GlasmioIconProvider

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val fonts = loadAppFonts(this)
        setContent {
            CompositionLocalProvider(LocalAppFonts provides fonts) {
                GlasmioTheme(typography = glasmioTypography(fonts.text)) {
                    GlasmioIconProvider(symbolTypeface = fonts.sfPro) { RootView() }
                }
            }
        }
    }
}
