import AVFoundation
import Combine
import MediaPlayer
import Observation
import UIKit

/// Queue + AVPlayer (HLS from TIDAL trackManifests), lock-screen controls and scrobbling.
@MainActor @Observable
final class Player {
    static let shared = Player()

    enum Repeat { case off, all, one }

    /// Base queue (album, playlist, mix...) and position in it.
    private(set) var queue: [Track] = [] { didSet { queueVersion &+= 1 } }
    private(set) var index: Int?
    /// "Play next" / "Add to queue" tracks; they play before the rest of the base queue.
    private(set) var userQueue: [Track] = [] { didSet { queueVersion &+= 1 } }
    /// Track currently playing (may come from the user queue).
    private(set) var current: Track?
    private(set) var isPlaying = false {
        didSet { if isPlaying != oldValue { scheduleSessionRelease() } }
    }
    /// Releases the audio session after a pause so iOS can suspend the app.
    @ObservationIgnored private var releaseTask: Task<Void, Never>?
    /// How long a paused player keeps the audio session (and the background audio grant).
    private static let idleRelease: Duration = .seconds(60)
    /// Fetching the current track's stream.
    private(set) var loading = false
    private(set) var position: Double = 0
    private(set) var duration: Double = 0
    private(set) var stream: Stream? {
        didSet { if let q = stream?.actualQuality, q != shownQuality { shownQuality = q } }
    }
    /// True when the current item plays the downloaded file, not the online stream.
    var isOffline: Bool { stream.flatMap { URL(string: $0.url) }?.isFileURL ?? false }
    /// Badge quality: keeps the previous value until the new stream's real quality is known.
    private(set) var shownQuality: Quality?
    private(set) var sourceTitle: String?
    private(set) var shuffle = false
    var repeatMode: Repeat = .off
    var error: String?

    var progress: Double { duration > 0 ? min(position / duration, 1) : 0 }
    /// Base-queue tracks after the current one.
    var upNext: ArraySlice<Track> { index.map { queue.dropFirst($0 + 1) } ?? queue[...] }
    /// Base queue in its unshuffled order, restored when shuffle is turned off.
    private var original: [Track] = [] { didSet { queueVersion &+= 1 } }

    private let avPlayer = AVQueuePlayer()
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
    private var systemObservers: [NSObjectProtocol] = []
    /// Set when an audio-session interruption paused active playback.
    private var resumeAfterInterruption = false
    /// Paused by the system (interruption, headphones unplugged) rather than
    /// by the user; Listen Together must not resume on its own then.
    private(set) var systemPaused = false
    /// Joining a session is a deliberate choice to listen: forget an older
    /// system pause so it doesn't hold the host's playback.
    func clearSystemPause() { systemPaused = false }
    private var infinite: Shuffle?
    private var loadTask: Task<Void, Never>?
    private var preloadTask: Task<Void, Never>?
    private var preloaded: (trackId: String, item: AVPlayerItem, stream: Stream)?
    private var startedAt: Date?
    private var listened: Double = 0
    private var scrobbled = false
    private var artwork: MPMediaItemArtwork?

    private init() {
        avPlayer.actionAtItemEnd = .advance
        configureSession()
        configureRemote()
        Task { await restore() }
        timeObserver = avPlayer.addPeriodicTimeObserver(
            forInterval: CMTime(seconds: 0.25, preferredTimescale: 600), queue: .main
        ) { [weak self] time in
            MainActor.assumeIsolated { self?.tick(time.seconds) }
        }
        endObserver = NotificationCenter.default.addObserver(
            forName: AVPlayerItem.didPlayToEndTimeNotification, object: nil, queue: .main
        ) { [weak self] note in
            let item = note.object as? AVPlayerItem
            MainActor.assumeIsolated { self?.trackEnded(item) }
        }
        observeSystemAudio()
    }

    // MARK: queue

    func play(_ tracks: [Track], startAt: Int = 0, from source: String? = nil) {
        recentIDs = []; recentOrder = []
        restoredInfinite = false
        guard tracks.indices.contains(startAt) else { return }
        infinite = nil
        original = tracks
        var list = tracks
        var start = startAt
        if shuffle {
            let first = list.remove(at: startAt)
            list.shuffle()
            list.insert(first, at: 0)
            start = 0
        }
        queue = list
        sourceTitle = source
        load(start)
    }

    /// Plays the list in a random order (Shuffle buttons).
    func playShuffled(_ tracks: [Track], from source: String? = nil) {
        guard !tracks.isEmpty else { return }
        shuffle = true
        play(tracks, startAt: Int.random(in: tracks.indices), from: source)
    }

    /// Discover's shuffle: endless queue mixing mixes, radios and discoveries.
    func playInfinite(mixes: [Mix]) {
        recentIDs = []; recentOrder = []
        restoredInfinite = false
        let source = Shuffle(mixes: mixes)
        infinite = source
        sourceTitle = "Discover"
        Task {
            do {
                let batch = try await source.nextBatch(min: 10)
                guard infinite === source, !batch.isEmpty else { return }
                queue = batch
                original = batch
                load(0)
            } catch { self.error = friendlyMessage(error) }
        }
    }

