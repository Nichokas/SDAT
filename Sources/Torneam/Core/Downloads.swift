import Foundation
import AVFoundation
import UIKit

/// Offline tracks. TIDAL only hands out HLS playlists, so a download fetches the
/// (fMP4) media playlist's init segment + fragments and concatenates them into one
/// fragmented MP4 that AVPlayer plays from disk. Encrypted (EXT-X-KEY) or MPEG-TS
/// streams are refused with a clear error.
/// Files live in Application Support/Downloads, excluded from iCloud backup;
/// `index.json` holds metadata, artwork path, quality and size.
@MainActor @Observable
final class Downloads {
    static let shared = Downloads()

    struct Entry: Codable {
        var track: DiscoverCache.T
        var file: String
        var cover: String?
        var quality: String
        var size: Int64
        var date: Date
        var gainDb, peak, albumGainDb, albumPeak: Double?
    }
    enum State: Equatable { case none, queued, downloading(Double), done, failed(String) }

    private(set) var entries: [String: Entry] = [:]
    private(set) var progress: [String: Double] = [:]
    private(set) var failures: [String: String] = [:]
    @ObservationIgnored private var pending: [Track] = []
    @ObservationIgnored private var worker: Task<Void, Never>?
    @ObservationIgnored private var currentID: String?
    /// Remote cover URL → local file, so covers show offline.
    @ObservationIgnored private var coverMap: [String: URL] = [:]

    nonisolated static var dir: URL {
        let d = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Downloads", isDirectory: true)
        if !FileManager.default.fileExists(atPath: d.path) {
            try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
            var u = d; var v = URLResourceValues(); v.isExcludedFromBackup = true
            try? u.setResourceValues(v)
        }
        return d
    }
    nonisolated private static var indexURL: URL { dir.appendingPathComponent("index.json") }

    private init() {
        if let d = try? Data(contentsOf: Self.indexURL),
           let e = try? JSONDecoder().decode([String: Entry].self, from: d) {
            // Drop entries whose file vanished.
            entries = e.filter { FileManager.default.fileExists(atPath: Self.dir.appendingPathComponent($0.value.file).path) }
        }
        rebuildCoverMap()
    }

    // MARK: queries

    func state(_ id: String) -> State {
        if entries[id] != nil { return .done }
        if let p = progress[id] { return currentID == id ? .downloading(p) : .queued }
        if let f = failures[id] { return .failed(f) }
        return .none
    }
    func isDownloaded(_ id: String) -> Bool { entries[id] != nil }
    func localURL(_ id: String) -> URL? { entries[id].map { Self.dir.appendingPathComponent($0.file) } }
    func localCover(_ id: String) -> URL? { entries[id]?.cover.map { Self.dir.appendingPathComponent($0) } }
    func coverFile(forRemote url: String) -> URL? { coverMap[url] }
    var totalSize: Int64 { entries.values.reduce(0) { $0 + $1.size } }
    var tracks: [Track] {
        entries.values.sorted { $0.date > $1.date }.compactMap { DiscoverCache.decode([$0.track]).first }
    }
    /// Queued or downloading, in order.
    var inProgress: [Track] { activeOrder.compactMap { progress[$0] != nil ? activeTracks[$0] : nil } }
    @ObservationIgnored private var activeTracks: [String: Track] = [:]
    private(set) var activeOrder: [String] = []
    func entry(_ id: String) -> Entry? { entries[id] }

    /// Stream record for a downloaded file, so Player can treat it like a remote stream.
    func localStream(_ id: String) -> Stream? {
        guard let e = entries[id], let u = localURL(id) else { return nil }
        let q = Quality(key: e.quality) ?? .hiFi
        return Stream(url: u.absoluteString, quality: q, formats: [],
                      trackGain: e.gainDb.map { Gain(gainDb: $0, peak: e.peak ?? 1) },
                      albumGain: e.albumGainDb.map { Gain(gainDb: $0, peak: e.albumPeak ?? 1) })
    }

    // MARK: actions

    /// Queues tracks; already downloaded / queued / unstreamable ones are skipped.
    func download(_ list: [Track]) {
        var added = 0
        for t in list where t.streamable && entries[t.id] == nil && progress[t.id] == nil {
            failures[t.id] = nil
            progress[t.id] = 0
            pending.append(t)
            activeTracks[t.id] = t; activeOrder.append(t.id)
            added += 1
        }
        if added > 0 { pump() }
    }

    func cancel(_ id: String) {
        pending.removeAll { $0.id == id }
        progress[id] = nil
        if currentID != id { activeOrder.removeAll { $0 == id }; activeTracks[id] = nil }
        if currentID == id { worker?.cancel() }
    }
    func cancelAll() { for id in Array(progress.keys) { cancel(id) } }

