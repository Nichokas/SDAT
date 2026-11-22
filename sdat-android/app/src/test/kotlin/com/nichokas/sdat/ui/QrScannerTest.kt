package com.nichokas.sdat.ui

import java.util.concurrent.Callable
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.CyclicBarrier
import java.util.concurrent.Executors
import java.util.concurrent.LinkedBlockingDeque
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class QrScannerTest {
    private class FakeLooperThread {
        private val queue = LinkedBlockingDeque<Runnable>()
        private val quitting = AtomicBoolean(false)
        private val poison = Runnable {}
        val thread = Thread({
            while (true) {
                val task = queue.take()
                if (task === poison) return@Thread
                task.run()
            }
        }, "qr-scanner-test").apply { start() }

        fun post(task: Runnable): Boolean = synchronized(quitting) {
            if (quitting.get()) return false
            queue.offerLast(task)
            true
        }

        fun postFront(task: Runnable): Boolean = synchronized(quitting) {
            if (quitting.get()) return false
            queue.offerFirst(task)
            true
        }

        fun quitSafely() = synchronized(quitting) {
            if (quitting.compareAndSet(false, true)) queue.offerLast(poison)
        }
    }

    @Test fun destroySurfaceInvalidatesTokenImmediatelyAndWaitsForSlowCameraTask() {
        val worker = FakeLooperThread()
        val gate = QrSurfaceGate()
        val token = gate.activateSurface()
        assertTrue(gate.isActive(token))

        val inFlightStarted = CountDownLatch(1)
        val allowInFlightFinish = CountDownLatch(1)
        val events = CopyOnWriteArrayList<String>()

        worker.post(Runnable {
            inFlightStarted.countDown()
            assertTrue(allowInFlightFinish.await(5, TimeUnit.SECONDS))
            events += "in-flight-done"
        })
        assertTrue(inFlightStarted.await(5, TimeUnit.SECONDS))

        val destroyReturned = CountDownLatch(1)
        val uiThread = Thread {
            gate.destroySurface(
                onWorkerThread = { Thread.currentThread() === worker.thread },
                postFront = worker::postFront,
                joinWorker = worker.thread::join,
                onWorker = { events += "closed" },
            )
            destroyReturned.countDown()
        }.apply { start() }

        // Must invalidate the surface immediately and must NOT return early after the old 500 ms timeout.
        assertFalse(destroyReturned.await(600, TimeUnit.MILLISECONDS))
        assertFalse(gate.isActive(token))
        assertFalse(gate.hasSurface)
        assertTrue(events.isEmpty())

        allowInFlightFinish.countDown()
        assertTrue(destroyReturned.await(5, TimeUnit.SECONDS))
        uiThread.join(5000)
        assertEquals(listOf("in-flight-done", "closed"), events)

        worker.quitSafely()
        worker.thread.join(5000)
    }

    @Test fun destroySurfaceWaitsThroughInterruptAndRestoresInterruptStatus() {
        val worker = FakeLooperThread()
        val gate = QrSurfaceGate()
        gate.activateSurface()

        val inFlightStarted = CountDownLatch(1)
        val allowInFlightFinish = CountDownLatch(1)
        val closedOnWorker = AtomicBoolean(false)
        val interruptedAfterReturn = AtomicBoolean(false)

        worker.post(Runnable {
            inFlightStarted.countDown()
            assertTrue(allowInFlightFinish.await(5, TimeUnit.SECONDS))
        })
        assertTrue(inFlightStarted.await(5, TimeUnit.SECONDS))

        val destroyReturned = CountDownLatch(1)
        val uiThread = Thread {
            // Pre-set interrupt flag to prove destroySurface never returns early on InterruptedException.
            Thread.currentThread().interrupt()
            gate.destroySurface(
                onWorkerThread = { Thread.currentThread() === worker.thread },
                postFront = worker::postFront,
                joinWorker = worker.thread::join,
                onWorker = { closedOnWorker.set(true) },
            )
            interruptedAfterReturn.set(Thread.currentThread().isInterrupted)
            destroyReturned.countDown()
        }.apply { start() }

        assertFalse(destroyReturned.await(150, TimeUnit.MILLISECONDS))
        assertFalse(closedOnWorker.get())

        allowInFlightFinish.countDown()
        assertTrue(destroyReturned.await(5, TimeUnit.SECONDS))
        uiThread.join(5000)
        assertTrue(closedOnWorker.get())
        assertTrue(interruptedAfterReturn.get())

        worker.quitSafely()
        worker.thread.join(5000)
    }

    @Test fun destroySurfacePreemptsQueuedWorkBeforeReturning() {
        val worker = FakeLooperThread()
        val gate = QrSurfaceGate()
        val token = gate.activateSurface()

        val inFlightStarted = CountDownLatch(1)
        val allowInFlightFinish = CountDownLatch(1)
        val queuedRan = CountDownLatch(1)
        val events = CopyOnWriteArrayList<String>()

        worker.post(Runnable {
            inFlightStarted.countDown()
            assertTrue(allowInFlightFinish.await(5, TimeUnit.SECONDS))
            events += "in-flight"
        })
        assertTrue(inFlightStarted.await(5, TimeUnit.SECONDS))

        worker.post(Runnable {
            events += if (gate.isActive(token)) "stale-active" else "stale-skipped"
            queuedRan.countDown()
        })

        val closePosted = CountDownLatch(1)
        val destroyDone = CountDownLatch(1)
        val uiThread = Thread {
            gate.destroySurface(
                onWorkerThread = { Thread.currentThread() === worker.thread },
                postFront = { task -> worker.postFront(task).also { closePosted.countDown() } },
                joinWorker = worker.thread::join,
                onWorker = { events += "closed" },
            )
            destroyDone.countDown()
        }.apply { start() }

        assertTrue(closePosted.await(5, TimeUnit.SECONDS))
        assertFalse(gate.hasSurface)

        allowInFlightFinish.countDown()
        assertTrue(destroyDone.await(5, TimeUnit.SECONDS))
        assertTrue(queuedRan.await(5, TimeUnit.SECONDS))
        uiThread.join(5000)
        assertEquals(listOf("in-flight", "closed", "stale-skipped"), events)

        worker.quitSafely()
        worker.thread.join(5000)
    }

    @Test fun destroySurfaceJoinsQuittingWorkerWhenPostIsRejectedAfterRelease() {
        val worker = FakeLooperThread()
        val gate = QrSurfaceGate()
        gate.activateSurface()

        val releaseCloseStarted = CountDownLatch(1)
        val allowReleaseCloseFinish = CountDownLatch(1)
        val closeCalls = AtomicInteger(0)

        assertTrue(gate.markReleased())
        assertFalse(gate.markReleased())
        assertTrue(worker.postFront(Runnable {
            releaseCloseStarted.countDown()
            assertTrue(allowReleaseCloseFinish.await(5, TimeUnit.SECONDS))
            closeCalls.incrementAndGet()
        }))
        worker.quitSafely()
        assertTrue(releaseCloseStarted.await(5, TimeUnit.SECONDS))

        val destroyReturned = CountDownLatch(1)
        val uiThread = Thread {
            gate.destroySurface(
                onWorkerThread = { Thread.currentThread() === worker.thread },
                postFront = worker::postFront,
                joinWorker = worker.thread::join,
                onWorker = { closeCalls.incrementAndGet() },
            )
            destroyReturned.countDown()
        }.apply { start() }

        assertFalse(destroyReturned.await(150, TimeUnit.MILLISECONDS))
        assertEquals(0, closeCalls.get())

        allowReleaseCloseFinish.countDown()
        assertTrue(destroyReturned.await(5, TimeUnit.SECONDS))
        uiThread.join(5000)
        assertFalse(worker.thread.isAlive)
        assertEquals(2, closeCalls.get())
    }

    @Test fun destroySurfaceOnWorkerThreadRunsInlineWithoutDeadlock() {
        val gate = QrSurfaceGate()
        val token = gate.activateSurface()
        var closed = false
        var posted = false
        var joined = false

        gate.destroySurface(
            onWorkerThread = { true },
            postFront = { posted = true; true },
            joinWorker = { joined = true },
            onWorker = { closed = true },
        )

        assertTrue(closed)
        assertFalse(posted)
        assertFalse(joined)
        assertFalse(gate.isActive(token))
    }

    @Test fun surfaceTokensStayStableAcrossDuplicateChangesAndRejectStaleLifecycles() {
        val gate = QrSurfaceGate()
        assertFalse(gate.hasSurface)
        assertFalse(gate.isActive(0))

        val first = gate.activateSurface()
        assertTrue(first > 0)
        assertTrue(gate.isActive(first))
        assertEquals(first, gate.activateSurface())

        gate.destroySurface(
            onWorkerThread = { true },
            postFront = { true },
            joinWorker = {},
            onWorker = {},
        )
        assertFalse(gate.isActive(first))
        assertFalse(gate.hasSurface)

        val second = gate.activateSurface()
        assertNotEquals(first, second)
        assertFalse(gate.isActive(first))
        assertTrue(gate.isActive(second))

        assertTrue(gate.markReleased())
        assertTrue(gate.released)
        assertFalse(gate.isActive(second))
        assertEquals(0, gate.activateSurface())
    }

    @Test fun activateSurfaceWaitsForInFlightDestroySurfaceAndAllocatesFreshToken() {
        val worker = FakeLooperThread()
        val gate = QrSurfaceGate()
        val first = gate.activateSurface()

        val inFlightStarted = CountDownLatch(1)
        val allowInFlightFinish = CountDownLatch(1)
        val destroyPosted = CountDownLatch(1)
        val destroyDone = CountDownLatch(1)
        val activateStarted = CountDownLatch(1)
        val activateDone = CountDownLatch(1)
        val secondToken = AtomicInteger(0)
        val events = CopyOnWriteArrayList<String>()

        worker.post(Runnable {
            inFlightStarted.countDown()
            assertTrue(allowInFlightFinish.await(5, TimeUnit.SECONDS))
        })
        assertTrue(inFlightStarted.await(5, TimeUnit.SECONDS))

        val destroyThread = Thread {
            gate.destroySurface(
                onWorkerThread = { Thread.currentThread() === worker.thread },
                postFront = { task -> worker.postFront(task).also { destroyPosted.countDown() } },
                joinWorker = worker.thread::join,
                onWorker = { events += "closed" },
            )
            destroyDone.countDown()
        }.apply { start() }

        assertTrue(destroyPosted.await(5, TimeUnit.SECONDS))
        assertFalse(gate.isActive(first))
        assertFalse(gate.hasSurface)

        val activateThread = Thread {
            activateStarted.countDown()
            secondToken.set(gate.activateSurface())
            events += "activated"
            activateDone.countDown()
        }.apply { start() }

        assertTrue(activateStarted.await(5, TimeUnit.SECONDS))
        assertFalse(activateDone.await(150, TimeUnit.MILLISECONDS))

        allowInFlightFinish.countDown()
        assertTrue(destroyDone.await(5, TimeUnit.SECONDS))
        assertTrue(activateDone.await(5, TimeUnit.SECONDS))
        destroyThread.join(5000)
        activateThread.join(5000)

        assertTrue(secondToken.get() > first)
        assertFalse(gate.isActive(first))
        assertTrue(gate.isActive(secondToken.get()))
        assertEquals(listOf("closed", "activated"), events)

        worker.quitSafely()
        worker.thread.join(5000)
    }

    @Test fun destroySurfacePreservesReleasedStateAndSerializesWithMarkReleased() {
        val worker = FakeLooperThread()
        val gate = QrSurfaceGate()
        gate.activateSurface()

        val inFlightStarted = CountDownLatch(1)
        val allowInFlightFinish = CountDownLatch(1)
        val destroyPosted = CountDownLatch(1)
        val destroyDone = CountDownLatch(1)
        val releaseStarted = CountDownLatch(1)
        val releaseDone = CountDownLatch(1)
        val releasedFirst = AtomicBoolean(false)

        worker.post(Runnable {
            inFlightStarted.countDown()
            assertTrue(allowInFlightFinish.await(5, TimeUnit.SECONDS))
        })
        assertTrue(inFlightStarted.await(5, TimeUnit.SECONDS))

        val destroyThread = Thread {
            gate.destroySurface(
                onWorkerThread = { Thread.currentThread() === worker.thread },
                postFront = { task -> worker.postFront(task).also { destroyPosted.countDown() } },
                joinWorker = worker.thread::join,
                onWorker = {},
            )
            destroyDone.countDown()
        }.apply { start() }

        assertTrue(destroyPosted.await(5, TimeUnit.SECONDS))

        val releaseThread = Thread {
            releaseStarted.countDown()
            releasedFirst.set(gate.markReleased())
            releaseDone.countDown()
        }.apply { start() }

        assertTrue(releaseStarted.await(5, TimeUnit.SECONDS))
        assertFalse(releaseDone.await(150, TimeUnit.MILLISECONDS))

        allowInFlightFinish.countDown()
        assertTrue(destroyDone.await(5, TimeUnit.SECONDS))
        assertTrue(releaseDone.await(5, TimeUnit.SECONDS))
        destroyThread.join(5000)
        releaseThread.join(5000)

        assertTrue(releasedFirst.get())
        assertTrue(gate.released)
        assertFalse(gate.hasSurface)
        assertEquals(0, gate.activateSurface())

        gate.destroySurface(
            onWorkerThread = { true },
            postFront = { true },
            joinWorker = {},
            onWorker = {},
        )
        assertTrue(gate.released)
        assertFalse(gate.hasSurface)
        assertEquals(0, gate.activateSurface())

        worker.quitSafely()
        worker.thread.join(5000)
    }

    @Test fun concurrentActivateDestroyAndReleaseKeepTokenStateConsistent() {
        val pool = Executors.newFixedThreadPool(2)
        try {
            repeat(200) {
                val gate = QrSurfaceGate()
                val first = gate.activateSurface()
                val barrier = CyclicBarrier(2)
                val destroyFuture = pool.submit(Runnable {
                    barrier.await(5, TimeUnit.SECONDS)
                    gate.destroySurface(
                        onWorkerThread = { true },
                        postFront = { true },
                        joinWorker = {},
                        onWorker = {},
                    )
                })
                val activateFuture = pool.submit(Callable {
                    barrier.await(5, TimeUnit.SECONDS)
                    gate.activateSurface()
                })
                destroyFuture.get(5, TimeUnit.SECONDS)
                val observed = activateFuture.get(5, TimeUnit.SECONDS)
                assertFalse(gate.isActive(first))
                if (observed == first) {
                    assertFalse(gate.hasSurface)
                } else {
                    assertTrue(observed > first)
                    assertTrue(gate.isActive(observed))
                }
            }

            repeat(200) {
                val gate = QrSurfaceGate()
                val barrier = CyclicBarrier(2)
                val releaseFuture = pool.submit(Callable {
                    barrier.await(5, TimeUnit.SECONDS)
                    gate.markReleased()
                })
                val activateFuture = pool.submit(Callable {
                    barrier.await(5, TimeUnit.SECONDS)
                    gate.activateSurface()
                })
                assertTrue(releaseFuture.get(5, TimeUnit.SECONDS))
                val observed = activateFuture.get(5, TimeUnit.SECONDS)
                assertTrue(gate.released)
                assertFalse(gate.hasSurface)
                assertFalse(gate.isActive(observed))
                assertEquals(0, gate.activateSurface())
            }
        } finally {
            pool.shutdownNow()
        }
    }
}
