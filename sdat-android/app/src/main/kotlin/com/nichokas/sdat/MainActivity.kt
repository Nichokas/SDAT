package com.nichokas.sdat

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.nichokas.sdat.ui.RootView
import com.nichokas.sdat.ui.inter
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.glasmioTypography
import io.github.neoruaa.glasmio.icons.GlasmioIconProvider

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            GlasmioTheme(typography = glasmioTypography(inter)) {
                GlasmioIconProvider { (application as SdatApp).let { RootView(it.session, it.playback, it.prefs, it.secrets) } }
            }
        }
    }
}
