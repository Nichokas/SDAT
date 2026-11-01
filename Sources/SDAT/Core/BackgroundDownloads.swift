import BackgroundTasks
import UIKit
import os

private let bgLog = Logger(subsystem: "com.nichokas.SDAT", category: "bgdownloads")

/// Keeps the download worker running in the background via a BGContinuedProcessingTask
/// (iOS 26+): the system shows its progress UI (title, subtitle, progress) and may expire
/// it. Started only from a user action (Downloads.download). Expiry keeps the queue on
/// disk (resumed on next open); a cancel from the system UI cancels everything.
@MainActor
final class BackgroundDownloads {
    static let shared = BackgroundDownloads()
    /// Info.plist BGTaskSchedulerPermittedIdentifiers holds "\(prefix).*".
    static let prefix = "com.nichokas.SDAT.downloads"

    private var task: BGContinuedProcessingTask?
    private var submitting = false
    private var ticker: Task<Void, Never>?

    func begin() {
        guard task == nil, !submitting, UIApplication.shared.applicationState == .active else { return }
        submitting = true
        let id = "\(Self.prefix).\(UUID().uuidString.prefix(8))"
        // Continued-processing handlers may be registered after launch, once per identifier.
        let ok = BGTaskScheduler.shared.register(forTaskWithIdentifier: id, using: .main) { t in
            MainActor.assumeIsolated {
                guard let t = t as? BGContinuedProcessingTask else { t.setTaskCompleted(success: false); return }
                BackgroundDownloads.shared.run(t)
            }
        }
        guard ok else { bgLog.error("register \(id, privacy: .public) refused"); submitting = false; return }
        let dl = Downloads.shared
        let r = BGContinuedProcessingTaskRequest(identifier: id, title: "Downloading \(dl.batchCount) song\(dl.batchCount == 1 ? "" : "s")",
                                                 subtitle: dl.currentTitle ?? "SDAT")
        r.strategy = .queue
        if #available(iOS 27, *) {
            Task {
                do { try await BGTaskScheduler.shared.submitTaskRequest(r) }
                catch { bgLog.error("submit failed: \(error.localizedDescription, privacy: .public)"); self.submitting = false }
            }
        } else {
            do { try BGTaskScheduler.shared.submit(r) }
            catch { bgLog.error("submit failed: \(error.localizedDescription, privacy: .public)"); submitting = false }
        }
    }

    private func run(_ t: BGContinuedProcessingTask) {
        task = t; submitting = false
        t.progress.totalUnitCount = 1000
        t.progress.cancellationHandler = {
            Task { @MainActor in BackgroundDownloads.shared.cancelledByUser() }
        }
        t.expirationHandler = {
            Task { @MainActor in BackgroundDownloads.shared.expired() }
        }
        ticker = Task { [weak self] in
            while !Task.isCancelled {
                let dl = Downloads.shared
                guard dl.isActive else { self?.finish(true); return }
                t.progress.completedUnitCount = Int64(dl.batchProgress * 1000)
                t.updateTitle("Downloading \(min(dl.batchDone + 1, dl.batchCount)) of \(dl.batchCount)",
                              subtitle: dl.currentTitle ?? "SDAT")
                try? await Task.sleep(for: .milliseconds(500))
            }
        }
    }

    private func cancelledByUser() {
        guard task != nil else { return }
        bgLog.notice("cancelled from system UI")
        Downloads.shared.cancelAll()
        finish(false)
    }

    private func expired() {
        guard let t = task else { return }
        if t.progress.isCancelled || UIApplication.shared.applicationState == .active {
            cancelledByUser(); return
        }
        bgLog.notice("expired; queue kept for next launch")
        Downloads.shared.suspend()
        finish(false)
    }

    private func finish(_ ok: Bool) {
        ticker?.cancel(); ticker = nil
        task?.setTaskCompleted(success: ok)
        task = nil
    }
}
