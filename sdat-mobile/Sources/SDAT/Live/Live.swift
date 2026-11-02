import AVFoundation
import Observation
import UIKit

/// Listen Together: one host plays, guests follow on their own TIDAL accounts.
/// Protocol, clock sync and drift decisions live in the Rust core; this drives the Player.
@MainActor @Observable
final class Live {
    static let shared = Live()

    enum Mode: Equatable { case idle, starting, hosting, joining, joined, ended(String) }

    private(set) var mode: Mode = .idle
    private(set) var invite: String?
    private(set) var guests: [LiveGuestInfo] = []
    private(set) var hostName = ""
    private(set) var reconnecting = false
    /// Guest view of the host's queue (resolved lazily).
    private(set) var upNext: [LiveRow] = []
    var showSheet = false
    var showScanner = false
    /// Manual extra output delay (e.g. Bluetooth), in ms.
    /// Stored (so views observe it) and persisted.
    var extraLatencyMs: Int = UserDefaults.standard.integer(forKey: "liveExtraLatencyMs") {
        didSet { UserDefaults.standard.set(extraLatencyMs, forKey: "liveExtraLatencyMs") }
    }

    private var host: LiveHost?
    private var guest: LiveGuest?
    private var loop: Task<Void, Never>?
    /// Bumped on every start, join and teardown. Async results and core callbacks
    /// carry the value they began with and are dropped once it is stale.
    private var generation = 0
    /// Bumped by every refreshUpNext(): only the newest refresh may publish.
    private var upNextRefresh = 0
    private let player = Player.shared

    // host bookkeeping
    private var sentTrackIds: [String] = []
    private var itemIds: [String] = []
    private var sent: (item: String?, playing: Bool, pos: Double, at: Date)?

    // guest bookkeeping
    private var cache: [String: Track] = [:]
    private var switching: String?

    var active: Bool { [.starting, .hosting, .joining, .joined].contains(mode) }

    private var deviceName: String { UIDevice.current.name }

    // MARK: lifecycle

    func startHosting() async {
        guard !active else { return }
        mode = .starting
        generation += 1
        let gen = generation
        do {
            let h = try await LiveHost.start(name: deviceName, relays: nil, observer: LiveBridge(generation: gen))
            // Cancelled (or replaced) while the endpoint was coming up.
            guard gen == generation, mode == .starting else { h.end(); return }
            host = h
            invite = h.inviteUrl()
            mode = .hosting
            sentTrackIds = []; sent = nil
            run(every: 0.5) { $0.hostTick() }
        } catch {
            guard gen == generation else { return }
            mode = .ended(friendlyMessage(error))
        }
    }

    /// Handles a scanned QR or an opened `sdat://join/...` link.
    func open(_ text: String) {
        guard liveIsInvite(text: text) else { return }
        showScanner = false
        showSheet = true
        if active { leave() }
        generation += 1
        do {
            guest = try LiveGuest.join(invite: text, name: deviceName, relays: nil, observer: LiveBridge(generation: generation))
            mode = .joining
            switching = nil
            player.liveFollowing = true
            run(every: 0.25) { $0.guestTick() }
        } catch {
            mode = .ended("This invitation isn't valid.")
        }
    }

    func leave() {
        host?.end()
        guest?.leave()
        finish(.idle)
    }

    func dismissEnded() { if case .ended = mode { mode = .idle } }

    private func finish(_ m: Mode) {
        generation += 1
        loop?.cancel(); loop = nil
        host = nil; guest = nil
        switching = nil
        player.liveFollowing = false
        invite = nil; guests = []; upNext = []; reconnecting = false
        if player.isPlaying { player.liveSetRate(1) }
        mode = m
    }

