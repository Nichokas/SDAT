import Foundation

@MainActor
protocol LivePlaybackItem: AnyObject {
    var liveDuration: Double { get }
    var liveFailed: Bool { get }
    var liveBufferHealthy: Bool { get }
    func liveBufferedAhead(from secs: Double) -> Double
}

@MainActor
protocol LivePlaybackPort: AnyObject {
    var liveItem: (any LivePlaybackItem)? { get }
    var isAdvancing: Bool { get }
    var isPlaying: Bool { get }
    var loading: Bool { get }
    var needsLiveLoad: Bool { get }
    var enginePosition: Double { get }
    var position: Double { get }

    func prepareLiveJoin()
    func loadLive(_ tracks: [Track], index: Int, at secs: Double, playing: Bool)
    func seekExactly(_ secs: Double) async -> Bool
    func pauseEngine()
    func playEngine()
    func toggle()
    func prerollIfReady() async
    func activateAudio()
    func scheduleLiveStart(after seconds: Double) -> () -> Bool
    func finishLiveStart(started: Bool, sameItem: Bool)
    func prepareLiveResume()
    func reportPlaying(_ playing: Bool)
    func setLivePosition(_ secs: Double, publish: Bool)
}

@MainActor
final class LiveEngine {
    private unowned let port: any LivePlaybackPort

    init(port: any LivePlaybackPort) { self.port = port }

    var liveFollowing = false
    private(set) var systemPaused = false
    var loadResumeAt: Double?
    var livePosition: Double {
        let t = port.enginePosition
        return t.isFinite ? t : port.position
    }
    var liveHasItem: Bool { port.liveItem != nil }
    var liveHostPlaying: Bool { port.isPlaying && !port.loading }
    var liveHostPosition: Double { port.loading ? loadResumeAt ?? livePosition : livePosition }

    func clearSystemPause() { systemPaused = false }
    func noteSystemPause() { systemPaused = true }
    func prepareLiveJoin() { port.prepareLiveJoin() }

    func shouldAlign(autoplay: Bool) -> Bool { liveFollowing && autoplay && liveTarget != nil }

    func livePlay(_ tracks: [Track], index i: Int, at secs: Double, playing: Bool) {
        guard tracks.indices.contains(i) else { return }
        cancelLiveSeek()
        liveSeekEndedAt = nil
        port.loadLive(tracks, index: i, at: max(secs, 0), playing: playing)
    }

    /// Where the Listen Together host is now (seconds, output latency
    /// included) for the track loaded here, and whether it plays. `nil` when
    /// unknown or when the host is on another track. Set by `Live`.
    var liveTarget: (() -> (secs: Double, playing: Bool)?)?

    /// A drift seek is still running (or the item is rebuffering after one):
    /// the position is not trustworthy, so no new correction may start.
    private(set) var liveSeeking = false
    private var liveSeekTask: Task<Void, Never>?
    /// `liveSeekTask` is a `liveResume` that has not started playing yet
    /// (`isPlaying` is already true).
    private var liveResumePending = false

    func cancelLiveSeek() {
        liveSeekTask?.cancel(); liveSeekTask = nil
        liveSeeking = false; liveResumePending = false
    }

    /// The Listen Together session ended: a resume still waiting for the host
    /// never happens, while a drift seek just lands and keeps playing.
    func liveEnded() {
        liveFollowing = false
        liveTarget = nil
        let resuming = liveResumePending
        cancelLiveSeek()
        guard resuming else { return }
        port.pauseEngine()
        port.reportPlaying(false)
    }

    /// True while audio is actually advancing (not buffering after a seek).
    var liveAdvancing: Bool { port.isAdvancing }

    /// AVPlayer expects to keep playing without stalling and has a few
    /// seconds buffered ahead: only then is the position worth measuring.
    var liveBufferHealthy: Bool {
        guard let item = port.liveItem else { return false }
        return item.liveBufferHealthy
    }

    /// When the last start or drift seek finished (Live waits before measuring again).
    private(set) var liveSeekEndedAt: Date?

    func alignmentFinished() { liveSeekEndedAt = Date() }

