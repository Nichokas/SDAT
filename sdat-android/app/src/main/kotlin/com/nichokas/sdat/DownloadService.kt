package com.nichokas.sdat

import android.app.ForegroundServiceStartNotAllowedException
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import androidx.compose.runtime.snapshotFlow
import com.nichokas.sdat.core.DownloadQueue
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.displayTitle
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

/**
 * Keeps the process alive while the download queue runs, with a progress notification (iOS BackgroundDownloads).
 * The queue itself lives in [Downloads]; this service only holds the foreground and stops once the queue is empty.
 */
class DownloadService : Service() {
    private val scope = MainScope()
    private var lastStart = 0

    override fun onBind(intent: Intent?) = null

    override fun onCreate() {
        super.onCreate()
        getSystemService(NotificationManager::class.java)
            .createNotificationChannel(NotificationChannel(CHANNEL, "Downloads", NotificationManager.IMPORTANCE_LOW))
        // Foreground before anything can stop it: a queue that fails at once (cellular off) empties within milliseconds.
        show(Progress(Downloads.queue))
        scope.launch {
            snapshotFlow { Downloads.queue }.map(::Progress).distinctUntilChanged().collect { p ->
                if (p.total == 0) stopSelf(lastStart) else show(p)
            }
        }
    }

    // Every startForegroundService call needs its own startForeground. Stopping by the latest start id keeps a start
    // that arrives while an emptied queue stops the service from being lost.
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        lastStart = startId
        show(Progress(Downloads.queue))
        if (!Downloads.queue.isActive) stopSelf(startId)
        return START_NOT_STICKY
    }

    // Android 15 caps data sync services at six hours a day; the queue waits on disk for the next launch.
    override fun onTimeout(startId: Int, fgsType: Int) {
        Downloads.pause()
        stopSelf()
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private fun show(p: Progress) {
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        val n = Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setContentTitle("Downloading ${minOf(p.done + 1, p.total)} of ${p.total}")
            .setContentText(p.title)
            .setProgress(100, p.percent, false)
            .setContentIntent(open)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .build()
        startForeground(NOTIFICATION, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
    }

    /** What the notification shows, so it is rebuilt only when that changes. */
    private data class Progress(val done: Int, val total: Int, val percent: Int, val title: String?) {
        constructor(q: DownloadQueue) : this(q.batchDone, q.batchTotal, (q.batchProgress * 100).toInt(), q.current?.let { "${it.displayTitle} · ${it.artistLine}" })
    }

    companion object {
        private const val CHANNEL = "downloads"
        private const val NOTIFICATION = 2

        /** From the foreground only; elsewhere the queue runs as long as the process happens to live. */
        fun start(context: Context) {
            try {
                context.startForegroundService(Intent(context, DownloadService::class.java))
            } catch (_: ForegroundServiceStartNotAllowedException) {
            }
        }
    }
}