    /// Tapping one Discover cover: that song first, then the rest of the visible Discover
    /// pool (from the tapped one onwards, wrapping), then the endless Discover shuffle.
    func playDiscover(_ track: Track, pool: [Track], mixes: [Mix]) {
        recentIDs = []; recentOrder = []
        restoredInfinite = false
        var seen: Set<String> = [String(describing: track.id)]
        let i = pool.firstIndex { $0.id == track.id } ?? 0
        let rest = (Array(pool.dropFirst(i + 1)) + Array(pool.prefix(i)))
            .filter { seen.insert(String(describing: $0.id)).inserted }
        let list = [track] + rest
        infinite = mixes.isEmpty ? nil : Shuffle(mixes: mixes)
        original = list
        queue = list
        sourceTitle = "Discover"
        load(0)
    }

    func setShuffle(_ on: Bool) {
        guard on != shuffle else { return }
        shuffle = on
        guard let i = index, infinite == nil else { return }
        let head = Array(queue[...i])
        if on {
            queue = head + queue.dropFirst(i + 1).shuffled()
        } else if let cur = queue[safe: i], let at = original.firstIndex(where: { $0.id == cur.id }) {
            queue = original
            index = at
            _ = head
        }
        invalidatePreload()
    }

    func playNext(_ track: Track) {
        defer { persist() }
        if current == nil { play([track]); return }
        userQueue.insert(track, at: 0)
        invalidatePreload()
    }

    /// Inserts `track` `offset` entries after the current one (clamped to the
    /// user queue, which plays before the rest).
    func insertUpNext(_ track: Track, at offset: Int) {
        defer { persist() }
        if current == nil { play([track]); return }
        userQueue.insert(track, at: min(max(offset, 0), userQueue.count))
        invalidatePreload()
    }

    func addToQueue(_ track: Track) {
        defer { persist() }
        if current == nil { play([track]); return }
        userQueue.append(track)
        invalidatePreload()
    }

    func removeUserQueue(at offsets: IndexSet) {
        defer { persist() }
        userQueue.remove(atOffsets: offsets)
        invalidatePreload()
    }

    func moveUserQueue(from: IndexSet, to: Int) {
        defer { persist() }
        userQueue.move(fromOffsets: from, toOffset: to)
        invalidatePreload()
    }

    func moveUpNext(from: IndexSet, to: Int) {
        defer { persist() }
        guard let i = index else { return }
        let offset = i + 1
        queue.move(fromOffsets: IndexSet(from.map { $0 + offset }), toOffset: to + offset)
        invalidatePreload()
    }

    func removeUpNext(at offsets: IndexSet) {
        defer { persist() }
        guard let i = index else { return }
        queue.remove(atOffsets: IndexSet(offsets.map { $0 + i + 1 }))
        invalidatePreload()
    }

    func jump(toUpNext offset: Int) {
        guard let i = index else { return }
        load(i + 1 + offset)
    }

    func jump(toUser offset: Int) {
        guard userQueue.indices.contains(offset) else { return }
        let t = userQueue[offset]
        userQueue.removeFirst(offset + 1)
        start(t)
    }

    // MARK: transport

    func toggle() {
        // Restored after a relaunch: nothing loaded yet; fetch a fresh stream URL and resume there.
        if needsStart, let t = current {
            needsStart = false
            start(t, resumeAt: position)
            return
        }
        resumeAfterInterruption = false
        systemPaused = false
        if isPlaying { avPlayer.pause() } else {
            try? AVAudioSession.sharedInstance().setActive(true)
            avPlayer.play()
        }
        isPlaying.toggle()
        updateNowPlaying()
    }

    func next() { advance(auto: false) }

    func previous() {
        guard let i = index else { return }
        if position > 3 || i == 0 { seek(to: 0) } else { load(i - 1) }
    }

    func seek(to secs: Double) {
        avPlayer.seek(to: CMTime(seconds: secs, preferredTimescale: 600))
        position = secs
        updateNowPlaying()
    }

    // MARK: Listen Together (guest side; the host just plays normally)

    /// Exact player position (the published `position` only updates on the periodic observer).
    var livePosition: Double {
        let t = avPlayer.currentTime().seconds
        return t.isFinite ? t : position
    }

    /// Position a Listen Together host publishes. While a track loads the
    /// engine still reports the previous item's time, which guests would
    /// take as the place to start the new track.
    var liveHostPosition: Double { loading ? loadResumeAt : livePosition }
    @ObservationIgnored private var loadResumeAt = 0.0

    /// Following a Listen Together host: the host decides what plays next, so the
    /// personal queue, repeat and preloading are suspended (nothing is consumed).
    var liveFollowing = false

    /// False once the guest's copy of a track has finished (the queue player drops the
    /// item), so the host's next anchor reloads it even when it is the same song.
    var liveHasItem: Bool { avPlayer.currentItem != nil }

    /// Plays the host's track at `secs`, replacing the local queue. `playing: false`
    /// loads it paused, so joining a paused session never blips audio.
    func livePlay(_ tracks: [Track], index i: Int, at secs: Double, playing: Bool) {
        guard tracks.indices.contains(i) else { return }
        liveSeekTask?.cancel(); liveSeekTask = nil; liveSeeking = false
        liveSeekEndedAt = nil
        liveAligning = false
        recentIDs = []; recentOrder = []
        infinite = nil
        original = tracks
        queue = tracks
        sourceTitle = "Listen Together"
        load(i, resumeAt: max(secs, 0), autoplay: playing)
    }

    /// Where the Listen Together host is now (seconds, output latency
    /// included) for the track loaded here, and whether it plays. `nil` when
    /// unknown or when the host is on another track. Set by `Live`.
    @ObservationIgnored var liveTarget: (() -> (secs: Double, playing: Bool)?)?

