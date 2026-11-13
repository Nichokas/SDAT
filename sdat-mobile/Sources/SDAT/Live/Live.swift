import os
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
    /// Manual extra output delay, in ms. Stored (so views observe it) and persisted.
    var extraLatencyMs: Int = UserDefaults.standard.integer(forKey: "liveExtraLatencyMsV2") {
        didSet { UserDefaults.standard.set(extraLatencyMs, forKey: "liveExtraLatencyMsV2") }
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
    private var outbox: [(ids: [String], next: Bool)] = []
    private var outboxFailures = 0
    private var outboxTask: Task<Void, Never>?

    // host: accepted guest track ids, fetched one at a time in arrival order
    private var hostEnqueue: [(epoch: Int, id: String, next: Bool)] = []
    private var hostEnqueueTask: Task<Void, Never>?
    /// Bumped when a session ends or control is turned off: requests (and the
    /// worker) from an older epoch are dropped, even mid-fetch.
    private var enqueueEpoch = 0

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
            h.setPlayNextObserver(observer: LiveControlBridge(generation: gen))
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
            // The permission may have arrived before the observer was installed.
            guestControl = guest?.guestControl() ?? false
            mode = .joining
            // Only an accepted invitation clears an older system pause.
            player.clearSystemPause()
            switching = nil
            player.liveFollowing = true
            player.liveTarget = { [weak self] in self?.hostPosition() }
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
        lastPath = nil; lastDiagAt = .distantPast; lastDriftLogAt = .distantPast; alignedLatencyMs = nil
        loop?.cancel(); loop = nil
        host?.setControlObserver(observer: nil)
        host?.setPlayNextObserver(observer: nil)
        guest?.setControlObserver(observer: nil)
        host = nil; guest = nil
        guestControl = false
        outbox = []; outboxFailures = 0; outboxTask?.cancel(); outboxTask = nil
        dropGuestEnqueues()
        switching = nil
        player.liveFollowing = false
        player.liveTarget = nil
        invite = nil; guests = []; upNext = []; reconnecting = false
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
        if !guestControl { dropGuestEnqueues() }
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
        // hostTick republishes on the next tick (0.5 s): the jump is sent at once.
        case .seek(let ms): player.seek(to: Double(max(ms, 0)) / 1000)
        case .enqueue(let ids):
            hostEnqueue += ids.map { (enqueueEpoch, $0, false) }
            pumpHostEnqueue()
        }
    }

    /// Host: a guest's Play Next (accepted only while control is on).
    fileprivate func guestPlayNext(_ ids: [String], generation gen: Int) {
        guard gen == generation, let host, host.guestControl() else { return }
        hostEnqueue += ids.map { (enqueueEpoch, $0, true) }
        pumpHostEnqueue()
    }

    private func dropGuestEnqueues() {
        enqueueEpoch += 1
        hostEnqueue = []
        hostEnqueueTask?.cancel(); hostEnqueueTask = nil
    }

    private func guestRequestValid(_ epoch: Int) -> Bool {
        epoch == enqueueEpoch && host?.guestControl() == true
    }

    /// One worker per epoch fetches guest tracks in arrival order. A worker
    /// from an older epoch stops (checked again after every fetch), so it can
    /// neither append a dropped track nor race a newer worker.
    private func pumpHostEnqueue() {
        guard hostEnqueueTask == nil else { return }
        let epoch = enqueueEpoch
        hostEnqueueTask = Task {
            while epoch == enqueueEpoch, !Task.isCancelled, !hostEnqueue.isEmpty {
                let (e, id, next) = hostEnqueue.removeFirst()
                guard guestRequestValid(e) else { continue }
                let track = try? await liveTrack(id: id)
                guard let track, !Task.isCancelled, guestRequestValid(e), let host else { continue }
                if next {
                    // Publish before asking (playback may have moved on since
                    // the last tick) and after inserting, so the core's Play
                    // Next run matches the queue.
                    hostTick()
                    player.insertUpNext(track, at: Int(host.playNextOffset(trackIds: [id])))
                    hostTick()
                } else {
                    player.addToQueue(track)
                }
            }
            if epoch == enqueueEpoch { hostEnqueueTask = nil }
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
        if hostPlaying, resumeLocally() { return }
        let handled = hostPlaying ? route(.pause) : requestPlay()
        if !handled { player.toggle() }
    }

    func userPlay() {
        if guest?.target(latencyMs: 0)?.playing == true, resumeLocally() { return }
        if !requestPlay(), !player.isPlaying { player.toggle() }
    }

    /// Asks the host to play. An explicit play also lifts a local system
    /// pause, otherwise the guest would stay silent once the host resumes.
    /// Returns `false` when the caller should act locally (not a guest).
    private func requestPlay() -> Bool {
        guard route(.play) else { return false }
        if guestControl, player.systemPaused { player.clearSystemPause() }
        return true
    }

    /// Guest paused by the system (call, headphones unplugged) while the host
    /// plays: "play" means resume here, not change the host.
    private func resumeLocally() -> Bool {
        guard guest != nil, player.systemPaused else { return false }
        player.clearSystemPause()
        return true
    }
    func userPause() { if !route(.pause) { if player.isPlaying { player.toggle() } } }
    func userNext() { if !route(.next) { player.next() } }

    /// Seeking. Guests with control ask the host, which moves everyone at
    /// once; returns `false` when the caller should seek locally.
    func userSeek(_ secs: Double) -> Bool {
        if let guest, guestControl, !guest.hostSupportsSeek() {
            Chrome.shared.flash("The host's app is too old to seek for everyone")
            return true
        }
        return route(.seek(positionMs: Int64(max(secs, 0) * 1000)))
    }

    /// Previous track: guests always follow the host.
    func guestRefuses() -> Bool {
        guard guest != nil else { return false }
        refuse()
        return true
    }

    /// "Add to Queue". Guests ask the host to append the songs; returns
    /// `false` when the caller should queue locally.
    func userEnqueue(_ tracks: [Track]) -> Bool {
        send(tracks, next: false, flash: "Sent to the host's queue")
    }

    /// "Play Next". Guests ask the host to play the songs after the current
    /// one (a host without Play Next appends them); a host inserts them after
    /// earlier Play Next songs. Returns `false` when the caller should queue
    /// locally.
    func userPlayNext(_ tracks: [Track]) -> Bool {
        if let host {
            // The host's own Play Next joins the same run as the guests', so
            // neither jumps ahead of the other.
            for t in tracks {
                hostTick()
                player.insertUpNext(t, at: Int(host.playNextOffset(trackIds: [t.id])))
            }
            hostTick()
            Chrome.shared.flash("Playing next")
            return true
        }
        let supported = guest?.hostSupportsPlayNext() ?? false
        return send(tracks, next: supported, flash: supported ? "Sent to play next on the host" : "The host's app is too old for Play Next, added to the end of the queue")
    }

    private func send(_ tracks: [Track], next: Bool, flash: String) -> Bool {
        guard guest != nil else { return false }
        guard guestControl else { refuse(); return true }
        let ids = tracks.map(\.id)
        outbox += stride(from: 0, to: ids.count, by: Self.maxEnqueue).map { (ids: Array(ids[$0..<min($0 + Self.maxEnqueue, ids.count)]), next: next) }
        pumpOutbox()
        Chrome.shared.flash(flash)
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
                let accepted = chunk.next ? guest.sendPlayNext(trackIds: chunk.ids) : guest.sendCommand(command: .enqueue(trackIds: chunk.ids))
                if accepted {
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
        let pos = player.liveHostPosition
        let now = Date()
        if !force, let s = sent, s.item == item, s.playing == playing {
            let expected = s.pos + (playing ? now.timeIntervalSince(s.at) : 0)
            if abs(expected - pos) < 0.25 { return }
        }
        host.setPlayback(itemId: item, trackId: player.current?.id, positionMs: Int64(pos * 1000), playing: playing)
        sent = (item, playing, pos, now)
    }

    // MARK: guest

    /// Only the manual offset: AVPlayer's position already is what is heard
    /// (it compensates Bluetooth and AirPlay itself), as desktop's is, so
    /// adding the system output latency put this guest ahead of the host.
    private var latencyMs: Int64 { Int64(extraLatencyMs) }

    private static func ms(_ v: Int64?) -> String { v.map(String.init) ?? "none" }

    @ObservationIgnored private var lastDiagAt = Date.distantPast
    @ObservationIgnored private var lastPath: LivePath?
    @ObservationIgnored private var lastDriftLogAt = Date.distantPast
    /// Latency the playing position was last aligned with.
    @ObservationIgnored private var alignedLatencyMs: Int64?

    /// Logs the network path and clock state when the path changes and every
    /// 10 s, so a slow same-network session can be told apart from a relay.
    private func logDiagnostics(_ guest: LiveGuest) {
        let now = Date()
        let d = guest.diagnostics()
        guard d.path != lastPath || now.timeIntervalSince(lastDiagAt) >= 10 else { return }
        lastPath = d.path; lastDiagAt = now
        Self.log.notice("path=\(String(describing: d.path), privacy: .public) pathRttMs=\(Self.ms(d.pathRttMs), privacy: .public) clockOffsetMs=\(Self.ms(d.clockOffsetMs), privacy: .public) clockRttMs=\(Self.ms(d.clockRttMs), privacy: .public) latencyMs=\(self.latencyMs)")
    }

    private func guestTick() {
        guard let guest, mode == .joined else { return }
        logDiagnostics(guest)
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
            if t.playing {
                player.liveResume()
            } else {
                player.toggle()
                player.seek(to: Double(t.positionMs) / 1000)
            }
            return
        }
        guard t.playing else {
            if abs(player.livePosition * 1000 - Double(t.positionMs)) > 1000 {
                player.seek(to: Double(t.positionMs) / 1000)
            }
            return
        }
        // Measure only while audio really plays with a healthy buffer, and
        // not right after a start or a seek: a player that is still filling
        // reports a frozen position, i.e. a large fake error.
        guard player.liveAdvancing, player.liveBufferHealthy else { return }
        let now = Date()
        if let s = player.liveSeekEndedAt, now.timeIntervalSince(s) < Self.seekSettle { return }
        let actual = Int64(player.livePosition * 1000)
        // A change of the manual delay stays below core's tolerance: realign
        // once instead of never applying it.
        let lat = latencyMs
        let previous = alignedLatencyMs
        if previous == nil { alignedLatencyMs = lat }
        if let previous, abs(lat - previous) >= Self.latencyRealignMs, let t = guest.target(latencyMs: lat) {
            alignedLatencyMs = lat
            Self.log.notice("latency changed \(previous)->\(lat) ms, realigning")
            player.liveSeek(to: Double(t.positionMs) / 1000)
            return
        }
        if now.timeIntervalSince(lastDriftLogAt) >= 10 {
            lastDriftLogAt = now
            Self.log.notice("drift errMs=\(t.positionMs - actual)")
        }
        // Core decides: it ignores errors up to a few seconds, spaces its own
        // corrections, and follows host seeks at once. The playback speed is
        // never changed (a `rate` answer is ignored).
        guard case .seek(let ms) = guest.correction(latencyMs: latencyMs, actualMs: actual) else { return }
        Self.log.notice("drift seek errMs=\(ms - actual)")
        alignedLatencyMs = lat
        player.liveSeek(to: Double(ms) / 1000)
    }

    /// The host's position for the track loaded here (latency included).
    private func hostPosition() -> (secs: Double, playing: Bool)? {
        let lat = latencyMs
        guard let guest, let t = guest.target(latencyMs: lat),
              let id = t.trackId, id == player.current?.id else { return nil }
        // The player aligns to this answer (start, rebuffer): it is the
        // latency the position now matches.
        alignedLatencyMs = lat
        return (Double(t.positionMs) / 1000, t.playing)
    }

    private static let log = Logger(subsystem: "dev.sdat", category: "live")
    /// Quiet time after a start or a drift seek before measuring again.
    private static let seekSettle: TimeInterval = 3
    /// Latency change that triggers one realigning seek (smaller ones are jitter).
    private static let latencyRealignMs: Int64 = 300

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
final class LiveControlBridge: LiveControlObserver, LivePlayNextObserver, @unchecked Sendable {
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
    func onGuestPlayNext(peer: String, name: String, trackIds: [String]) {
        let gen = generation
        Task { @MainActor in Live.shared.guestPlayNext(trackIds, generation: gen) }
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
