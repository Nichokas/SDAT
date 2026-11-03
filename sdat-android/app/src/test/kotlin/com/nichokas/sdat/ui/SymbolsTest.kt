package com.nichokas.sdat.ui

import org.junit.Assert.assertTrue
import org.junit.Test

class SymbolsTest {
    @Test
    fun sfCodepointsAreInSupplementaryPrivateUseAreaB() {
        symbols.forEach { (name, s) -> assertTrue(name, s.sfCodePoint in 0x100000..0x10FFFD) }
    }
}
