import os
import Observation
import UIKit

/// Continue playback on another device: this install is a player of the user's
/// TIDAL account (core `Handoff`). Attached while signed in and in the
/// foreground or playing; reports the local playback and follows the others.
@MainActor @Observable
final class HandoffHub {
    static let shared = HandoffHub()

    /// What another device plays or played last (the banner).
    private(set) var elsewhere: HandoffElsewhere?
    /// "Send to" targets.
    private(set) var devices: [HandoffDevice] = []
    /// A "Continue here" is running (the banner hides meanwhile).
    private(set) var continuing = false
    var foreground = UIApplication.shared.applicationState != .background {
        didSet { update() }
    }

    @ObservationIgnored private var handoff: Handoff?
    @ObservationIgnored private var starting = false
    @ObservationIgnored private var wanted = false
    @ObservationIgnored private var signingOut = false
    @ObservationIgnored private var retryAt: Date?
    /// Bumped on every start and stop; a late start result or event from an older one is dropped.
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var reported: (local: HandoffLocal, at: Date)?
    /// An Upcoming that came while `continuing`: applied once that result is played.
    @ObservationIgnored private var pendingUpcoming: (currentId: String, tracks: [Track])?
    private let player = Player.shared

    var showsBanner: Bool { elsewhere != nil && !continuing && !Live.shared.isGuest }

