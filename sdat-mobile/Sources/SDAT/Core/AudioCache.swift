import Foundation
import Observation

@MainActor @Observable
final class AudioCache {
    static let shared = AudioCache()

    private(set) var usage = CacheUsage(bytes: 0, tracks: 0, limitBytes: 0)
    /// Most recently played first.
    private(set) var tracks: [CachedTrack] = []

    /// In Caches, so it stays out of backups; iOS may empty it when storage runs low, which the core tolerates.
    static let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("AudioCache", isDirectory: true)

    private init() { refresh() }

    static func open() {
        try? audioCacheOpen(dir: dir.path, limitBytes: Preferences.shared.audioCacheLimit)
    }

    func refresh() {
        usage = audioCacheUsage()
        tracks = audioCacheTracks()
    }

    func remove(_ id: String) {
        audioCacheRemove(trackId: id)
        refresh()
    }

    func purge() {
        audioCachePurge()
        refresh()
    }
}
