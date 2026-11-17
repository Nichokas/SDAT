package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.tonearm_ffi.Country
import uniffi.tonearm_ffi.automaticCountry
import uniffi.tonearm_ffi.countries
import uniffi.tonearm_ffi.preferredCountry
import uniffi.tonearm_ffi.setPreferredCountry

/** The TIDAL country choice, shared by Settings and the picker page it opens. */
object Region {
    val all: List<Country> by lazy { countries() }

    /** The chosen code, `null` for automatic. */
    var preferred by mutableStateOf(preferredCountry())
        private set

    /** What automatic resolves to, once [loadAutomatic] has run. */
    var automatic by mutableStateOf<Country?>(null)
        private set

    val automaticLabel: String get() = automatic?.let { "Automatic (${it.name})" } ?: "Automatic"

    val label: String get() = preferred?.let { code -> all.firstOrNull { it.code == code }?.name } ?: automaticLabel

    fun select(code: String?) {
        setPreferredCountry(code)
        preferred = code
    }

    suspend fun loadAutomatic() {
        automatic = automaticCountry()
    }
}
