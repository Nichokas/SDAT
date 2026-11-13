package com.nichokas.sdat

import android.app.Application
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.core.Settings
import uniffi.tonearm_ffi.initialize

class SdatApp : Application() {
    lateinit var session: Session
        private set
    lateinit var playback: Playback
        private set
    lateinit var prefs: Preferences
        private set
    lateinit var secrets: Secrets
        private set

    override fun onCreate() {
        super.onCreate()
        session = Session(this)
        prefs = Preferences(this)
        secrets = Secrets(this)
        val settings = Settings(filesDir.path, prefs)
        initialize(session, settings)
        playback = Playback(this, prefs, secrets)
    }
}