    /// Large drift correction (only above a few seconds of error). A plain
    /// seek to where the host is, while playing: AVPlayer waits for its own
    /// buffer there. Parking ahead like `liveAlign` would mean seconds of
    /// silence; this costs only the refill. Playback speed is never changed.
    /// The destination is re-read from the host when the seek starts, and a
    /// slow seek that still lands more than `liveLateTolerance` behind is
    /// followed by one more seek that aims ahead by the time the first took
    /// (capped), so it does not leave an error that triggers another seek.
    func liveSeek(to secs: Double) {
        guard !liveSeeking, let item = port.liveItem else { return }
        liveSeeking = true
        liveSeekTask = Task {
            var lead = 0.0
            for _ in 0..<2 {
                let dest = max((liveTarget?()?.secs ?? secs) + lead, 0)
                let started = Date()
                port.setLivePosition(dest, publish: false)
                _ = await port.seekExactly(dest)
                guard !Task.isCancelled, port.liveItem === item else { break }
                let took = Date().timeIntervalSince(started)
                guard let host = liveTarget?(), host.playing,
                      host.secs - livePosition > Self.liveLateTolerance else { break }
                lead = min(took, Self.liveMaxSeekLead)
            }
            guard !Task.isCancelled else { return }
            liveSeeking = false
            liveSeekEndedAt = Date()
            if port.liveItem === item {
                port.setLivePosition(livePosition, publish: true)
            }
        }
    }

    /// Upper bound for aiming ahead of a slow drift seek.
    private static let liveMaxSeekLead: Double = 3

    /// How far ahead of the host the item is parked while it buffers.
    private static let liveJoinLead: Double = 8
    /// Buffer required ahead of the start point before playing.
    private static let liveMinBuffer: Double = 6
    /// Give up waiting for the buffer after this long (AVPlayer keeps filling).
    private static let liveBufferTimeout: Double = 15
    /// Starting this much later than the parked point is still fine (core
    /// tolerates a few seconds); later than that, park again further ahead.
    private static let liveLateTolerance: Double = 1
    private static let liveAlignAttempts = 3
    /// How long before the host reaches the parked point the start is
    /// scheduled, so a main actor that wakes up late doesn't delay it.
    private static let liveScheduleAhead: Double = 0.5
    /// Largest guest-host gap a resume is synchronized over (see `liveResume`).
    private static let liveResumeWindow: Double = 3

    /// Joins the host's playback without audible corrections: parks the item
    /// paused a little ahead of the host, waits until AVPlayer has a healthy
    /// buffer there, then starts when the host reaches that point. Slower to
    /// join, but nothing has to be corrected right after starting. Returns
    /// whether it ended up playing (`false`: host paused, track changed or
    /// cancelled; the item is then left paused at the host position).
    func liveAlign(_ item: any LivePlaybackItem) async -> Bool {
        func stillOurs() -> Bool { !Task.isCancelled && port.liveItem === item }
        attempt: for _ in 0..<Self.liveAlignAttempts {
            port.pauseEngine()
            guard stillOurs(), let t = liveTarget?() else { return false }
            guard t.playing else {
                _ = await port.seekExactly(max(t.secs, 0))
                return false
            }
            var startAt = t.secs + Self.liveJoinLead
            let dur = item.liveDuration
            if dur.isFinite, dur > 0 { startAt = min(startAt, max(dur - 1, t.secs)) }
            guard await port.seekExactly(max(startAt, 0)), stillOurs() else {
                if !stillOurs() { return false }
                continue
            }
            await waitForBuffer(item, from: startAt)
            if stillOurs() { await port.prerollIfReady() }
            // Wait until the host is about to reach the parked point. If the
            // host jumped back (seek, restart), the wait would be longer than
            // the lead: park again near its new position instead of staying silent.
            let hostDeadline = Date().addingTimeInterval(Self.liveJoinLead + Self.liveBufferTimeout)
            while stillOurs(), let now = liveTarget?(), now.playing, now.secs < startAt - Self.liveScheduleAhead {
                // The host stopped advancing (e.g. it is buffering): re-park.
                if Date() > hostDeadline { continue attempt }
                if startAt - now.secs > Self.liveJoinLead + Self.liveLateTolerance { continue attempt }
                try? await Task.sleep(for: .milliseconds(50))
            }
            guard stillOurs(), let now = liveTarget?() else { return false }
            guard now.playing else {
                _ = await port.seekExactly(max(now.secs, 0))
                return false
            }
            if now.secs - startAt > Self.liveLateTolerance { continue } // buffering took too long
            if systemPaused { return false } // call / headphones unplugged meanwhile
            port.activateAudio()
            return await startInSync()
        }
        // Very slow network: start where the host is and let AVPlayer buffer.
        guard stillOurs(), let t = liveTarget?() else { return false }
        _ = await port.seekExactly(max(t.secs, 0))
        // Re-read: the host may have paused or changed track during the seek.
        guard stillOurs(), let now = liveTarget?(), now.playing, !systemPaused else { return false }
        port.activateAudio()
        port.playEngine()
        return true
    }

