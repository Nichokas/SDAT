package com.nichokas.sdat

import android.app.Application
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.core.Settings
import uniffi.tonearm_ffi.initialize

class SdatApp : Application() {
    lateinit var session: Session
        private set

    override fun onCreate() {
        super.onCreate()
        session = Session(this)
        initialize(session, Settings(filesDir.path))
    }
}
