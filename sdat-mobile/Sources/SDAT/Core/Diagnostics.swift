import Foundation
import Observation
import OSLog
import Sentry

/// The user's answer about sending crash reports to the developer. Stored as a string so that
/// "never asked" stays distinct from "no".
enum CrashReportConsent: String {
    case unset, allowed, denied
}

/// Owns the crash-report consent and the Sentry lifecycle. The UI reads it; the core's events reach
/// Sentry and the system log through `DiagnosticsSink`, never through this object.
@MainActor @Observable
final class Diagnostics {
    static let shared = Diagnostics()

    private static let consentKey = "crash_reports"

    /// Filled in at release time only (Info.plist `SentryDSN`). Empty in dev builds, local builds and forks.
    let dsn: String
    private(set) var consent: CrashReportConsent
    private(set) var sentryRunning = false

    /// False means no Sentry in this build: the prompt and the toggle stay hidden.
    var available: Bool { !dsn.isEmpty }
    var needsPrompt: Bool { available && consent == .unset }

    private init() {
        // Before anything else at launch, so the core log covers the whole run.
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)
            .first?.appending(path: "diag", directoryHint: .isDirectory).path ?? ""
        diagInit(dir: dir, sink: DiagnosticsSink())

        dsn = (Bundle.main.object(forInfoDictionaryKey: "SentryDSN") as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        consent = CrashReportConsent(rawValue: UserDefaults.standard.string(forKey: Self.consentKey) ?? "") ?? .unset
        syncSentry()
    }

    func setConsent(_ value: CrashReportConsent) {
        consent = value
        UserDefaults.standard.set(value.rawValue, forKey: Self.consentKey)
        syncSentry()
    }

    private func syncSentry() {
        let want = available && consent == .allowed
        guard want != sentryRunning else { return }
        if want {
            let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev"
            let dsn = dsn
            SentrySDK.start { options in
                options.dsn = dsn
                options.releaseName = "sdat-ios@\(version)"
                options.environment = "production"
                options.sendDefaultPii = false
                options.tracesSampleRate = 0
                options.enableAutoPerformanceTracing = false
                options.enableUserInteractionTracing = false
                options.attachScreenshot = false
                options.attachViewHierarchy = false
                options.enableAppHangTracking = false
                // Requests carry TIDAL URLs and tokens; failed ones are already counted by the core.
                options.enableNetworkBreadcrumbs = false
                options.enableNetworkTracking = false
                options.enableCaptureFailedRequests = false
                options.sessionReplay.sessionSampleRate = 0
                options.sessionReplay.onErrorSampleRate = 0
            }
        } else {
            SentrySDK.close()
        }
        sentryRunning = want
    }
}

/// Forwards the core's events to Sentry and the system log. Called from Rust on any thread, so it holds no
/// state and never throws. The core keeps this one sink for the whole run, so it checks
/// `SentrySDK.isEnabled` on every event: consent given or withdrawn later takes effect at once.
final class DiagnosticsSink: DiagSink, Sendable {
    func onEvent(event: DiagEvent) {
        let log = Logger(subsystem: Bundle.main.bundleIdentifier ?? "com.nichokas.SDAT", category: event.target)
        switch event.level {
        case .error: log.error("\(event.message, privacy: .public)")
        case .warn: log.warning("\(event.message, privacy: .public)")
        case .info: log.info("\(event.message, privacy: .public)")
        case .debug, .trace: log.debug("\(event.message, privacy: .public)")
        }

        guard SentrySDK.isEnabled else { return }
        if event.level == .error || event.backtrace != nil {
            let sentryEvent = Event(level: .error)
            sentryEvent.message = SentryMessage(formatted: event.message)
            sentryEvent.tags = ["target": event.target]
            let trace = event.backtrace
            SentrySDK.capture(event: sentryEvent) { scope in
                guard let trace, let data = trace.data(using: .utf8) else { return }
                scope.addAttachment(Attachment(data: data, filename: "backtrace.txt", contentType: "text/plain"))
            }
        } else {
            let crumb = Breadcrumb(level: event.level == .warn ? .warning : .info, category: event.target)
            crumb.message = event.message
            SentrySDK.addBreadcrumb(crumb)
        }
    }
}
