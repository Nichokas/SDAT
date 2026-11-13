package com.nichokas.sdat.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class QueueKeysTest {
    @Test fun repeatsGetTheirOccurrenceNumber() {
        assertEquals(listOf("a#0", "b#0", "a#1"), occurrenceKeys(listOf("a", "b", "a")))
    }

    @Test fun rowsKeepTheirKeyWhenOneAboveIsRemoved() {
        val before = occurrenceKeys(listOf("a", "b", "c"))
        val after = occurrenceKeys(listOf("a", "c"))
        assertEquals(before[2], after[1])
    }
}
