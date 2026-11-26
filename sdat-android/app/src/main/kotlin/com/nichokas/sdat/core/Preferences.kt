package com.nichokas.sdat.core

import android.content.Context
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.edit
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.audioCacheLimitDefault
import uniffi.tonearm_ffi.diagClear
import uniffi.tonearm_ffi.handoffNewInstallationKey

/** Picker order, keys and copy as on iOS. */
val qualities = listOf(Quality.HI_FI, Quality.CD, Quality.AAC)

val Quality.key: String
    get() = when (this) {
        Quality.HI_FI -> "hifi"
        Quality.CD -> "cd"
        Quality.AAC -> "aac"
    }

fun qualityOf(key: String?): Quality? = qualities.firstOrNull { it.key == key }

val Quality.label: String
    get() = when (this) {
        Quality.HI_FI -> "Hi-Res"
        Quality.CD -> "CD"
        Quality.AAC -> "AAC"
    }

val Quality.detail: String
    get() = when (this) {
        Quality.HI_FI -> "Up to 24-bit/192 kHz FLAC"
        Quality.CD -> "16-bit/44.1 kHz FLAC"
        Quality.AAC -> "Up to 320 kbps AAC"
    }

/** The app's accent: the wallpaper's Material You colour, or a fixed one. */
enum class Accent(val key: String, val label: String) {
    System("system", "Wallpaper"),
    Blue("blue", "Blue"),
    Green("green", "Green"),
    Rose("rose", "Rose"),
}

/** User preferences. Secrets never go here; see [Secrets]. */
class Preferences(context: Context) {
    private val d = context.getSharedPreferences("preferences", Context.MODE_PRIVATE)

    private val quality = mutableStateOf(Quality.HI_FI)
    private val cellular = mutableStateOf(false)
    private val preload = mutableStateOf(true)
    private val cacheLimit = mutableLongStateOf(0)
    private val lastFm = mutableStateOf(false)
    private val listenBrainz = mutableStateOf(false)
    private val liveDelay = mutableIntStateOf(0)
    private val crash = mutableStateOf(CrashReports.Unset)
    private val accentState = mutableStateOf(Accent.System)

    var streamingQuality: Quality
        get() = quality.value
        set(q) { quality.value = q; d.edit { putString("streamingQuality", q.key) } }
    var downloadOverCellular: Boolean
        get() = cellular.value
        set(on) { cellular.value = on; d.edit { putBoolean("downloadOverCellular", on) } }
    var preloadNext: Boolean
        get() = preload.value
        set(on) { preload.value = on; d.edit { putBoolean("preloadNext", on) } }
    /** Bytes of tracks the audio cache keeps; 0 turns it off. Set through [AudioCache.setLimit]. */
    var audioCacheLimit: Long
        get() = cacheLimit.longValue
        set(bytes) { cacheLimit.longValue = bytes; d.edit { putLong("audioCacheLimit", bytes) } }
    var scrobbleLastFm: Boolean
        get() = lastFm.value
        set(on) { lastFm.value = on; d.edit { putBoolean("scrobbleLastFm", on) } }
    var scrobbleListenBrainz: Boolean
        get() = listenBrainz.value
        set(on) { listenBrainz.value = on; d.edit { putBoolean("scrobbleListenBrainz", on) } }

    /** Listen Together guest's manual output delay, in ms (iOS key). */
    var liveExtraLatencyMs: Int
        get() = liveDelay.intValue
        set(ms) { liveDelay.intValue = ms; d.edit { putInt("liveExtraLatencyMsV2", ms) } }

    var accent: Accent
        get() = accentState.value
        set(a) { accentState.value = a; d.edit { putString("accent", a.key) } }

    var crashReports: CrashReports
        get() = crash.value
        set(c) { crash.value = c; d.edit { if (c.key == null) remove("crash_reports") else putString("crash_reports", c.key) } }

    /** This install as a player of the TIDAL account (Handoff), made once. */
    val handoffInstallationKey: String
        get() = d.getString("handoffInstallationKey", null)
            ?: handoffNewInstallationKey().also { d.edit { putString("handoffInstallationKey", it) } }

    init { load() }

    private fun load() {
        quality.value = qualityOf(d.getString("streamingQuality", null)) ?: Quality.HI_FI
        cellular.value = d.getBoolean("downloadOverCellular", false)
        preload.value = d.getBoolean("preloadNext", true)
        cacheLimit.longValue = d.getLong("audioCacheLimit", audioCacheLimitDefault().toLong())
        lastFm.value = d.getBoolean("scrobbleLastFm", false)
        listenBrainz.value = d.getBoolean("scrobbleListenBrainz", false)
        liveDelay.intValue = d.getInt("liveExtraLatencyMsV2", 0)
        crash.value = CrashReports.of(d.getString("crash_reports", null))
        accentState.value = d.getString("accent", null).let { key -> Accent.entries.firstOrNull { it.key == key } } ?: Accent.System
    }

    fun clear() { d.edit(commit = true) { clear() }; load() }
}

/** Last.fm and ListenBrainz credentials, encrypted like the TIDAL refresh token (the iOS Keychain entries). */
class Secrets(context: Context) {
    private val d = context.getSharedPreferences("secrets", Context.MODE_PRIVATE)

    fun get(key: String): String? = d.getString(key, null)?.let(Vault::decrypt)

    fun set(key: String, value: String?) = d.edit { if (value == null) remove(key) else putString(key, Vault.encrypt(value)) }

    fun clear() = d.edit(commit = true) { clear() }

    companion object {
        const val LASTFM_SESSION = "lastfm.session"
        const val LASTFM_USER = "lastfm.user"
        const val LISTENBRAINZ_TOKEN = "listenbrainz.token"
    }
}

// Outlives Settings, which leaves the composition as soon as the sign-out lands.
private val eraseScope = MainScope()

/**
 * Erase all app data: TIDAL session, Last.fm and ListenBrainz tokens, their Keystore key, settings, caches and stored files.
 * Signs out first, so nothing in the core still uses the files when they go.
 */
fun eraseAll(context: Context, session: Session, prefs: Preferences, secrets: Secrets, crashReporting: CrashReporting) {
    session.signOut()
    secrets.clear()
    prefs.clear()
    AudioCache.setLimit(prefs.audioCacheLimit)
    crashReporting.reset()
    diagClear()
    Vault.forget()
    Region.select(null)
    eraseScope.launch(Dispatchers.IO) {
        // diagClear emptied the diagnostics files; the core keeps diag.log open, so its directory stays.
        val diag = File(context.filesDir, "diag")
        listOf(context.cacheDir, context.filesDir).forEach { dir -> dir.listFiles()?.filter { it != diag }?.forEach { it.deleteRecursively() } }
    }
}
