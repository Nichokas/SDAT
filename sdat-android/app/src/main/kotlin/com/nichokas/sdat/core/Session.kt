package com.nichokas.sdat.core

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.edit
import java.security.KeyStore
import java.util.Base64
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import uniffi.tonearm_ffi.AppConfig
import uniffi.tonearm_ffi.LoginTokens
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.TokenStore
import uniffi.tonearm_ffi.sessionChanged

/** TIDAL credentials for the core; the refresh token is encrypted with an Android Keystore key. */
class Session(context: Context) : TokenStore {
    private val prefs = context.getSharedPreferences("session", Context.MODE_PRIVATE)

    var signedIn by mutableStateOf(refreshToken() != null)
        private set

    private val picture = mutableStateOf(prefs.getString(PICTURE, null))

    /** Kept with the session so it never outlives the account; cached so the button doesn't flash the placeholder. */
    var profilePicture: String?
        get() = picture.value
        set(url) { picture.value = url; prefs.edit { putString(PICTURE, url) } }

    override fun refreshToken(): String? = prefs.getString(REFRESH, null)?.let(Vault::decrypt)

    override fun userId(): String? = prefs.getString(USER, null)

    fun signIn(tokens: LoginTokens) {
        picture.value = null
        // A session that failed to restore leaves the previous account's entries behind.
        prefs.edit {
            clear()
            putString(REFRESH, Vault.encrypt(tokens.refreshToken))
            putString(USER, tokens.userId)
        }
        Favs.clear()
        sessionChanged()
        signedIn = true
    }

    /** Also the way out of a session TIDAL no longer accepts. */
    fun signOut() {
        prefs.edit { clear() }
        picture.value = null
        Favs.clear()
        sessionChanged()
        signedIn = false
    }

    private companion object {
        const val REFRESH = "refresh_token"
        const val USER = "user_id"
        const val PICTURE = "profile_picture"
    }
}

class Settings(private val configDir: String, private val prefs: Preferences) : AppConfig {
    override fun streamingQuality() = prefs.streamingQuality

    override fun configDir() = configDir
}

internal object Vault {
    private const val ALIAS = "sdat-session"
    private const val IV_BYTES = 12

    fun forget() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }.deleteEntry(ALIAS)

    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(ALIAS, null) as SecretKey?)?.let { return it }
        val spec = KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .build()
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply { init(spec) }.generateKey()
    }

    fun encrypt(plain: String): String {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        return Base64.getEncoder().encodeToString(cipher.iv + cipher.doFinal(plain.toByteArray()))
    }

    /** Null when the key is gone (app data restored on another device): the user signs in again. */
    fun decrypt(sealed: String): String? = runCatching {
        val bytes = Base64.getDecoder().decode(sealed)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, bytes, 0, IV_BYTES))
        String(cipher.doFinal(bytes, IV_BYTES, bytes.size - IV_BYTES))
    }.getOrNull()
}