    func remove(_ id: String) {
        cancel(id)
        failures[id] = nil
        guard let e = entries.removeValue(forKey: id) else { return }
        try? FileManager.default.removeItem(at: Self.dir.appendingPathComponent(e.file))
        if let c = e.cover, !entries.values.contains(where: { $0.cover == c }) {
            try? FileManager.default.removeItem(at: Self.dir.appendingPathComponent(c))
        }
        saveIndex()
    }
    func remove(_ list: [Track]) { for t in list { remove(t.id) } }

    /// Cancels queued / in-flight downloads, waits for the worker to stop, then deletes
    /// every file (tracks, covers, index). Nothing can be recreated afterwards.
    func removeAll() async {
        pending = []
        progress = [:]; activeOrder = []; activeTracks = [:]
        while let w = worker { w.cancel(); await w.value; if worker == w { worker = nil } }
        failures = [:]; entries = [:]; coverMap = [:]
        let d = Self.dir
        await Task.detached(priority: .userInitiated) { try? FileManager.default.removeItem(at: d) }.value
    }

    // MARK: worker

    private func pump() {
        guard worker == nil else { return }
        worker = Task { [weak self] in
            while let self, let t = self.pending.first {
                self.pending.removeFirst()
                self.currentID = t.id
                self.progress[t.id] = 0
                let bg = UIApplication.shared.beginBackgroundTask(withName: "download") {}
                do {
                    let e = try await self.fetch(t)
                    if self.progress[t.id] != nil {          // not cancelled meanwhile
                        self.entries[t.id] = e
                        self.saveIndex()
                    } else { try? FileManager.default.removeItem(at: Self.dir.appendingPathComponent(e.file)) }
                } catch is CancellationError {
                } catch {
                    if self.progress[t.id] != nil { self.failures[t.id] = friendlyMessage(error) }
                }
                UIApplication.shared.endBackgroundTask(bg)
                self.progress[t.id] = nil
                self.activeOrder.removeAll { $0 == t.id }; self.activeTracks[t.id] = nil
                self.currentID = nil
                if Task.isCancelled { self.worker = nil; self.pump(); return }
            }
            self?.worker = nil
        }
    }

    enum Failure: LocalizedError {
        case encrypted, unsupported(String), unplayable
        var errorDescription: String? {
            switch self {
            case .encrypted: "This track's stream is encrypted (DRM) and can't be saved."
            case .unsupported(let w): "Unsupported stream format (\(w))."
            case .unplayable: "The downloaded file isn't playable."
            }
        }
    }

    private func session() -> URLSession {
        let c = URLSessionConfiguration.default
        c.allowsCellularAccess = Preferences.shared.downloadOverCellular
        c.waitsForConnectivity = false
        c.timeoutIntervalForRequest = 30
        return URLSession(configuration: c)
    }

    private func fetch(_ t: Track) async throws -> Entry {
        let net = session()
        defer { net.finishTasksAndInvalidate() }
        let s = try await Torneam.stream(trackId: t.id, quality: Preferences.shared.downloadQuality)
        var (text, base) = try await Self.playlist(s.url, net)
        if text.contains("#EXT-X-STREAM-INF") {
            let lines = text.split(whereSeparator: \.isNewline).map(String.init)
            guard let i = lines.firstIndex(where: { $0.hasPrefix("#EXT-X-STREAM-INF") }),
                  let uri = lines.dropFirst(i + 1).first(where: { !$0.hasPrefix("#") && !$0.isEmpty }),
                  let u = URL(string: uri, relativeTo: base) else { throw Failure.unsupported("master playlist") }
            (text, base) = try await Self.playlist(u.absoluteString, net)
        }
        let media = try Self.parse(text, base: base)

        let name = "\(t.id).mp4"
        let tmp = Self.dir.appendingPathComponent(name + ".part")
        let dst = Self.dir.appendingPathComponent(name)
        try? FileManager.default.removeItem(at: tmp)
        FileManager.default.createFile(atPath: tmp.path, contents: nil)
        let h = try FileHandle(forWritingTo: tmp)
        do {
            let parts = [media.map] + media.segments
            for (i, u) in parts.enumerated() {
                try Task.checkCancellation()
                guard progress[t.id] != nil else { throw CancellationError() }
                try h.write(contentsOf: try await Self.get(u, net))
                progress[t.id] = Double(i + 1) / Double(parts.count)
            }
            try h.close()
        } catch {
            try? h.close(); try? FileManager.default.removeItem(at: tmp); throw error
        }
        guard (try? await AVURLAsset(url: tmp).load(.isPlayable)) == true else {
            try? FileManager.default.removeItem(at: tmp); throw Failure.unplayable
        }
        if Task.isCancelled || progress[t.id] == nil { try? FileManager.default.removeItem(at: tmp); throw CancellationError() }
        try? FileManager.default.removeItem(at: dst)
        try FileManager.default.moveItem(at: tmp, to: dst)
        let size = (try? dst.resourceValues(forKeys: [.fileSizeKey]).fileSize).map(Int64.init) ?? 0

        var cover: String?
        if let a = t.album, let cu = URL(string: a.coverUrlLarge ?? a.coverUrl ?? "") {
            let cname = "cover-\(a.id).jpg"
            let cf = Self.dir.appendingPathComponent(cname)
            if FileManager.default.fileExists(atPath: cf.path) { cover = cname }
            else if let d = try? await Self.get(cu, net), !Task.isCancelled { try? d.write(to: cf, options: .atomic); cover = cname }
        }
        if let a = t.album, let cover {
            for r in [a.coverUrl, a.coverUrlLarge].compactMap({ $0 }) { coverMap[r] = Self.dir.appendingPathComponent(cover) }
        }
        return Entry(track: DiscoverCache.encode([t])[0], file: name, cover: cover, quality: s.quality.key,
                     size: size, date: Date(), gainDb: s.trackGain?.gainDb, peak: s.trackGain?.peak,
                     albumGainDb: s.albumGain?.gainDb, albumPeak: s.albumGain?.peak)
    }