    /// Starts the paused item in step with the host: its timebase is tied to
    /// the system clock so the parked position plays exactly when the host is
    /// there (a host already past it is caught up by skipping ahead). A plain
    /// `play()` starts after a variable AVPlayer delay that nothing measures,
    /// and core leaves errors that small alone, so the guest stayed behind for
    /// the whole song. Returns `false` when the host is not playing, or when
    /// it paused, the system paused us or the start was cancelled before it,
    /// leaving the player paused.
    private func startInSync() async -> Bool {
        guard let item = port.liveItem, let host = liveTarget?(), host.playing else { return false }
        let isDue = port.scheduleLiveStart(after: livePosition - host.secs)
        // Polled, not slept: callers block `Live.guestTick` meanwhile, so a
        // host or system pause, or a cancelled start, has to revoke the
        // scheduled rate here, also one that arrived during the last sleep
        // (`play()` below would undo it).
        var started = true
        while true {
            guard !Task.isCancelled, port.liveItem === item, !systemPaused,
                  let now = liveTarget?(), now.playing else { started = false; break }
            if isDue() { break }
            try? await Task.sleep(for: .milliseconds(50))
        }
        port.finishLiveStart(started: started, sameItem: port.liveItem === item)
        return started
    }

    /// The host resumed after a pause: resume in step with it (see
    /// `startInSync`). Too far from the host (more than the buffer kept while
    /// paused can cover) it joins like a fresh start (`liveAlign`) instead:
    /// playing from the old spot would be the wrong part of the song until
    /// drift correction caught it, and a plain seek before resuming could
    /// land behind again after a slow seek, over and over.
    func liveResume() {
        guard !port.isPlaying else { return }
        guard !port.needsLiveLoad, let item = port.liveItem, let host = liveTarget?(), host.playing else { port.toggle(); return }
        let far = abs(livePosition - host.secs) > Self.liveResumeWindow
        systemPaused = false
        port.prepareLiveResume()
        liveSeeking = true
        liveResumePending = true
        liveSeekTask = Task {
            let started = await far ? liveAlign(item) : startInSync()
            guard !Task.isCancelled else { return }
            liveSeeking = false
            liveResumePending = false
            liveSeekEndedAt = Date()
            if !started, port.liveItem === item {
                port.reportPlaying(false)
            }
        }
    }

    /// Waits (up to `liveBufferTimeout`) until AVPlayer has `liveMinBuffer`
    /// seconds loaded after `secs` (or up to the end of the track) and
    /// expects to play through.
    private func waitForBuffer(_ item: any LivePlaybackItem, from secs: Double) async {
        let deadline = Date().addingTimeInterval(Self.liveBufferTimeout)
        while !Task.isCancelled, port.liveItem === item, !item.liveFailed, Date() < deadline {
            let dur = item.liveDuration
            let need = dur.isFinite && dur > 0 ? min(Self.liveMinBuffer, max(dur - secs - 0.5, 0)) : Self.liveMinBuffer
            if item.liveBufferedAhead(from: secs) >= need, item.liveBufferHealthy { return }
            try? await Task.sleep(for: .milliseconds(100))
        }
    }

}
