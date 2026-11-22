package com.nichokas.sdat.core

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChromeTest {
    @After fun cleanUp() {
        Chrome.reset()
    }

    @Test fun resetCancelsInFlightActionsAndKeepsTheScopeUsable() = runBlocking {
        val started = CompletableDeferred<Unit>()
        val cancelled = CompletableDeferred<Unit>()
        val first = Chrome.actionScope.launch(Dispatchers.Default) {
            started.complete(Unit)
            try {
                CompletableDeferred<Unit>().await()
            } finally {
                cancelled.complete(Unit)
            }
        }
        started.await()

        Chrome.reset()
        first.join()
        cancelled.await()
        assertTrue(first.isCancelled)

        val result = CompletableDeferred<String>()
        val second = Chrome.actionScope.launch(Dispatchers.Default) { result.complete("after-reset") }
        second.join()
        assertEquals("after-reset", result.await())
    }
}
