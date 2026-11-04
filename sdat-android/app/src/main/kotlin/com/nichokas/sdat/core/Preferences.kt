package com.nichokas.sdat.core

import android.content.Context
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.edit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.tonearm_ffi.Quality

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

/** User preferences. Secrets never go here; see [Secrets]. */
class Preferences(context: Context) {
    private val d = context.getSharedPreferences("preferences", Context.MODE_PRIVATE)

    private val quality = mutableStateOf(Quality.HI_FI)
    private val cellular = mutableStateOf(false)
    private val preload = mutableStateOf(true)
    private val lastFm = mutableStateOf(false)
    private val listenBrainz = mutableStateOf(false)
    private val picture = mutableStateOf<String?>(null)

    var streamingQuality: Quality
        get() = quality.value
        set(q) { quality.value = q; d.edit { putString("streamingQuality", q.key) } }
    var downloadOverCellular: Boolean
        get() = cellular.value
        set(on) { cellular.value = on; d.edit { putBoolean("downloadOverCellular", on) } }
    var preloadNext: Boolean
        get() = preload.value
        set(on) { preload.value = on; d.edit { putBoolean("preloadNext", on) } }
    var scrobbleLastFm: Boolean
        get() = lastFm.value
        set(on) { lastFm.value = on; d.edit { putBoolean("scrobbleLastFm", on) } }
    var scrobbleListenBrainz: Boolean
        get() = listenBrainz.value
        set(on) { listenBrainz.value = on; d.edit { putBoolean("scrobbleListenBrainz", on) } }

    /** Cached across launches so the profile button doesn't flash the placeholder. */
    var profilePicture: String?
        get() = picture.value
        set(url) { picture.value = url; d.edit { putString("profilePicture320", url) } }

    init { load() }

    private fun load() {
        quality.value = qualityOf(d.getString("streamingQuality", null)) ?: Quality.HI_FI
        cellular.value = d.getBoolean("downloadOverCellular", false)
        preload.value = d.getBoolean("preloadNext", true)
        lastFm.value = d.getBoolean("scrobbleLastFm", false)
        listenBrainz.value = d.getBoolean("scrobbleListenBrainz", false)
        picture.value = d.getString("profilePicture320", null)
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

/** Erase all app data: TIDAL session, Last.fm and ListenBrainz tokens, settings, caches and stored files. */
suspend fun eraseAll(context: Context, session: Session, prefs: Preferences, secrets: Secrets) {
    secrets.clear()
    prefs.clear()
    withContext(Dispatchers.IO) {
        listOf(context.cacheDir, context.filesDir).forEach { dir -> dir.listFiles()?.forEach { it.deleteRecursively() } }
    }
    session.signOut()
}