    /// A drift seek is still running (or the item is rebuffering after one):
    /// the position is not trustworthy, so no new correction may start.
    @ObservationIgnored private(set) var liveSeeking = false
    @ObservationIgnored private var liveSeekTask: Task<Void, Never>?

    /// True while audio is actually advancing (not buffering after a seek).
    var liveAdvancing: Bool { avPlayer.timeControlStatus == .playing }

    /// AVPlayer expects to keep playing without stalling and has a few
    /// seconds buffered ahead: only then is the position worth measuring.
    var liveBufferHealthy: Bool {
        guard let item = avPlayer.currentItem else { return false }
        return item.isPlaybackLikelyToKeepUp || item.isPlaybackBufferFull
    }

    /// When the last start or drift seek finished (Live waits before measuring again).
    @ObservationIgnored private(set) var liveSeekEndedAt: Date?

    /// Large drift correction (only above a few seconds of error). A plain
    /// seek to where the host is, while playing: AVPlayer waits for its own
    /// buffer there. Parking ahead like `liveAlign` would mean seconds of
    /// silence; this costs only the refill. Playback speed is never changed.
    /// The destination is re-read from the host when the seek starts, and a
    /// slow seek that still lands more than `liveLateTolerance` behind is
    /// followed by one more seek that aims ahead by the time the first took
    /// (capped), so it does not leave an error that triggers another seek.
    func liveSeek(to secs: Double) {
        guard !liveSeeking, let item = avPlayer.currentItem else { return }
        liveSeeking = true
        liveSeekTask = Task {
            var lead = 0.0
            for _ in 0..<2 {
                let dest = max((liveTarget?()?.secs ?? secs) + lead, 0)
                let started = Date()
                position = dest
                _ = await avPlayer.seek(to: CMTime(seconds: dest, preferredTimescale: 600),
                                        toleranceBefore: .zero, toleranceAfter: .zero)
                guard !Task.isCancelled, avPlayer.currentItem === item else { break }
                let took = Date().timeIntervalSince(started)
                guard let host = liveTarget?(), host.playing,
                      host.secs - livePosition > Self.liveLateTolerance else { break }
                lead = min(took, Self.liveMaxSeekLead)
            }
            guard !Task.isCancelled else { return }
            liveSeeking = false
            liveSeekEndedAt = Date()
            if avPlayer.currentItem === item {
                position = livePosition
                updateNowPlaying()
            }
        }
    }