    /// Downloads with 3 attempts (backoff 1 s, 2 s).
    private static func get(_ u: URL, _ net: URLSession) async throws -> Data {
        var last: Error = URLError(.unknown)
        for attempt in 0..<3 {
            try Task.checkCancellation()
            do {
                let (d, r) = try await net.data(from: u)
                if let h = r as? HTTPURLResponse, !(200..<300).contains(h.statusCode) { throw URLError(.badServerResponse) }
                return d
            } catch let e as URLError where e.code == .cancelled { throw CancellationError() }
            catch { last = error; try await Task.sleep(for: .seconds(attempt + 1)) }
        }
        throw last
    }

    private static func playlist(_ s: String, _ net: URLSession) async throws -> (String, URL?) {
        if s.hasPrefix("data:"), let comma = s.firstIndex(of: ",") {
            let meta = s[..<comma], body = String(s[s.index(after: comma)...])
            let data = meta.hasSuffix(";base64") ? Data(base64Encoded: body) : body.removingPercentEncoding?.data(using: .utf8)
            guard let data, let text = String(data: data, encoding: .utf8) else { throw Failure.unsupported("manifest") }
            return (text, nil)
        }
        guard let u = URL(string: s) else { throw Failure.unsupported("manifest URL") }
        let d = try await get(u, net)
        guard let text = String(data: d, encoding: .utf8) else { throw Failure.unsupported("manifest") }
        return (text, u)
    }

    private static func parse(_ text: String, base: URL?) throws -> (map: URL, segments: [URL]) {
        var map: URL?, segs: [URL] = []
        for raw in text.split(whereSeparator: \.isNewline) {
            let l = raw.trimmingCharacters(in: .whitespaces)
            if l.hasPrefix("#EXT-X-KEY") || l.hasPrefix("#EXT-X-SESSION-KEY") {
                if !l.contains("METHOD=NONE") { throw Failure.encrypted }
            } else if l.hasPrefix("#EXT-X-BYTERANGE") || (l.hasPrefix("#EXT-X-MAP") && l.contains("BYTERANGE")) {
                throw Failure.unsupported("byte ranges")
            } else if l.hasPrefix("#EXT-X-MAP"), let r = l.range(of: #"URI="([^"]+)""#, options: .regularExpression) {
                let uri = l[r].dropFirst(5).dropLast()
                map = URL(string: String(uri), relativeTo: base)?.absoluteURL
            } else if !l.isEmpty, !l.hasPrefix("#") {
                guard let u = URL(string: l, relativeTo: base)?.absoluteURL else { throw Failure.unsupported("segment URL") }
                segs.append(u)
            }
        }
        guard let map else { throw Failure.unsupported(segs.isEmpty ? "empty playlist" : "MPEG-TS") }
        guard !segs.isEmpty else { throw Failure.unsupported("empty playlist") }
        return (map, segs)
    }

    // MARK: index

    private func saveIndex() {
        rebuildCoverMap()
        let snapshot = entries
        Task.detached(priority: .utility) {
            try? JSONEncoder().encode(snapshot).write(to: Downloads.indexURL, options: .atomic)  // fails if the folder was wiped
        }
    }
    private func rebuildCoverMap() {
        coverMap = [:]
        for e in entries.values {
            guard let c = e.cover else { continue }
            for r in [e.track.cover, e.track.coverLarge].compactMap({ $0 }) { coverMap[r] = Self.dir.appendingPathComponent(c) }
        }
    }
}
