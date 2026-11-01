import Foundation

/// Human-readable text for any error shown in the UI (never the raw FfiError dump).
func friendlyMessage(_ error: Error) -> String {
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
