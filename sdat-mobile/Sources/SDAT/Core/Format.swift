import Foundation

extension Track {
    var artistLine: String { artists.map(\.name).joined(separator: ", ") }
    var coverURL: URL? { album?.coverUrl.flatMap(URL.init(string:)) }
    var coverURLLarge: URL? { (album?.coverUrlLarge ?? album?.coverUrl).flatMap(URL.init(string:)) }
    var displayTitle: String { version.isEmpty ? title : "\(title) (\(version))" }
}

extension Album {
    var artistLine: String { artists.map(\.name).joined(separator: ", ") }
}

enum Fmt {
    static func time(_ secs: Double) -> String {
        guard secs.isFinite, secs >= 0 else { return "0:00" }
        let s = Int(secs)
        return String(format: "%d:%02d", s / 60, s % 60)
    }

    /// "12 songs · 48 min"
    static func summary(count: UInt32, secs: UInt32) -> String {
        let mins = Int(secs) / 60
        let dur = mins >= 60 ? "\(mins / 60) h \(mins % 60) min" : "\(mins) min"
        return "\(count) \(count == 1 ? "song" : "songs") · \(dur)"
    }

    static func updated(_ ts: Int64?) -> String? {
        guard let ts else { return nil }
        let date = Date(timeIntervalSince1970: TimeInterval(ts))
        if Calendar.current.isDateInToday(date) { return "Updated today" }
        if Calendar.current.isDateInYesterday(date) { return "Updated yesterday" }
        return "Updated " + date.formatted(date: .abbreviated, time: .omitted)
    }
    static func bytes(_ n: Int64) -> String {
        ByteCountFormatter.string(fromByteCount: n, countStyle: .file)
    }

    /// "just now", "5 min ago", "2 h ago", "3 d ago".
    static func ago(_ date: Date, now: Date = Date()) -> String {
        let mins = Int(now.timeIntervalSince(date) / 60)
        if mins < 1 { return "just now" }
        if mins < 60 { return "\(mins) min ago" }
        if mins < 24 * 60 { return "\(mins / 60) h ago" }
        return "\(mins / (24 * 60)) d ago"
    }
}