    private func run(every secs: Double, _ tick: @escaping @MainActor (Live) -> Void) {
        loop?.cancel()
        loop = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { return }
                tick(self)
                try? await Task.sleep(for: .seconds(secs))
            }
        }
    }

    fileprivate func handle(_ u: LiveUpdate, generation gen: Int) {
        guard gen == generation, host != nil || guest != nil else { return }
        switch u {
        case .guests(let g): guests = g
        case .joined(let name):
            hostName = name; reconnecting = false
            if mode == .joining { mode = .joined }
            refreshUpNext()
        case .queueChanged: refreshUpNext()
        case .playbackChanged: reconnecting = false
        case .reconnecting: reconnecting = true
        case .ended(let r): finish(.ended(Self.message(r)))
        }
    }

    private static func message(_ r: LiveEnd) -> String {
        switch r {
        case .hostEnded: "The host ended the session."
        case .hostLost: "Lost connection to the host. The session ended."
        case .left: "You left the session."
        case .badInvite: "This invitation isn't valid."
        case .incompatible: "The host is using a different version of SDAT. Update both apps."
        case .full: "This session is full."
        }
    }

    // MARK: host

    private func hostTick() {
        guard let host else { return }
        var ids: [String] = []
        if let c = player.current { ids.append(c.id) }
        ids += player.userQueue.map(\.id)
        ids += player.upNext.map(\.id)
        ids = Array(ids.prefix(500))
        var force = false
        if ids != sentTrackIds {
            if let items = try? host.setQueue(trackIds: ids) {
                itemIds = items; sentTrackIds = ids; force = true
            }
        }
        let item = player.current == nil ? nil : itemIds.first
        let playing = player.isPlaying && !player.loading
        let pos = player.livePosition
        let now = Date()
        if !force, let s = sent, s.item == item, s.playing == playing {
            let expected = s.pos + (playing ? now.timeIntervalSince(s.at) : 0)
            if abs(expected - pos) < 0.25 { return }
        }
        host.setPlayback(itemId: item, trackId: player.current?.id, positionMs: Int64(pos * 1000), playing: playing)
        sent = (item, playing, pos, now)
    }

    // MARK: guest

    private var latencyMs: Int64 {
        Int64(AVAudioSession.sharedInstance().outputLatency * 1000) + Int64(extraLatencyMs)
    }

    private func guestTick() {
        guard let guest, mode == .joined else { return }
        guest.setSeekOnly(on: ProcessInfo.processInfo.isLowPowerModeEnabled)
        guard let t = guest.target(latencyMs: latencyMs) else { return }
        guard let trackId = t.trackId else {
            if player.isPlaying { player.toggle() }
            return
        }
        // Reload also when our copy ended (host repeating the same song).
        if player.current?.id != trackId || (!player.liveHasItem && !player.loading) {
            guard switching != trackId else { return }
            switching = trackId
            let gen = generation
            Task {
                let track = await resolve(trackId)
                guard gen == generation else { return }
                defer { if switching == trackId { switching = nil } }
                // Re-read the target: the host may have moved on while resolving.
                guard let track, let now = self.guest?.target(latencyMs: latencyMs),
                      now.trackId == trackId else { return }
                player.livePlay([track], index: 0, at: Double(now.positionMs) / 1000, playing: now.playing)
            }
            return
        }
        if player.loading || switching != nil { return }
        // Paused by the system (call, headphones unplugged): wait for the
        // user or the system to resume instead of restarting on the speaker.
        if t.playing && !player.isPlaying && player.systemPaused { return }
        if t.playing != player.isPlaying {
            player.toggle()
            if !t.playing { player.seek(to: Double(t.positionMs) / 1000) }
            return
        }
        guard t.playing else {
            if abs(player.livePosition * 1000 - Double(t.positionMs)) > 1000 {
                player.seek(to: Double(t.positionMs) / 1000)
            }
            return
        }
        switch guest.correction(latencyMs: latencyMs, actualMs: Int64(player.livePosition * 1000)) {
        case .none: break
        case .rate(let r): player.liveSetRate(Float(r))
        case .seek(let ms):
            player.seek(to: Double(ms) / 1000)
            player.liveSetRate(1) // a seek always ends any nudge
        }
    }

    private func resolve(_ id: String) async -> Track? {
        if let t = cache[id] { return t }
        guard let t = try? await liveTrack(id: id) else { return nil }
        cache[id] = t
        return t
    }

    private func refreshUpNext() {
        guard let guest else { return }
        // Entries after the one playing (by item id, so repeated tracks stay distinct).
        let entries = guest.queue()
        let playing = guest.target(latencyMs: 0)?.itemId
        let start = playing.flatMap { p in entries.firstIndex { $0.itemId == p } }.map { $0 + 1 } ?? 1
        let next = Array(entries.dropFirst(start).prefix(30))
        let gen = generation
        upNextRefresh &+= 1
        let refresh = upNextRefresh
        Task {
            var out: [LiveRow] = []
            for e in next { if let t = await resolve(e.trackId) { out.append(LiveRow(id: e.itemId, track: t)) } }
            if gen == generation && refresh == upNextRefresh { upNext = out }
        }
    }
}

/// A guest's upcoming entry, keyed by the shared queue item id so the same
/// song queued twice gets two rows.
struct LiveRow: Identifiable {
    let id: String
    let track: Track
}

/// Callbacks arrive on core threads; hop to the main actor.
final class LiveBridge: LiveObserver, @unchecked Sendable {
    private let generation: Int
    init(generation: Int) { self.generation = generation }
    func onUpdate(update: LiveUpdate) {
        let gen = generation
        Task { @MainActor in Live.shared.handle(update, generation: gen) }
    }
}
