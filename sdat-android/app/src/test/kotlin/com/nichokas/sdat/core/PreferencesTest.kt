package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.tonearm_ffi.Quality

class PreferencesTest {
    @Test
    fun qualityKeysMatchIos() {
        assertEquals(listOf("hifi", "cd", "aac"), qualities.map { it.key })
        qualities.forEach { assertEquals(it, qualityOf(it.key)) }
    }

    @Test
    fun unknownKeyHasNoQuality() {
        assertNull(qualityOf("lossless"))
        assertNull(qualityOf(null))
        assertEquals(Quality.HI_FI, qualityOf(null) ?: Quality.HI_FI)
    }
}
