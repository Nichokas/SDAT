import Foundation

/// Warm-up after a fresh sign-in (never on launch): builds a varied Discover pool
/// (saved to discover.json, deduplicated by track, album and cover) and a first look at
/// the user's library (first pages + their covers in ImageStore), in parallel.
/// Weighted progress: Discover 50, playlists 15, albums 15, favourites/artists 10,
/// cover prefetch 10. Each step has its own timeout; failures count as done; 25 s cap.
@MainActor @Observable
final class Boot {
    static let shared = Boot()
    private(set) var active = false
    private var parts: [String: Double] = [:]
    private static let weights: [String: Double] = ["discover": 0.5, "playlists": 0.15, "albums": 0.15, "favs": 0.1, "covers": 0.1]
    var progress: Double { Self.weights.reduce(0) { $0 + $1.value * min(1, parts[$1.key] ?? 0) } }

    /// Distinct covers wanted before Discover counts as ready.
    static let targetCovers = 100

    @ObservationIgnored private var covers: [String] = []
    @ObservationIgnored private var run: Task<Void, Never>?

    func start() {
        run?.cancel()
        parts = [:]; covers = []
        active = true
        run = Task { await warmUp() }
    }

    private func set(_ k: String, _ f: Double) { if f > (parts[k] ?? 0) { parts[k] = f } }  // monotonic

    private func warmUp() async {
        let begin = Date()
        var done = 0
        let steps: [(String, Double, () async -> Void)] = [
            ("discover", 15, { await self.discover() }),
            ("playlists", 8, { if let p = try? await collectionPlaylists().nextPage() { self.addCovers(p.prefix(24).map(\.coverUrl)) } }),
            ("albums", 8, { if let a = try? await collectionAlbums().nextPage() { self.addCovers(a.prefix(24).map(\.coverUrl)) } }),
            ("favs", 8, {
                async let t = try? collectionTracks().nextPage()
                async let a = try? collectionArtists().nextPage()
                if let t = await t { Favs.shared.mark(t.map(\.id), true) }
                if let a = await a { self.addCovers(a.prefix(24).map(\.pictureUrl)) }
            }),
        ]
        var libraryDone = 0
        for (k, timeout, op) in steps {
            Task {
                _ = await Self.timed(timeout) { await op(); return true }
                self.set(k, 1); done += 1
                if k != "discover" { libraryDone += 1 }
            }
        }
        // Cover prefetch once the library lists are in (or timed out).
        Task {
            while libraryDone < 3, Date().timeIntervalSince(begin) < 10 { try? await Task.sleep(for: .milliseconds(100)) }
            let list = covers.map { CoverURL.sized($0, points: 52) }
            var n = 0
            _ = await Self.timed(6) {
                await withTaskGroup(of: Void.self) { g in
                    for u in list { g.addTask { _ = await ImageStore.shared.image(u) } }
                    for await _ in g { n += 1; self.set("covers", Double(n) / Double(max(list.count, 1))) }
                }
                return true
            }
            self.set("covers", 1); done += 1
        }
        while done < 5, Date().timeIntervalSince(begin) < 25 { try? await Task.sleep(for: .milliseconds(100)) }
        for k in Self.weights.keys { set(k, 1) }
        try? await Task.sleep(for: .milliseconds(350))   // let the ring visibly reach 100%
        active = false
    }

    private func addCovers(_ urls: [String?]) { covers += urls.compactMap { $0 } }

    /// Same source and dedupe as DiscoverView.load, but continues until the pool is varied.
    private func discover() async {
        guard let mixes = try? await homeMixes() else { return }
        let source = Shuffle(mixes: mixes)
        var pool: [Track] = []
        var ids = Set<String>(), albums = Set<String>(), seen = Set<String>()
        for _ in 0..<12 {
            guard !Task.isCancelled, let batch = try? await source.nextBatch(min: 60) else { break }
            for t in batch {
                guard let a = t.album, let c = a.coverUrl else { continue }
                if ids.insert(t.id).inserted, albums.insert(a.id).inserted, seen.insert(c).inserted { pool.append(t) }
            }
            if !pool.isEmpty { DiscoverCache.save(pool) }   // incremental: a timeout keeps what we have
            set("discover", Double(pool.count) / Double(Self.targetCovers))
            if pool.count >= 320 || (pool.count >= Self.targetCovers && batch.isEmpty) { break }
            if pool.count >= Self.targetCovers * 2 { break }
        }
    }

    /// Runs `op`, returning nil if it hasn't finished within `s` seconds (it is cancelled).
    private static func timed<T>(_ s: Double, _ op: @escaping @MainActor () async -> T?) async -> T? {
        await withCheckedContinuation { c in
            var fired = false
            let work = Task { @MainActor in
                let v = await op()
                if !fired { fired = true; c.resume(returning: v) }
            }
            Task { @MainActor in
                try? await Task.sleep(for: .seconds(s))
                if !fired { fired = true; work.cancel(); c.resume(returning: nil) }
            }
        }
    }
}
