import Foundation
import Observation
import os

@MainActor @Observable
final class AudioCache {
    static let shared = AudioCache()
    nonisolated static let log = Logger(subsystem: "com.nichokas.SDAT", category: "audio-cache")

    private(set) var usage = CacheUsage(bytes: 0, tracks: 0, limitBytes: 0)
    /// Most recently played first.
    private(set) var tracks: [CachedTrack] = []

    /// In Caches, so it stays out of backups; iOS may empty it when storage runs low, which the core tolerates.
    static let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("AudioCache", isDirectory: true)

    private init() { refresh() }

    static func open() {
        do {
            try audioCacheOpen(dir: dir.path, limitBytes: Preferences.shared.audioCacheLimit)
        } catch {
            log.error("failed to open the audio cache: \(String(describing: error), privacy: .public)")
        }
    }

    func refresh() {
        usage = audioCacheUsage()
        tracks = audioCacheTracks()
    }

    func remove(_ id: String) {
        audioCacheRemove(trackId: id)
        refresh()
    }

    /// `keepPlaying` for Clear Cache; false once playback stopped, as at sign-out.
    func purge(keepPlaying: Bool) {
        audioCachePurge(keepPlaying: keepPlaying)
        refresh()
    }
}