    /// Upper bound for aiming ahead of a slow drift seek.
    private static let liveMaxSeekLead: Double = 3
    /// `start` is aligning a live item: a downloaded-file fallback must align
    /// its replacement item too.
    @ObservationIgnored private var liveAligning = false

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
    private func liveAlign(_ item: AVPlayerItem) async -> Bool {
        func at(_ secs: Double) -> CMTime { CMTime(seconds: max(secs, 0), preferredTimescale: 600) }
        func stillOurs() -> Bool { !Task.isCancelled && avPlayer.currentItem === item }
        attempt: for _ in 0..<Self.liveAlignAttempts {
            avPlayer.pause()
            guard stillOurs(), let t = liveTarget?() else { return false }
            guard t.playing else {
                _ = await avPlayer.seek(to: at(t.secs), toleranceBefore: .zero, toleranceAfter: .zero)
                return false
            }
            var startAt = t.secs + Self.liveJoinLead
            let dur = item.duration.seconds
            if dur.isFinite, dur > 0 { startAt = min(startAt, max(dur - 1, t.secs)) }
            guard await avPlayer.seek(to: at(startAt), toleranceBefore: .zero, toleranceAfter: .zero), stillOurs() else {
                if !stillOurs() { return false }
                continue
            }
            await waitForBuffer(item, from: startAt)
            if stillOurs(), avPlayer.status == .readyToPlay { _ = await avPlayer.preroll(atRate: 1) }
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
                _ = await avPlayer.seek(to: at(now.secs), toleranceBefore: .zero, toleranceAfter: .zero)
                return false
            }
            if now.secs - startAt > Self.liveLateTolerance { continue } // buffering took too long
            if systemPaused { return false } // call / headphones unplugged meanwhile
            try? AVAudioSession.sharedInstance().setActive(true)
            return await startInSync()
        }
        // Very slow network: start where the host is and let AVPlayer buffer.
        guard stillOurs(), let t = liveTarget?() else { return false }
        _ = await avPlayer.seek(to: at(t.secs), toleranceBefore: .zero, toleranceAfter: .zero)
        // Re-read: the host may have paused or changed track during the seek.
        guard stillOurs(), let now = liveTarget?(), now.playing, !systemPaused else { return false }
        try? AVAudioSession.sharedInstance().setActive(true)
        avPlayer.play()
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
        guard let item = avPlayer.currentItem, let host = liveTarget?(), host.playing else { return false }
        let clock = CMClockGetHostTimeClock()
        let startAt = CMClockGetTime(clock) + CMTime(seconds: livePosition - host.secs, preferredTimescale: 1_000_000_000)
        // `setRate(_:time:atHostTime:)` requires it off; it goes back on once
        // the start is due.
        avPlayer.automaticallyWaitsToMinimizeStalling = false
        avPlayer.setRate(1, time: .invalid, atHostTime: startAt)
        // Polled, not slept: callers block `Live.guestTick` meanwhile, so a
        // host or system pause, or a cancelled start, has to revoke the
        // scheduled rate here.
        var started = true
        while (startAt - CMClockGetTime(clock)).seconds > 0 {
            guard !Task.isCancelled, avPlayer.currentItem === item, !systemPaused,
                  let now = liveTarget?(), now.playing else { started = false; break }
            try? await Task.sleep(for: .milliseconds(50))
        }
        // Another item (e.g. local playback after leaving) is not ours to pause.
        if !started, avPlayer.currentItem === item { avPlayer.pause() }
        avPlayer.automaticallyWaitsToMinimizeStalling = true
        // Without waiting, a stall drops the rate to 0 for good: hand it back
        // to AVPlayer, which resumes once it has buffered.
        if started, avPlayer.rate == 0 { avPlayer.play() }
        return started
    }

    /// The host resumed after a pause: resume in step with it (see
    /// `startInSync`). Too far from the host (more than the buffer kept while
    /// paused can cover) it first moves there, still paused, and the next
    /// tick resumes in step: playing from the old spot would be the wrong
    /// part of the song until drift correction caught it.
    func liveResume() {
        guard !isPlaying else { return }
        guard !needsStart, avPlayer.currentItem != nil, let host = liveTarget?(), host.playing else { toggle(); return }
        guard abs(livePosition - host.secs) <= Self.liveResumeWindow else { liveSeek(to: host.secs); return }
        resumeAfterInterruption = false
        systemPaused = false
        try? AVAudioSession.sharedInstance().setActive(true)
        isPlaying = true
        updateNowPlaying()
        liveSeeking = true
        let item = avPlayer.currentItem
        liveSeekTask = Task {
            let started = await startInSync()
            guard !Task.isCancelled else { return }
            liveSeeking = false
            liveSeekEndedAt = Date()
            if !started, avPlayer.currentItem === item {
                isPlaying = false
                updateNowPlaying()
            }
        }
    }

    /// Waits (up to `liveBufferTimeout`) until AVPlayer has `liveMinBuffer`
    /// seconds loaded after `secs` (or up to the end of the track) and
    /// expects to play through.
    private func waitForBuffer(_ item: AVPlayerItem, from secs: Double) async {
        let deadline = Date().addingTimeInterval(Self.liveBufferTimeout)
        while !Task.isCancelled, avPlayer.currentItem === item, item.status != .failed, Date() < deadline {
            let dur = item.duration.seconds
            let need = dur.isFinite && dur > 0 ? min(Self.liveMinBuffer, max(dur - secs - 0.5, 0)) : Self.liveMinBuffer
            if Self.bufferedAhead(item, from: secs) >= need, item.isPlaybackLikelyToKeepUp || item.isPlaybackBufferFull { return }
            try? await Task.sleep(for: .milliseconds(100))
        }
    }

    private static func bufferedAhead(_ item: AVPlayerItem, from secs: Double) -> Double {
        for v in item.loadedTimeRanges {
            let r = v.timeRangeValue
            let s = r.start.seconds, e = (r.start + r.duration).seconds
            if s.isFinite, e.isFinite, s <= secs + 0.5, e > secs { return e - secs }
        }
        return 0
    }

    func cycleRepeat() {
        repeatMode = switch repeatMode { case .off: .all; case .all: .one; case .one: .off }
        syncRemote()
        invalidatePreload()
    }

    /// Re-requests the current track's manifest after a quality change, keeping the position.
    func reloadQuality() {
        guard let t = current else { return }
        start(t, resumeAt: position)
    }

    // MARK: internals

    /// What plays after the current track: (track, comes from user queue, base index).
    /// `manual`: an explicit Next skips past a repeat-one track (it only loops on natural end).
    private func following(manual: Bool = false) -> (Track, Bool, Int?)? {
        if liveFollowing { return nil }
        if repeatMode == .one, !manual, let c = current { return (c, false, index) }
        if let u = userQueue.first { return (u, true, index) }
        guard let i = index else { return nil }
        if i + 1 < queue.count { return (queue[i + 1], false, i + 1) }
        if repeatMode == .all, !queue.isEmpty { return (queue[0], false, 0) }
        return nil
    }

    private func advance(auto: Bool) {
        guard let (t, fromUser, newIndex) = following(manual: !auto) else {
            // Endless (Discover) queue ran dry before the refill landed: fetch, then go on.
            if let source = infinite {
                loading = true
                Task {
                    let more = await fetchMore(from: source)
                    guard infinite === source else { return }
                    loading = false
                    if more { advance(auto: auto) } else { stopAtEnd() }
                }
                return
            }
            stopAtEnd()
            return
        }
        if fromUser { userQueue.removeFirst() }
        index = newIndex
        if auto, let p = preloaded, p.trackId == t.id, avPlayer.currentItem === p.item {
            preloaded = nil
            adopt(t, stream: p.stream)
            avPlayer.play()
            isPlaying = true
            afterStart(t)
        } else {
            start(t)
        }
    }

    /// End of a finite queue (repeat off): really stop, rewound and paused.
    private func stopAtEnd() {
        // End of queue (repeat off): really stop the engine and rewind to the start,
        // paused, like Apple Music. Previously only the flag flipped, so the UI showed
        // "paused" while the audio kept playing after a manual next.
        avPlayer.pause()
        isPlaying = false
        seek(to: 0)
    }

    private func load(_ i: Int, resumeAt: Double = 0, autoplay: Bool = true) {
        guard queue.indices.contains(i) else { return }
        index = i
        start(queue[i], resumeAt: resumeAt, autoplay: autoplay)
    }

    private func adopt(_ track: Track, stream s: Stream?) {
        current = track
        stream = s
        position = 0
        duration = Double(track.durationSecs)
        listened = 0
        scrobbled = false
        startedAt = Date()
        artwork = nil
    }

    private func start(_ track: Track, resumeAt: Double = 0, autoplay: Bool = true) {
        needsStart = false
        adopt(track, stream: nil)
        loading = true
        position = resumeAt
        loadResumeAt = resumeAt
        invalidatePreload()
        loadTask?.cancel()
        loadTask = Task {
            do {
                let s = try await Self.source(track.id)
                guard !Task.isCancelled, current?.id == track.id, let url = URL(string: s.url) else { return }
                stream = s
                avPlayer.removeAllItems()
                let item = Self.makeItem(url)
                avPlayer.insert(item, after: nil)
                if url.isFileURL { watchLocal(item, track: track) }
                if liveFollowing && autoplay && liveTarget != nil {
                    // Following a playing host: start only with a healthy buffer
                    // and on time, so nothing needs correcting after the start.
                    liveAligning = true
                    let playing = await liveAlign(item)
                    guard !Task.isCancelled, current?.id == track.id else { return }
                    liveAligning = false
                    // A failed download was replaced meanwhile: `watchLocal`
                    // aligns the replacement and finishes the start.
                    guard avPlayer.currentItem === item else { return }
                    isPlaying = playing
                    loading = false
                    liveSeekEndedAt = Date()
                    afterStart(track)
                    return
                }
                if resumeAt > 0 {
                    let tol: CMTime = liveFollowing ? .zero : .positiveInfinity
                    await avPlayer.seek(to: CMTime(seconds: resumeAt, preferredTimescale: 600),
                                        toleranceBefore: tol, toleranceAfter: tol)
                }
                try? AVAudioSession.sharedInstance().setActive(true)
                if autoplay {
                    avPlayer.play()
                    isPlaying = true
                } else {
                    avPlayer.pause()
                    isPlaying = false
                }
                loading = false
                afterStart(track)
            } catch {
                if !Task.isCancelled { loading = false }
                if !Task.isCancelled { self.error = friendlyMessage(error) }
            }
        }
    }

    /// A downloaded file that AVPlayer can't open falls back to the online stream.
    private func watchLocal(_ item: AVPlayerItem, track: Track) {
        Task { [weak self] in
            for await st in item.publisher(for: \.status).values {
                if st == .readyToPlay { return }
                if st == .failed { break }
            }
            guard let self, self.avPlayer.currentItem === item, self.current?.id == track.id,
                  let s = try? await SDAT.stream(trackId: track.id, quality: Preferences.shared.streamingQuality),
                  self.current?.id == track.id, let u = URL(string: s.url) else { return }
            let at = self.position
            self.stream = s
            self.avPlayer.removeAllItems()
            let replacement = Self.makeItem(u)
            self.avPlayer.insert(replacement, after: nil)
            if self.liveFollowing, self.liveAligning, self.liveTarget != nil {
                // Following a host: align the online item like a fresh start.
                let playing = await self.liveAlign(replacement)
                guard self.avPlayer.currentItem === replacement, self.current?.id == track.id else { return }
                self.liveAligning = false
                self.isPlaying = playing
                self.loading = false
                self.liveSeekEndedAt = Date()
                self.afterStart(track)
                return
            }
            if at > 0 { await self.avPlayer.seek(to: CMTime(seconds: at, preferredTimescale: 600)) }
            if self.isPlaying { self.avPlayer.play() }
        }
    }

    private func afterStart(_ track: Track) {
        updateNowPlaying()
        Task {
            await loadArtwork(track)
        }
        nowPlayingScrobble(track)
        refillIfNeeded()
        schedulePreload()
    }

    /// Every item gets a real forward buffer. The rate is always 1.0
    /// (Listen Together never changes the speed); the cheap time-domain pitch
    /// algorithm is kept so a stray rate change could never cost much CPU.
    private static func makeItem(_ url: URL) -> AVPlayerItem {
        let item = AVPlayerItem(url: url)
        item.preferredForwardBufferDuration = 10
        item.audioTimePitchAlgorithm = .timeDomain
        return item
    }

    /// Downloaded file when available (works offline), else a fresh HLS stream.
    private static func source(_ id: String) async throws -> Stream {
        if let s = Downloads.shared.localStream(id) { return s }
        return try await SDAT.stream(trackId: id, quality: Preferences.shared.streamingQuality)
    }

    /// Prepares the next track's AVPlayerItem behind the current one (gapless-ish start).
    private func schedulePreload() {
        preloadTask?.cancel()
        guard Preferences.shared.preloadNext, repeatMode != .one,
              let (next, _, _) = following(), let cur = current else { return }
        preloadTask = Task {
            guard let s = try? await Self.source(next.id),
                  !Task.isCancelled, current?.id == cur.id, let url = URL(string: s.url),
                  let curItem = avPlayer.currentItem else { return }
            let item = Self.makeItem(url)
            guard avPlayer.canInsert(item, after: curItem) else { return }
            avPlayer.insert(item, after: curItem)
            preloaded = (next.id, item, s)
        }
    }

    private func invalidatePreload() {
        preloadTask?.cancel()
        if let p = preloaded { avPlayer.remove(p.item) }
        preloaded = nil
        if current != nil, stream != nil { schedulePreload() }
    }

    private func tick(_ t: Double) {
        guard t.isFinite, !needsStart else { return }
        if isPlaying, Date().timeIntervalSince(lastSave) > 5 { persist() }
        if isPlaying, t > position, t - position < 2 { listened += t - position }
        position = t
        if let d = avPlayer.currentItem?.duration.seconds, d.isFinite, d > 0 { duration = d }
        if !scrobbled, let track = current, shouldScrobble(listenedSecs: UInt64(listened), durationSecs: track.durationSecs) {
            scrobbled = true
            submitScrobble(track)
        }
    }

    private func trackEnded(_ item: AVPlayerItem?) {
        // Only react to the item that was actually playing.
        guard item == nil || item === avPlayer.items().first || item !== preloaded?.item else { return }
        // Listen Together guest: wait for the host's next track instead of advancing.
        if liveFollowing { DispatchQueue.main.async { self.avPlayer.pause(); self.isPlaying = false }; return }
        if repeatMode == .one, let c = current { start(c); return }
        DispatchQueue.main.async { self.advance(auto: true) }
    }

    private func refillIfNeeded() {
        guard let source = infinite, let i = index, queue.count - i <= 5 else { return }
        Task { _ = await fetchMore(from: source) }
    }

    @ObservationIgnored private var refilling: Task<Bool, Never>?
    /// Appends a new endless batch, skipping tracks already in the queue. One fetch at a time;
    /// returns whether anything was added.
    private func fetchMore(from source: Shuffle) async -> Bool {
        if let r = refilling { return await r.value }
        let task = Task { @MainActor () -> Bool in
            defer { refilling = nil }
            for _ in 0..<3 {
                guard let batch = try? await source.nextBatch(min: 10), infinite === source else { return false }
                remember(queue.map(\.id))
                var ids = recentIDs
                let fresh = batch.filter { ids.insert($0.id).inserted }
                if !fresh.isEmpty {
                    let mirrored = original == queue
                    queue.append(contentsOf: fresh)
                    if mirrored { original = queue } else { original.append(contentsOf: fresh) }
                    remember(fresh.map(\.id))
                    trimPlayedHistory()
                    invalidatePreload(); schedulePreload()
                    return true
                }
            }
            return false
        }
        refilling = task
        return await task.value
    }

    // MARK: persistence

    /// Restored state not loaded into AVPlayer yet (first play fetches the stream).
    @ObservationIgnored private var needsStart = false
    @ObservationIgnored private var restoredInfinite = false
    @ObservationIgnored private var lastSave = Date.distantPast
    @ObservationIgnored private var restoring = true

    @ObservationIgnored private var queueVersion = 0
    @ObservationIgnored private var savedQueueVersion = -1
    @ObservationIgnored private var savedQueueOffset = 0
    @ObservationIgnored private var queueID = 0

    /// Cheap: the small state file every call; the queue file only when the queues changed.
    /// Both are bounded windows (see PlaybackStore caps), JSON-encoded off the main thread.
    func persist() {
        guard !restoring else { return }
        lastSave = Date()
        guard let c = current else { PlaybackStore.clear(); savedQueueVersion = -1; return }
        let e = DiscoverCache.encode
        let endless = infinite != nil || restoredInfinite
        if queueVersion != savedQueueVersion {
            let w = PlaybackStore.window(count: queue.count, index: index, endless: endless)
            // Original order: same window when it mirrors the queue (unshuffled / endless),
            // else capped prefix (only used to un-shuffle).
            let orig = original == queue ? Array(original[w]) : Array(original.prefix(PlaybackStore.maxSaved))
            queueID &+= 1
            PlaybackStore.saveQueue(.init(queue: e(Array(queue[w])), original: e(orig),
                                          userQueue: e(Array(userQueue.prefix(PlaybackStore.maxSaved))),
                                          infinite: endless, sourceTitle: sourceTitle, id: queueID))
            savedQueueVersion = queueVersion
            savedQueueOffset = w.lowerBound
        }
        let rm: String = switch repeatMode { case .off: "off"; case .all: "all"; case .one: "one" }
        PlaybackStore.saveState(.init(current: e([c])[0], index: index.map { $0 - savedQueueOffset },
                                      position: position, repeatMode: rm, shuffle: shuffle, queueID: queueID))
    }

    private func restore() async {
        defer { restoring = false }
        guard let (s, q) = await PlaybackStore.load(), current == nil,
              let cur = DiscoverCache.decode([s.current]).first else { return }
        let d = DiscoverCache.decode
        if let q {
            queue = d(q.queue); original = d(q.original); userQueue = d(q.userQueue)
            sourceTitle = q.sourceTitle
            restoredInfinite = q.infinite
        } else {
            queue = [cur]; original = [cur]
        }
        index = q == nil ? 0 : s.index.flatMap { queue.indices.contains($0) ? $0 : nil }
        recentIDs = Set(queue.map(\.id))
        repeatMode = switch s.repeatMode { case "all": .all; case "one": .one; default: .off }
        shuffle = s.shuffle
        queueID = s.queueID
        adopt(cur, stream: nil)
        position = max(0, min(s.position, duration > 0 ? duration - 1 : s.position))
        isPlaying = false
        needsStart = true
        restoring = false
        updateNowPlaying()
        Task { await loadArtwork(cur) }
    }

    /// Endless queues: ids already queued or played (survives history trimming) for dedupe.
    @ObservationIgnored private var recentIDs: Set<String> = []
    @ObservationIgnored private var recentOrder: [String] = []
    private static let maxPlayedHistory = 300, maxRecentIDs = 2000

    private func remember(_ ids: [String]) {
        for id in ids where recentIDs.insert(id).inserted { recentOrder.append(id) }
        if recentOrder.count > Self.maxRecentIDs {
            let drop = recentOrder.count - Self.maxRecentIDs
            for id in recentOrder.prefix(drop) { recentIDs.remove(id) }
            recentOrder.removeFirst(drop)
        }
    }

    /// Endless queues only: keep at most `maxPlayedHistory` played tracks in memory.
    private func trimPlayedHistory() {
        guard infinite != nil, let i = index, i > Self.maxPlayedHistory else { return }
        let drop = i - Self.maxPlayedHistory
        let mirrored = original == queue
        queue.removeFirst(drop)
        if mirrored { original = queue }
        index = i - drop
        invalidatePreload(); schedulePreload()
    }

    /// Re-attaches the endless Discover source to a restored Discover queue once mixes load.
    func resumeInfinite(mixes: [Mix]) {
        guard restoredInfinite, infinite == nil, !mixes.isEmpty else { return }
        restoredInfinite = false
        infinite = Shuffle(mixes: mixes)
        refillIfNeeded()
    }

    // MARK: system integration

    private func configureSession() {
        let s = AVAudioSession.sharedInstance()
        try? s.setCategory(.playback, mode: .default, policy: .longFormAudio)
        // Activated only when playback starts: an active session at launch keeps an idle app awake.
    }

    /// While paused (and not in Listen Together), deactivate the audio session after a grace
    /// period so iOS suspends the app. Play/resume reactivates it; Now Playing stays registered.
    private func scheduleSessionRelease() {
        releaseTask?.cancel(); releaseTask = nil
        guard !isPlaying else { return }
        releaseTask = Task { [weak self] in
            // Re-check until idle: a Listen Together session or a load in progress may end later.
            while !Task.isCancelled {
                try? await Task.sleep(for: Self.idleRelease)
                guard !Task.isCancelled, let self, !self.isPlaying else { return }
                if !self.loading, !Live.shared.active { break }
            }
            guard !Task.isCancelled else { return }
            try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
        }
    }

    /// Keeps `isPlaying` in step with the real audio: calls / Siri / other apps
    /// (interruptions), unplugged headphones (route change), and any rate change
    /// we didn't initiate.
    private func observeSystemAudio() {
        let nc = NotificationCenter.default
        let session = AVAudioSession.sharedInstance()
        systemObservers.append(nc.addObserver(forName: AVAudioSession.interruptionNotification,
                                              object: session, queue: .main) { [weak self] n in
            let raw = n.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt
            let opts = n.userInfo?[AVAudioSessionInterruptionOptionKey] as? UInt ?? 0
            MainActor.assumeIsolated {
                guard let self, let raw, let type = AVAudioSession.InterruptionType(rawValue: raw) else { return }
                switch type {
                case .began:
                    // Remember only real playback; a cold-restored paused state never auto-resumes.
                    if self.isPlaying { self.resumeAfterInterruption = true; self.systemPaused = true }
                    self.markPaused()
                case .ended:
                    let should = AVAudioSession.InterruptionOptions(rawValue: opts).contains(.shouldResume)
                    let resume = should && self.resumeAfterInterruption && !self.needsStart && self.current != nil
                    self.resumeAfterInterruption = false
                    if resume {
                        self.systemPaused = false
                        try? AVAudioSession.sharedInstance().setActive(true)
                        self.avPlayer.play()
                        self.isPlaying = true
                        self.updateNowPlaying()
                    }
                @unknown default: break
                }
            }
        })
        systemObservers.append(nc.addObserver(forName: AVAudioSession.routeChangeNotification,
                                              object: session, queue: .main) { [weak self] n in
            let raw = n.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt
            MainActor.assumeIsolated {
                guard let raw, AVAudioSession.RouteChangeReason(rawValue: raw) == .oldDeviceUnavailable,
                      let self, self.isPlaying else { return }
                self.resumeAfterInterruption = false
                self.systemPaused = true
                self.markPaused()
            }
        })
        systemObservers.append(nc.addObserver(forName: AVPlayer.rateDidChangeNotification,
                                              object: avPlayer, queue: .main) { [weak self] n in
            let reason = n.userInfo?[AVPlayer.rateDidChangeReasonKey] as? AVPlayer.RateDidChangeReason
            MainActor.assumeIsolated {
                guard let self, reason != .setRateCalled, !self.loading, !self.needsStart else { return }
                let audible = self.avPlayer.rate > 0
                if audible != self.isPlaying, self.avPlayer.currentItem != nil {
                    self.isPlaying = audible
                    self.updateNowPlaying()
                }
            }
        })
    }

    /// Paused by the system: reflect it in the UI / Now Playing without dropping the item.
    private func markPaused() {
        avPlayer.pause()
        guard isPlaying else { return }
        isPlaying = false
        updateNowPlaying()
    }

    private func configureRemote() {
        let c = MPRemoteCommandCenter.shared()
        c.playCommand.addTarget { _ in
            MainActor.assumeIsolated { Live.shared.userPlay() }
            return .success
        }
        c.pauseCommand.addTarget { _ in
            MainActor.assumeIsolated { Live.shared.userPause() }
            return .success
        }
        c.togglePlayPauseCommand.addTarget { _ in
            MainActor.assumeIsolated { Live.shared.userToggle() }
            return .success
        }
        c.nextTrackCommand.addTarget { _ in
            MainActor.assumeIsolated { Live.shared.userNext() }
            return .success
        }
        c.previousTrackCommand.addTarget { [weak self] _ in
            MainActor.assumeIsolated { if !Live.shared.guestRefuses() { self?.previous() } }
            return .success
        }
        // Lock screen / Control Center: favourite (like).
        c.likeCommand.localizedTitle = "Favourite"
        c.likeCommand.localizedShortTitle = "Favourite"
        c.likeCommand.addTarget { [weak self] _ in
            MainActor.assumeIsolated {
                guard let id = self?.current?.id else { return }
                Favs.shared.toggle(id)
            }
            return .success
        }
        Favs.shared.onChange = { [weak self] in MainActor.assumeIsolated { self?.syncRemote() } }
        c.changePlaybackPositionCommand.addTarget { [weak self] event in
            guard let e = event as? MPChangePlaybackPositionCommandEvent else { return .commandFailed }
            MainActor.assumeIsolated { if !Live.shared.guestRefuses() { self?.seek(to: e.positionTime) } }
            return .success
        }
    }

    /// Like state shown by the system controls.
    private func syncRemote() {
        let c = MPRemoteCommandCenter.shared()
        let fav = current.flatMap { Favs.shared.known($0.id) } ?? false
        c.likeCommand.isEnabled = current != nil
        c.likeCommand.isActive = fav
        c.likeCommand.localizedTitle = fav ? "Remove from Favourites" : "Add to Favourites"
    }

    private func updateNowPlaying() {
        syncRemote()
        persist()
        if let id = current?.id, Favs.shared.known(id) == nil {
            Task { await Favs.shared.check(id); syncRemote() }
        }
        guard let t = current else {
            MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
            return
        }
        var info: [String: Any] = [
            MPMediaItemPropertyTitle: t.displayTitle,
            MPMediaItemPropertyArtist: t.artistLine,
            MPMediaItemPropertyAlbumTitle: t.album?.title ?? "",
            MPMediaItemPropertyPlaybackDuration: duration,
            MPNowPlayingInfoPropertyElapsedPlaybackTime: position,
            MPNowPlayingInfoPropertyPlaybackRate: isPlaying ? 1.0 : 0.0,
        ]
        if let artwork { info[MPMediaItemPropertyArtwork] = artwork }
        MPNowPlayingInfoCenter.default().nowPlayingInfo = info
    }

    private func loadArtwork(_ track: Track) async {
        var data = Downloads.shared.localCover(track.id).flatMap { try? Data(contentsOf: $0) }
        if data == nil, let url = track.coverURLLarge { data = try? await URLSession.shared.data(from: url).0 }
        guard let data,
              let image = UIImage(data: data), current?.id == track.id else { return }
        artwork = MPMediaItemArtwork(boundsSize: image.size) { _ in image }
        updateNowPlaying()
    }

    // MARK: scrobbling

    private func scrobbleTrack(_ t: Track) -> ScrobbleTrack {
        ScrobbleTrack(
            title: t.displayTitle,
            artist: t.artists.first?.name ?? "",
            album: t.album?.title ?? "",
            url: "https://tidal.com/track/\(t.id)",
            durationSecs: t.durationSecs
        )
    }

    private func nowPlayingScrobble(_ t: Track) {
        let st = scrobbleTrack(t)
        let prefs = Preferences.shared
        if prefs.scrobbleLastFm, let sk = Keychain.get("lastfm.session") {
            Task.detached { try? await lastfmNowPlaying(sessionKey: sk, track: st) }
        }
        if prefs.scrobbleListenBrainz, let tok = Keychain.get("listenbrainz.token") {
            Task.detached { await listenbrainzNowPlaying(userToken: tok, track: st) }
        }
    }

    private func submitScrobble(_ t: Track) {
        let st = scrobbleTrack(t)
        let ts = Int64((startedAt ?? Date()).timeIntervalSince1970)
        let prefs = Preferences.shared
        if prefs.scrobbleLastFm, let sk = Keychain.get("lastfm.session") {
            Task.detached { try? await lastfmScrobble(sessionKey: sk, track: st, startedAt: ts) }
        }
        if prefs.scrobbleListenBrainz, let tok = Keychain.get("listenbrainz.token") {
            Task.detached { await listenbrainzScrobble(userToken: tok, track: st, listenedAt: ts) }
        }
    }
}

extension Array {
    subscript(safe i: Int) -> Element? { indices.contains(i) ? self[i] : nil }
}

extension Player {
    /// Stops playback and forgets the queue (used by "Erase all app data").
    func stopAndClear() {
        loadTask?.cancel(); preloadTask?.cancel(); refilling?.cancel(); refilling = nil
        liveSeekTask?.cancel(); liveSeekTask = nil; liveSeeking = false
        avPlayer.pause(); avPlayer.removeAllItems()
        resumeAfterInterruption = false
        recentIDs = []; recentOrder = []
        savedQueueVersion = -1; savedQueueOffset = 0
        preloaded = nil; infinite = nil
        queue = []; original = []; userQueue = []; index = nil; current = nil
        isPlaying = false; loading = false; position = 0; duration = 0; stream = nil; sourceTitle = nil
        MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
        needsStart = false; restoredInfinite = false
        // With `current` nil any later persist() (tick, scene change) only clears again;
        // the store's serial queue keeps this delete after any save already queued.
        PlaybackStore.clear()
    }
}

