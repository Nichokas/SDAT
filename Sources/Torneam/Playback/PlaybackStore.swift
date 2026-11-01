import Foundation

/// Playback state persisted across launches (Application Support/playback.json).
enum PlaybackStore {
    struct State: Codable {
        var queue: [DiscoverCache.T]
        var original: [DiscoverCache.T]
        var userQueue: [DiscoverCache.T]
        var current: DiscoverCache.T
        var index: Int?
        var position: Double
        var repeatMode: String
        var shuffle: Bool
        var infinite: Bool
        var sourceTitle: String?
    }
    private static let io = DispatchQueue(label: "torneam.playback.store", qos: .utility)
    private static var url: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("playback.json")
    }
    /// Encodes + writes off the main thread.
    static func save(_ s: State) { io.async { try? JSONEncoder().encode(s).write(to: url, options: .atomic) } }
    static func clear() { io.async { try? FileManager.default.removeItem(at: url) } }
    /// Off the main thread; a corrupt file is deleted and nil returned.
    static func load() async -> State? {
        await withCheckedContinuation { c in
            io.async {
                guard let d = try? Data(contentsOf: url) else { return c.resume(returning: nil) }
                if let s = try? JSONDecoder().decode(State.self, from: d) { c.resume(returning: s) }
                else { try? FileManager.default.removeItem(at: url); c.resume(returning: nil) }
            }
        }
    }
}
