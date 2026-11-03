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

    override fun refreshToken(): String? = prefs.getString(REFRESH, null)?.let(Vault::decrypt)

    override fun userId(): String? = prefs.getString(USER, null)

    fun signIn(tokens: LoginTokens) {
        prefs.edit {
            putString(REFRESH, Vault.encrypt(tokens.refreshToken))
            putString(USER, tokens.userId)
        }
        sessionChanged()
        signedIn = true
    }

    private companion object {
        const val REFRESH = "refresh_token"
        const val USER = "user_id"
    }
}

class Settings(private val configDir: String) : AppConfig {
    override fun streamingQuality() = Quality.HI_FI

    override fun configDir() = configDir
}

private object Vault {
    private const val ALIAS = "sdat-session"
    private const val IV_BYTES = 12

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
