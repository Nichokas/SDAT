package com.nichokas.sdat

import android.app.Application
import android.os.Build
import com.nichokas.sdat.core.CrashReporting
import com.nichokas.sdat.core.DiagLog
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.Handoff
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Recents
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.SentryReporter
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.core.Settings
import java.io.File
import uniffi.tonearm_ffi.diagInit
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
    lateinit var crashReporting: CrashReporting
        private set

    override fun onCreate() {
        super.onCreate()
        diagInit(File(filesDir, "diag").path, DiagLog)
        session = Session(this)
        prefs = Preferences(this)
        crashReporting = CrashReporting(BuildConfig.SENTRY_DSN, prefs.crashReports, { prefs.crashReports = it }, SentryReporter(this))
        Downloads.init(this, prefs) { DownloadService.start(this) }
        secrets = Secrets(this)
        Recents.open(File(filesDir, "recents"))
        val settings = Settings(filesDir.path, prefs)
        initialize(session, settings)
        playback = Playback(this, prefs, secrets)
        Live.attach(playback, prefs, ::deviceName)
        Handoff.attach(playback, prefs, session, ::deviceName)
    }

    private fun deviceName() =
        android.provider.Settings.Global.getString(contentResolver, android.provider.Settings.Global.DEVICE_NAME) ?: Build.MODEL
}
