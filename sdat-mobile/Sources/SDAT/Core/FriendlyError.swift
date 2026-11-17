import Foundation

/// Human-readable text for any error shown in the UI (never the raw FfiError dump).
func friendlyMessage(_ error: Error, file: StaticString = #fileID, line: UInt = #line) -> String {
    let level = diagLevel(for: error)
    // Frames only, and only for the unexpected errors, which become Sentry reports.
    diagRecord(level: level, target: "app", message: "\(diagSummary(error)) at \(file):\(line)",
               backtrace: level == .error ? Thread.callStackSymbols.joined(separator: "\n") : nil)
    guard let e = error as? FfiError else {
        if (error as? URLError) != nil { return "Can't reach TIDAL. Check your connection." }
        return "Something went wrong. Please try again."
    }
    switch e {
    case .NotSignedIn: return "You're signed out. Sign in to TIDAL again."
    case .Network: return "Can't reach TIDAL. Check your connection."
    case .Http(let status, _):
        switch status {
        case 401, 403: return "TIDAL refused the request. Try signing in again."
        case 404: return "Not found on TIDAL."
        case 429: return "Too many requests. Wait a moment and try again."
        case 500...: return "TIDAL is having trouble right now. Try again later."
        default: return "TIDAL couldn't handle this request (\(status))."
        }
    case .Decode: return "TIDAL sent something unexpected. Try again."
    case .LastFm(_, let message): return "Last.fm: \(message)"
    case .Cancelled: return "Cancelled."
    case .Other: return "Something went wrong. Please try again."
    }
}

/// Offline, cancelled, signed-out, Last.fm and 4xx errors are expected: they stay in the log and become Sentry
/// breadcrumbs, while anything else is reported as an error.
private func diagLevel(for error: Error) -> DiagLevel {
    if error is URLError { return .warn }
    // A Rust panic surfaces as the generated, fileprivate `UniffiInternalError.rustPanic`; the core's panic hook
    // already reported it with its own backtrace. The type's other cases are binding bugs and stay errors.
    if (error as NSError).domain.hasSuffix("UniffiInternalError"), String(describing: error).hasPrefix("rustPanic(") {
        return .warn
    }
    switch error as? FfiError {
    case .Network?, .Cancelled?, .NotSignedIn?, .LastFm?: return .warn
    case .Http(let status, _)? where status < 500: return .warn
    default: return .error
    }
}

/// Kind and status only: error details can hold request URLs or response bodies, and the log goes to
/// os_log and, with consent, Sentry.
private func diagSummary(_ error: Error) -> String {
    switch error {
    case let e as URLError: return "URLError \(e.code.rawValue)"
    case let e as FfiError:
        switch e {
        case .NotSignedIn: return "Not signed in"
        case .Network: return "Network error"
        case .Http(let status, _): return "HTTP \(status)"
        case .Decode: return "Unexpected response"
        case .LastFm(let code, _): return "Last.fm error \(code)"
        case .Cancelled: return "Cancelled"
        case .Other: return "Other error"
        }
    default:
        let ns = error as NSError
        return "\(ns.domain) \(ns.code)"
    }
}