    private init() {
        NotificationCenter.default.addObserver(forName: UIApplication.willTerminateNotification, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated { HandoffHub.shared.stop() }
        }
        Task { [weak self] in
            while let self {
                self.update()
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    /// Detaches before the credentials go; nothing restarts until signed in again.
    func signOut() {
        signingOut = true
        stop()
    }

    /// A sign-in ends the sign-out, even one that came before `update()` saw the signed-out state.
    func signedIn() {
        signingOut = false
    }

    // MARK: lifecycle

    private func update() {
        let session = Session.shared
        if !session.signedIn { signingOut = false }
        let want = session.signedIn && !signingOut && (foreground || player.isPlaying)
        // Coming to the foreground or signing in retries a failed start at once.
        if want != wanted { wanted = want; retryAt = nil }
        // The last report (a pause, say) goes out before the detach.
        guard want else { if handoff != nil || starting { report(); stop() }; return }
        if handoff != nil { report(); return }
        guard !starting, retryAt.map({ Date() >= $0 }) ?? true else { return }
        starting = true
        generation += 1
        let gen = generation
        Task {
            do {
                let h = try await Handoff.start(installationKey: Preferences.shared.handoffKey,
                                                deviceName: UIDevice.current.name,
                                                observer: HandoffBridge(generation: gen))
                guard gen == generation else { h.stop(); return }
                starting = false
                handoff = h
                reported = nil
                report()
            } catch {
                guard gen == generation else { return }
                starting = false
                retryAt = Date().addingTimeInterval(Self.retryAfter)
                Self.log.notice("start failed: \(String(describing: error), privacy: .public)")
            }
        }
    }

    func stop() {
        generation += 1
        starting = false
        // An account switch must not wait for the old account's retry.
        retryAt = nil
        handoff?.stop()
        handoff = nil
        reported = nil
        elsewhere = nil
        devices = []
        continuing = false
        pendingUpcoming = nil
    }

    // MARK: reporting

    private var local: HandoffLocal {
        let r: HandoffRepeat = switch player.repeatMode { case .off: .off; case .all: .all; case .one: .one }
        // A queue restored at launch has no player item yet: its position is the saved one.
        let secs = player.liveHasItem ? player.liveHostPosition : player.position
        return HandoffLocal(current: player.current?.id,
                            future: player.userQueue.map(\.id) + player.upNext.map(\.id),
                            past: player.played.map(\.id), discover: player.endless,
                            positionMs: Int64(max(secs, 0) * 1000),
                            playing: player.isPlaying, repeat: r)
    }

    /// Reports when something changed since the last report; the position only
    /// when it left the course the last one implies (a seek).
    private func report(force: Bool = false) {
        guard let handoff, !Live.shared.isGuest else { return }
        let l = local
        let now = Date()
        if !force, let last = reported {
            var expected = last.local
            if expected.playing { expected.positionMs += Int64(now.timeIntervalSince(last.at) * 1000) }
            let moved = abs(expected.positionMs - l.positionMs) > Self.seekMs
            expected.positionMs = l.positionMs
            if expected == l, !moved { return }
        }
        handoff.report(local: l)
        reported = (l, now)
    }

    // MARK: actions

    func continueHere() {
        guard let h = handoff, !continuing, !Live.shared.isGuest else { return }
        continuing = true
        let gen = generation
        let started = (player.starts, player.plays)
        Task {
            do {
                let r = try await h.continueHere()
                // Joined Listen Together meanwhile (the host drives playback now), or
                // the user started something else: that wins.
                if gen == generation, !Live.shared.isGuest, (player.starts, player.plays) == started { play(r) }
            } catch {
                if gen == generation { Chrome.shared.flash("Couldn't continue here") }
            }
            guard gen == generation else { return }
            continuing = false
            // Also when this result was not played: it may be for a track that arrived meanwhile.
            if let u = pendingUpcoming, !Live.shared.isGuest { appendUpcoming(u.currentId, u.tracks) }
            pendingUpcoming = nil
        }
    }

    func send(to device: HandoffDevice) {
        guard let h = handoff else { return }
        report(force: true)
        let sent = (player.starts, player.plays)
        let gen = generation
        Task {
            do {
                try await h.sendTo(deviceId: device.id)
                // Signed out or into another account meanwhile: nothing of this send applies.
                guard gen == generation else { return }
                // Not what the user started while the send ran.
                if (player.starts, player.plays) == sent, !Live.shared.isGuest { player.pause() }
                Chrome.shared.flash("Playing on \(Self.name(device))")
            } catch {
                if gen == generation { Chrome.shared.flash("Couldn't send to \(Self.name(device))") }
            }
        }
    }

    private func play(_ r: HandoffResume) {
        let rep: Player.Repeat = switch r.repeat { case .off: .off; case .all: .all; case .one: .one }
        player.resume([r.current], at: Double(r.positionMs) / 1000, repeat: rep, from: r.from.name, discover: r.discover)
    }

    /// The report after the append is what the core waits for.
    private func appendUpcoming(_ currentId: String, _ tracks: [Track]) {
        if player.appendUpNext(after: currentId, tracks) { report() }
    }

    // MARK: events

    /// Events can arrive before `Handoff.start` returns; the core sends each
    /// state once, so they are kept rather than waiting for `handoff`.
    fileprivate func handle(_ e: HandoffEvent, generation gen: Int) {
        guard gen == generation else { return }
        switch e {
        case .devices(let d): devices = d
        case .elsewhere(let other): elsewhere = other
        case .arrived(let r):
            // A Listen Together guest follows its host.
            guard !Live.shared.isGuest else { return }
            play(r)
            Chrome.shared.flash("Continued from \(Self.name(r.from))")
        case .movedTo(let d):
            guard !Live.shared.isGuest else { return }
            player.pause()
            Chrome.shared.flash("Playing on \(Self.name(d))")
        case .upcoming(let currentId, let tracks):
            guard !Live.shared.isGuest else { return }
            if continuing { pendingUpcoming = (currentId, tracks) } else { appendUpcoming(currentId, tracks) }
        }
    }

    static func name(_ d: HandoffDevice) -> String { d.name.isEmpty ? "another device" : d.name }

    private static let retryAfter: TimeInterval = 60
    /// Below the core's own seek threshold, so every seek it would write is reported.
    private static let seekMs: Int64 = 1000
    private static let log = Logger(subsystem: "dev.sdat", category: "handoff")
}

/// Events arrive on a core thread; hop to the main actor.
final class HandoffBridge: HandoffObserver, @unchecked Sendable {
    private let generation: Int
    init(generation: Int) { self.generation = generation }
    func onEvent(event: HandoffEvent) {
        let gen = generation
        Task { @MainActor in HandoffHub.shared.handle(event, generation: gen) }
    }
}
