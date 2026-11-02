import os
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
    /// Host: whether guests may play/pause, skip and add songs (off by default).
    /// Guest: whether the host currently allows that.
    private(set) var guestControl = false
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
    /// Guest: queue requests waiting for room in the command budget.
    private var outbox: [[String]] = []
    private var outboxFailures = 0
    private var outboxTask: Task<Void, Never>?

    // host: accepted guest track ids, fetched one at a time in arrival order
    private var hostEnqueue: [(gen: Int, id: String)] = []
    private var hostEnqueueTask: Task<Void, Never>?

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
            guestControl = false
            h.setControlObserver(observer: LiveControlBridge(generation: gen))
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
            guestControl = false
            guest?.setControlObserver(observer: LiveControlBridge(generation: generation))
            mode = .joining
            // Only an accepted invitation clears an older system pause.
            player.clearSystemPause()
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
        host?.setControlObserver(observer: nil)
        guest?.setControlObserver(observer: nil)
        host = nil; guest = nil
        guestControl = false
        outbox = []; outboxFailures = 0; outboxTask?.cancel(); outboxTask = nil
        hostEnqueue = []; hostEnqueueTask?.cancel(); hostEnqueueTask = nil
        switching = nil
        lastSeekAt = nil; seekBackoff = Self.seekBackoffMin; lastTarget = nil
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
        case .playbackChanged: if reconnecting { reconnecting = false }
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

    // MARK: guest playback control

    var isGuest: Bool { guest != nil }
    /// A guest who may not play/pause, skip or add songs right now.
    var playbackLocked: Bool { guest != nil && !guestControl }

    /// Host: lets guests play/pause, skip and add songs.
    func setGuestControl(_ allowed: Bool) {
        guard let host else { return }
        host.setGuestControl(allowed: allowed)
        guestControl = host.guestControl()
        if !guestControl { hostEnqueue = [] }
    }

    fileprivate func controlChanged(_ allowed: Bool, generation gen: Int) {
        guard gen == generation, guest != nil else { return }
        guestControl = allowed
        if !allowed { outbox = []; outboxFailures = 0 }
    }

    /// Host: applies an accepted guest command through the normal player path;
    /// the regular host tick then publishes the result to every guest.
    fileprivate func guestCommand(_ cmd: LiveCommand, generation gen: Int) {
        guard gen == generation, let host, host.guestControl() else { return }
        switch cmd {
        case .play: if !player.isPlaying { player.toggle() }
        case .pause: if player.isPlaying { player.toggle() }
        case .next: player.next()
        case .enqueue(let ids):
            hostEnqueue += ids.map { (gen, $0) }
            pumpHostEnqueue()
        }
    }

    private func guestRequestValid(_ gen: Int) -> Bool {
        gen == generation && host?.guestControl() == true
    }

    private func pumpHostEnqueue() {
        guard hostEnqueueTask == nil else { return }
        let session = generation
        hostEnqueueTask = Task {
            while !hostEnqueue.isEmpty {
                let (gen, id) = hostEnqueue.removeFirst()
                guard guestRequestValid(gen) else { continue }
                let track = try? await liveTrack(id: id)
                // The session may have ended or control been turned off meanwhile.
                guard let track, guestRequestValid(gen) else { continue }
                player.addToQueue(track)
            }
            if session == generation { hostEnqueueTask = nil }
        }
    }

    /// Returns `true` when Listen Together handled the action (sent to the
    /// host or refused), `false` when the caller should act locally.
    private func route(_ cmd: LiveCommand) -> Bool {
        guard let guest else { return false }
        guard guestControl else { refuse(); return true }
        if !guest.sendCommand(command: cmd) { Chrome.shared.flash("Couldn't reach the host") }
        return true
    }

    private func refuse() { Chrome.shared.flash("The host controls playback") }

    /// Play/pause from the UI or the lock screen.
    func userToggle() {
        let hostPlaying = guest?.target(latencyMs: 0)?.playing ?? false
        if !route(hostPlaying ? .pause : .play) { player.toggle() }
    }

    func userPlay() { if !route(.play) { if !player.isPlaying { player.toggle() } } }
    func userPause() { if !route(.pause) { if player.isPlaying { player.toggle() } } }
    func userNext() { if !route(.next) { player.next() } }

    /// Previous track and seeking: guests always follow the host.
    func guestRefuses() -> Bool {
        guard guest != nil else { return false }
        refuse()
        return true
    }

    /// "Add to Queue" / "Play Next". Guests ask the host to append the songs;
    /// returns `false` when the caller should queue locally.
    func userEnqueue(_ tracks: [Track]) -> Bool {
        guard guest != nil else { return false }
        guard guestControl else { refuse(); return true }
        let ids = tracks.map(\.id)
        outbox += stride(from: 0, to: ids.count, by: Self.maxEnqueue).map { Array(ids[$0..<min($0 + Self.maxEnqueue, ids.count)]) }
        pumpOutbox()
        Chrome.shared.flash("Sent to the host's queue")
        return true
    }

    private static let maxEnqueue = 50
    private static let outboxRetries = 20

    /// Sends queue requests in order; when the command budget refuses one it
    /// waits and retries (about 10 s) instead of dropping it.
    private func pumpOutbox() {
        guard outboxTask == nil else { return }
        let gen = generation
        outboxTask = Task {
            while let chunk = outbox.first, gen == generation, let guest, guestControl {
                if guest.sendCommand(command: .enqueue(trackIds: chunk)) {
                    outbox.removeFirst(); outboxFailures = 0
                    continue
                }
                outboxFailures += 1
                if outboxFailures >= Self.outboxRetries {
                    outbox = []; outboxFailures = 0
                    Chrome.shared.flash("Couldn't reach the host")
                    break
                }
                try? await Task.sleep(for: .milliseconds(500))
            }
            if gen == generation { outboxTask = nil }
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
        // Seek-only mode turns every 30 ms of clock jitter into an audible
        // exact seek; rate nudges are much gentler, so always use them.
        guest.setSeekOnly(on: false)
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
                // A system pause (call, headphones unplugged) also holds a new
                // host track: it loads paused until the user or system resumes.
                player.livePlay([track], index: 0, at: Double(now.positionMs) / 1000,
                                playing: now.playing && !player.systemPaused)
            }
            return
        }
        // Don't measure drift mid-seek or while rebuffering: the position is
        // frozen there, so the error looks huge and would trigger seek after seek.
        if player.loading || switching != nil || player.liveSeeking { return }
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
        guard player.liveAdvancing else { return }
        let now = Date()
        // The host itself jumped (seek, restart): follow it at once, the
        // seek backoff only exists to stop our own correction loop.
        if let last = lastTarget, abs(Double(t.positionMs - last.ms) - now.timeIntervalSince(last.at) * 1000) > Self.hostJumpMs {
            lastSeekAt = nil; seekBackoff = Self.seekBackoffMin
        }
        lastTarget = (t.positionMs, now)
        // Right after a seek the position isn't trustworthy yet (the item is
        // still refilling), so measuring then reports a large error and
        // seeks again: a silent gap every ~0.7 s. Let it settle first.
        if let s = player.liveSeekEndedAt, now.timeIntervalSince(s) < Self.seekSettle { return }
        let actual = Int64(player.livePosition * 1000)
        switch guest.correction(latencyMs: latencyMs, actualMs: actual) {
        case .none: break
        case .rate(let r):
            Self.log.debug("drift nudge rate=\(r) actualMs=\(actual)")
            player.liveSetRate(Float(r))
        case .seek(let ms):
            // A seek that didn't fix the error means the error isn't real
            // drift (or seeking can't fix it): back off instead of looping.
            if let last = lastSeekAt, now.timeIntervalSince(last) < seekBackoff {
                Self.log.debug("drift seek skipped (backoff \(self.seekBackoff)s) errMs=\(ms - actual)")
                player.liveSetRate(1) // core ended its nudge with this seek decision
                return
            }
            seekBackoff = lastSeekAt.map { now.timeIntervalSince($0) < self.seekBackoff * 2 } == true
                ? min(seekBackoff * 2, Self.seekBackoffMax) : Self.seekBackoffMin
            lastSeekAt = now
            Self.log.notice("drift seek errMs=\(ms - actual) next backoff=\(self.seekBackoff)s")
            player.liveSeek(to: Double(ms) / 1000)
            player.liveSetRate(1) // a seek always ends any nudge
        }
    }

    private static let log = Logger(subsystem: "dev.sdat", category: "live")
    private static let seekSettle: TimeInterval = 1.0
    private static let seekBackoffMin: TimeInterval = 2
    private static let seekBackoffMax: TimeInterval = 16
    /// A host target jump larger than the core seek threshold is a host seek, not drift.
    private static let hostJumpMs: Double = 300
    private var lastSeekAt: Date?
    private var lastTarget: (ms: Int64, at: Date)?
    private var seekBackoff: TimeInterval = Live.seekBackoffMin

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

/// Guest-control callbacks, also from core threads.
final class LiveControlBridge: LiveControlObserver, @unchecked Sendable {
    private let generation: Int
    init(generation: Int) { self.generation = generation }
    func onControlChanged(allowed: Bool) {
        let gen = generation
        Task { @MainActor in Live.shared.controlChanged(allowed, generation: gen) }
    }
    func onGuestCommand(peer: String, name: String, command: LiveCommand) {
        let gen = generation
        Task { @MainActor in Live.shared.guestCommand(command, generation: gen) }
    }
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
