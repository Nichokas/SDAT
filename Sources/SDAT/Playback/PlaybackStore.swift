import Foundation

/// Playback state persisted across launches, in Application Support:
/// - `playback-queue.json`: the (windowed) queues; rewritten only when they change.
/// - `playback-state.json`: tiny (current track, index, position, flags); rewritten often.
enum PlaybackStore {
    struct QueueFile: Codable {
        var queue: [DiscoverCache.T]
        var original: [DiscoverCache.T]
        var userQueue: [DiscoverCache.T]
        var infinite: Bool
        var sourceTitle: String?
        /// Matches `StateFile.queueID`, so a state never pairs with a stale queue.
        var id: Int
    }
    struct StateFile: Codable {
        var current: DiscoverCache.T
        var index: Int?
        var position: Double
        var repeatMode: String
        var shuffle: Bool
        var queueID: Int
    }

    /// Played tracks kept around the current one for endless queues / huge queues.
    static let endlessBehind = 50, endlessAhead = 200
    static let maxSaved = 2000

    private static let io = DispatchQueue(label: "sdat.playback.store", qos: .utility)
    private static var dir: URL {
        let d = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }
    private static var queueURL: URL { dir.appendingPathComponent("playback-queue.json") }
    private static var stateURL: URL { dir.appendingPathComponent("playback-state.json") }
    private static var legacyURL: URL { dir.appendingPathComponent("playback.json") }

    /// JSON encoding and writing happen on the background queue.
    static func saveQueue(_ q: QueueFile) { io.async { try? JSONEncoder().encode(q).write(to: queueURL, options: .atomic) } }
    static func saveState(_ s: StateFile) { io.async { try? JSONEncoder().encode(s).write(to: stateURL, options: .atomic) } }
    static func clear() {
        io.async { for u in [queueURL, stateURL, legacyURL] { try? FileManager.default.removeItem(at: u) } }
    }
    /// Reads and decodes off the main thread; corrupt / mismatched files are dropped.
    static func load() async -> (StateFile, QueueFile?)? {
        await withCheckedContinuation { c in
            io.async {
                try? FileManager.default.removeItem(at: legacyURL)
                guard let d = try? Data(contentsOf: stateURL) else { return c.resume(returning: nil) }
                guard let s = try? JSONDecoder().decode(StateFile.self, from: d) else {
                    for u in [queueURL, stateURL] { try? FileManager.default.removeItem(at: u) }
                    return c.resume(returning: nil)
                }
                let q = (try? Data(contentsOf: queueURL)).flatMap { try? JSONDecoder().decode(QueueFile.self, from: $0) }
                c.resume(returning: (s, q?.id == s.queueID ? q : nil))
            }
        }
    }

    /// Range of `count` items to persist around `index`.
    static func window(count: Int, index: Int?, endless: Bool) -> Range<Int> {
        let i = min(max(index ?? 0, 0), max(count - 1, 0))
        let behind = endless ? endlessBehind : maxSaved / 4
        let ahead = endless ? endlessAhead : maxSaved - behind
        var lo = max(0, i - behind), hi = min(count, i + ahead + 1)
        if !endless, hi - lo < maxSaved { lo = max(0, min(lo, hi - maxSaved)); hi = min(count, max(hi, lo + maxSaved)) }
        return lo..<hi
    }
}
