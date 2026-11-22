import AVFoundation
import Combine
import MediaPlayer
import Observation
import UIKit

/// Queue + AVPlayer (HLS from TIDAL trackManifests), lock-screen controls and scrobbling.
@MainActor @Observable
final class Player: LivePlaybackPort {
    static let shared = Player()

    enum Repeat { case off, all, one }

    /// Base queue (album, playlist, mix...) and position in it.
    private(set) var queue: [Track] = [] { didSet { queueVersion &+= 1 } }
    private(set) var index: Int?
    /// "Play next" / "Add to queue" tracks; they play before the rest of the base queue.
    private(set) var userQueue: [Track] = [] { didSet { queueVersion &+= 1 } }
    /// Track currently playing.
    private(set) var current: Track?
    private(set) var isPlaying = false {
        didSet { if isPlaying != oldValue { scheduleSessionRelease() } }
    }
    /// Releases the audio session after a pause so iOS can suspend the app.
    @ObservationIgnored private var releaseTask: Task<Void, Never>?
    /// How long a paused player keeps the audio session (and the background audio grant).
    private static let idleRelease: Duration = .seconds(10 * 60)
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
    /// Base-queue tracks before the current one: repeat all plays them again after the last.
    var played: ArraySlice<Track> { index.map { queue.prefix($0) } ?? [] }
    /// The queue is Discover's endless shuffle, also before its source is attached again.
    var endless: Bool { infinite != nil || restoredInfinite }
    /// Base queue in its unshuffled order, restored when shuffle is turned off.
    private var original: [Track] = [] { didSet { queueVersion &+= 1 } }

    private let avPlayer = AVQueuePlayer()
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
    private var systemObservers: [NSObjectProtocol] = []
    /// Set when an audio-session interruption paused active playback.
    private var resumeAfterInterruption = false
    @ObservationIgnored lazy var liveEngine = LiveEngine(port: self)
    private var infinite: Shuffle?
    private var loadTask: Task<Void, Never>?
    private var preloadTask: Task<Void, Never>?
    @ObservationIgnored private var advanceTask: Task<Void, Never>?
    private var preloaded: (trackId: String, item: AVPlayerItem, stream: Stream)?
    /// The engine item of `current`: ends of other items are stale.
    @ObservationIgnored private var playingItem: AVPlayerItem? {
        didSet { if playingItem !== oldValue { watchFailure(playingItem) } }
    }
    @ObservationIgnored private var failureWatch: AnyCancellable?
    @ObservationIgnored private var refetched: (id: String, at: Date)?
    @ObservationIgnored private var itemObserver: AnyCancellable?
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
        itemObserver = avPlayer.publisher(for: \.currentItem).receive(on: DispatchQueue.main).sink { [weak self] item in
            MainActor.assumeIsolated { self?.engineMoved(to: item) }
        }
        observeSystemAudio()
    }

    // MARK: queue

    func play(_ tracks: [Track], startAt: Int = 0, from source: String? = nil) {
        if Live.shared.guestRefuses() { return }
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
        if Live.shared.guestRefuses() { return }
        guard !tracks.isEmpty else { return }
        shuffle = true
        play(tracks, startAt: Int.random(in: tracks.indices), from: source)
    }

    /// Discover's shuffle: endless queue mixing mixes, radios and discoveries.
    func playInfinite(mixes: [Mix]) {
        if Live.shared.guestRefuses() { return }
        recentIDs = []; recentOrder = []
        restoredInfinite = false
        let source = Shuffle(mixes: mixes)
        infinite = source
        sourceTitle = "Discover"
        Task {
            do {
                let batch = try await source.nextBatch(min: 10)
                guard !liveEngine.liveFollowing, infinite === source, !batch.isEmpty else { return }
                queue = batch
                original = batch
                load(0)
            } catch {
                guard infinite === source else { return }
                self.error = friendlyMessage(error)
            }
        }
    }

    /// Tapping one Discover cover: that song first, then the rest of the visible Discover
    /// pool (from the tapped one onwards, wrapping), then the endless Discover shuffle.
    func playDiscover(_ track: Track, pool: [Track], mixes: [Mix]) {
        if Live.shared.guestRefuses() { return }
        recentIDs = []; recentOrder = []
        restoredInfinite = mixes.isEmpty
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

    /// Playback taken over from another device (Handoff): `tracks` already are in
    /// play order, so shuffle is off and the personal queue is replaced too.
    /// `discover`: Discover's endless queue, which goes on with this device's own shuffle.
    func resume(_ tracks: [Track], at secs: Double, repeat r: Repeat, from device: String, discover: Bool = false) {
        guard !tracks.isEmpty else { return }
        recentIDs = []; recentOrder = []
        restoredInfinite = discover
        infinite = nil
        shuffle = false
        original = tracks
        queue = tracks
        userQueue = []
        sourceTitle = discover ? "Discover" : (device.isEmpty ? nil : device)
        repeatMode = r
        // `start` swaps the item only once the new stream is fetched; the old song must not keep playing meanwhile.
        avPlayer.pause()
        avPlayer.removeAllItems()
        // Where the other device was, not the nearest segment start.
        exactResume = true
        load(0, resumeAt: max(secs, 0))
        resumedVersion = queueVersion
        if discover { attachDiscover() }
    }

    /// Asks for the home mixes while a Discover queue restored or taken over waits for its source: without it,
    /// playback ends with the tracks it has.
    private func attachDiscover() {
        discoverWait &+= 1
        let wait = discoverWait
        Task {
            var pause = 2.0
            // A newer resume or a sign-out ends it: mixes on their way are not that queue's.
            while wait == discoverWait, restoredInfinite {
                if let mixes = try? await homeMixes(), !mixes.isEmpty {
                    if wait == discoverWait { resumeInfinite(mixes: mixes) }
                    return
                }
                try? await Task.sleep(for: .seconds(pause))
                pause = min(pause * 2, 60)
            }
        }
    }

    /// What follows a handoff's resumed track `id`, only while the queues still are what that resume set up: else
    /// the user started or queued something else, or it is a duplicate. No tracks: nothing follows it, and a
    /// Discover queue goes on from its own source. Returns whether tracks were appended.
    @discardableResult
    func appendUpNext(after id: String, _ tracks: [Track]) -> Bool {
        guard resumedVersion == queueVersion, index != nil, current?.id == id else { return false }
        resumedVersion = nil
        guard !tracks.isEmpty else { refillIfNeeded(); return false }
        queue.append(contentsOf: tracks)
        original.append(contentsOf: tracks)
        invalidatePreload()
        persist()
        refillIfNeeded()
        return true
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

    func playNext(_ track: Track) { playNext([track]) }

    /// With nothing playing the list plays in order from its first track, shuffle or not, like Add to Queue.
    func playNext(_ tracks: [Track]) {
        defer { persist() }
        if current == nil, let first = tracks.first {
            userQueue.insert(contentsOf: tracks.dropFirst(), at: 0)
            play([first])
            return
        }
        userQueue.insert(contentsOf: tracks, at: 0)
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
        let mirrored = original == queue
        queue.move(fromOffsets: IndexSet(from.map { $0 + offset }), toOffset: to + offset)
        if mirrored { original = queue }
        invalidatePreload()
    }

    func removeUpNext(at offsets: IndexSet) {
        defer { persist() }
        guard let i = index else { return }
        queue.remove(atOffsets: IndexSet(offsets.map { $0 + i + 1 }))
        original = original.keeping(queue, by: \.id)
        invalidatePreload()
    }

    func jump(toUpNext offset: Int) {
        if Live.shared.guestRefuses() { return }
        guard let i = index else { return }
        load(i + 1 + offset)
    }

    func jump(toUser offset: Int) {
        if Live.shared.guestRefuses() { return }
        guard userQueue.indices.contains(offset) else { return }
        let t = userQueue[offset]
        userQueue.removeFirst(offset + 1)
        index = joinQueue(t)
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
        liveEngine.clearSystemPause()
        pausedByUser = isPlaying
        if isPlaying {
            plays += 1
            // A start still loading would otherwise autoplay over this pause.
            if loading { holdAutoplay = true }
            avPlayer.pause()
        } else {
            // A play after a pause during a start: the start ends playing after all.
            holdAutoplay = false
            // Playing here is a newer choice than a handoff still on its way.
            plays += 1
            try? AVAudioSession.sharedInstance().setActive(true)
            avPlayer.play()
        }
        isPlaying.toggle()
        updateNowPlaying()
    }

    /// Pauses, also a start still loading: it then ends paused.
    func pause() {
        plays += 1
        pausedByUser = true
        // A start still loading ends paused; an item already going but not yet marked playing stops.
        if loading { holdAutoplay = true }
        resumeAfterInterruption = false
        if isPlaying { toggle() } else { avPlayer.pause() }
    }

    func next() { advance(auto: false) }

    func previous() {
        guard let i = index else { return }
        if position > 3 || i == 0 { seek(to: 0) } else { load(i - 1) }
    }

    func seek(to secs: Double) {
        seekGeneration += 1
        let generation = seekGeneration
        seeking = true
        avPlayer.seek(to: CMTime(seconds: secs, preferredTimescale: 600)) { [weak self] _ in
            Task { @MainActor in
                if self?.seekGeneration == generation { self?.seeking = false }
            }
        }
        position = secs
        updateNowPlaying()
    }

    /// True while a `seek(to:)` is in flight: the time observer still reports
    /// the old time then, which would snap the slider back for a moment.
    @ObservationIgnored private var seeking = false
    @ObservationIgnored private var seekGeneration = 0

    // MARK: Listen Together playback adapter

    var liveHasItem: Bool { liveEngine.liveHasItem }
    var liveHostPosition: Double { liveEngine.liveHostPosition }
    var liveItem: (any LivePlaybackItem)? { avPlayer.currentItem }
    var isAdvancing: Bool { avPlayer.timeControlStatus == .playing }
    var needsLiveLoad: Bool { needsStart }
    var enginePosition: Double { avPlayer.currentTime().seconds }

    func prepareLiveJoin() {
        starts += 1
        loadTask?.cancel(); loadTask = nil
        advanceTask?.cancel(); advanceTask = nil
        refilling?.cancel(); refilling = nil
        infinite = nil
        restoredInfinite = false
        discoverWait &+= 1
        resumedVersion = nil
        resumeAfterInterruption = false
        holdAutoplay = false
        clearPreload()
        seekGeneration += 1
        seeking = false
        loading = false
        liveEngine.loadResumeAt = nil
        avPlayer.pause()
        avPlayer.removeAllItems()
        playingItem = nil
        needsStart = current != nil
        if isPlaying || current != nil { reportPlaying(false) }
    }

    func loadLive(_ tracks: [Track], index i: Int, at secs: Double, playing: Bool) {
        recentIDs = []; recentOrder = []
        infinite = nil
        restoredInfinite = false
        discoverWait &+= 1
        resumedVersion = nil
        original = tracks
        queue = tracks
        sourceTitle = "Listen Together"
        load(i, resumeAt: secs, autoplay: playing)
    }

    func seekExactly(_ secs: Double) async -> Bool {
        await avPlayer.seek(to: CMTime(seconds: secs, preferredTimescale: 600),
                            toleranceBefore: .zero, toleranceAfter: .zero)
    }

    func pauseEngine() { avPlayer.pause() }
    func playEngine() { avPlayer.play() }
    func activateAudio() { try? AVAudioSession.sharedInstance().setActive(true) }

    func prerollIfReady() async {
        if avPlayer.status == .readyToPlay { _ = await avPlayer.preroll(atRate: 1) }
    }

    func scheduleLiveStart(after seconds: Double) -> () -> Bool {
        let clock = CMClockGetHostTimeClock()
        let startAt = CMClockGetTime(clock) + CMTime(seconds: seconds, preferredTimescale: 1_000_000_000)
        avPlayer.automaticallyWaitsToMinimizeStalling = false
        avPlayer.setRate(1, time: .invalid, atHostTime: startAt)
        return { (startAt - CMClockGetTime(clock)).seconds <= 0 }
    }

    func finishLiveStart(started: Bool, sameItem: Bool) {
        if !started, sameItem { avPlayer.pause() }
        avPlayer.automaticallyWaitsToMinimizeStalling = true
        // AVPlayer can then resume on its own when it buffers.
        if started, avPlayer.rate == 0 { avPlayer.play() }
    }

    func prepareLiveResume() {
        resumeAfterInterruption = false
        pausedByUser = false
        activateAudio()
        reportPlaying(true)
    }

    func reportPlaying(_ playing: Bool) {
        isPlaying = playing
        updateNowPlaying()
    }

    func setLivePosition(_ secs: Double, publish: Bool) {
        position = secs
        if publish { updateNowPlaying() }
    }

    func cycleRepeat() {
        repeatMode = switch repeatMode { case .off: .all; case .all: .one; case .one: .off }
        syncRemote()
        invalidatePreload()
    }

    /// A track not loaded yet (restored, or a failed start) fetches the new quality on its first play.
    func reloadQuality() {
        guard let t = current, !needsStart, !isOffline else { return }
        start(t, resumeAt: position, autoplay: isPlaying || (loading && !pausedByUser))
    }

    // MARK: internals

    /// What plays after the current track: (track, comes from user queue, base index).
    /// `manual`: an explicit Next skips past a repeat-one track (it only loops on natural end).
    private func following(manual: Bool = false) -> (Track, Bool, Int?)? {
        if liveEngine.liveFollowing { return nil }
        if repeatMode == .one, !manual, let c = current { return (c, false, index) }
        if let u = userQueue.first { return (u, true, index) }
        guard let i = index else { return nil }
        if i + 1 < queue.count { return (queue[i + 1], false, i + 1) }
        if repeatMode == .all, !queue.isEmpty, resumedVersion != queueVersion { return (queue[0], false, 0) }
        return nil
    }

    private func advance(auto: Bool) {
        guard !liveEngine.liveFollowing else { return }
        guard let (t, fromUser, newIndex) = following(manual: !auto) else {
            // Endless (Discover) or handed-off queue ran dry before its tracks arrived: wait, then go on.
            guard infinite != nil || restoredInfinite || resumedVersion == queueVersion else { stopAtEnd(); return }
            loading = true
            // Restored or taken over, and its source is not here yet: asked for now.
            if restoredInfinite, infinite == nil { attachDiscover() }
            let started = starts
            advanceTask?.cancel()
            advanceTask = Task {
                // A handoff's own tracks come first, when they are still on their way.
                await awaitHandoff(manual: !auto, started: started)
                // Something else plays by now, or nothing does (a sign-out).
                guard !Task.isCancelled, starts == started, current != nil else { return }
                guard !liveEngine.liveFollowing else { loading = false; return }
                if resumedVersion == queueVersion { resumedVersion = nil }
                // The handed-over tracks play without the source.
                if following(manual: !auto) == nil {
                    guard let source = infinite else { loading = false; stopAtEnd(); return }
                    let more = await fetchMore(from: source)
                    // Another Next tapped meanwhile waited too, and went on first.
                    guard !Task.isCancelled, infinite === source, starts == started else { return }
                    guard !liveEngine.liveFollowing else { loading = false; return }
                    guard more else { loading = false; stopAtEnd(); return }
                }
                loading = false
                // A pause while the refill loaded holds for the track it starts.
                let held = holdAutoplay
                advance(auto: auto)
                if held { holdAutoplay = true }
            }
            return
        }
        advanceTask?.cancel(); advanceTask = nil
        if fromUser {
            userQueue.removeFirst()
            index = joinQueue(t)
        } else {
            index = newIndex
        }
        if auto, let p = preloaded, p.trackId == t.id, avPlayer.currentItem === p.item, p.item.status != .failed {
            preloaded = nil
            let play = autoplayAfterEnd()
            playingItem = p.item
            adopt(t, stream: p.stream)
            if play { avPlayer.play() } else { avPlayer.pause() }
            isPlaying = play
            afterStart(t)
        } else {
            start(t, autoplay: auto ? autoplayAfterEnd() : true)
        }
    }

    /// The user's last command was a pause. Unlike `isPlaying` it does not drop when an item ends,
    /// so a track change the engine makes seconds later can still honor that pause.
    @ObservationIgnored private var pausedByUser = false

    private func autoplayAfterEnd() -> Bool { !pausedByUser && !liveEngine.systemPaused }

    /// A starting user-queue track joins the base queue right after the current track, so Previous
    /// and Next find it again. Returns its index.
    private func joinQueue(_ t: Track) -> Int {
        let at = index.map { $0 + 1 } ?? 0
        if original == queue {
            original.insert(t, at: at)
        } else {
            let cur = index.flatMap { queue[safe: $0] }
            let o = cur.flatMap { c in original.firstIndex { $0.id == c.id } }.map { $0 + 1 } ?? original.count
            original.insert(t, at: o)
        }
        queue.insert(t, at: at)
        return at
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

    private func start(_ track: Track, resumeAt: Double = 0, autoplay: Bool = true, online: Bool = false) {
        needsStart = false
        holdAutoplay = false
        pausedByUser = !autoplay
        // Playing again ends an earlier system pause, as `toggle` does.
        if autoplay {
            resumeAfterInterruption = false
            liveEngine.clearSystemPause()
        }
        starts += 1
        let mine = starts
        let exact = exactResume
        exactResume = false
        adopt(track, stream: nil)
        loading = true
        position = resumeAt
        liveEngine.loadResumeAt = resumeAt
        // The old item may still end while the new stream loads; that end is not this track's.
        playingItem = nil
        advanceTask?.cancel(); advanceTask = nil
        clearPreload()
        loadTask?.cancel()
        loadTask = Task {
            do {
                let s = try await online ? Self.online(track.id) : Self.source(track.id)
                guard !Task.isCancelled, starts == mine, current?.id == track.id, let url = URL(string: s.url) else { return }
                stream = s
                let shouldAlign = liveEngine.shouldAlign(autoplay: autoplay && !holdAutoplay)
                if shouldAlign || resumeAt > 0 || !autoplay || holdAutoplay { avPlayer.pause() }
                avPlayer.removeAllItems()
                let item = Self.makeItem(url)
                avPlayer.insert(item, after: nil)
                playingItem = item
                if shouldAlign {
                    // Following a playing host: start only with a healthy buffer
                    // and on time, so nothing needs correcting after the start.
                    let playing = await liveEngine.liveAlign(item)
                    guard !Task.isCancelled, starts == mine, current?.id == track.id else { return }
                    // Only a failure drops the item here; `streamFailed` recovers it.
                    guard avPlayer.currentItem === item else { return }
                    isPlaying = playing
                    loading = false
                    liveEngine.loadResumeAt = nil
                    liveEngine.alignmentFinished()
                    afterStart(track)
                    return
                }
                if resumeAt > 0 {
                    let tol: CMTime = liveEngine.liveFollowing || exact ? .zero : .positiveInfinity
                    await avPlayer.seek(to: CMTime(seconds: resumeAt, preferredTimescale: 600),
                                        toleranceBefore: tol, toleranceAfter: tol)
                    // A newer start replaced this one during the seek: its item is not ours to play.
                    guard !Task.isCancelled, starts == mine else { return }
                }
                if autoplay, !holdAutoplay {
                    try? AVAudioSession.sharedInstance().setActive(true)
                    avPlayer.play()
                    isPlaying = true
                } else {
                    avPlayer.pause()
                    isPlaying = false
                }
                loading = false
                if !Task.isCancelled { liveEngine.loadResumeAt = nil }
                afterStart(track)
            } catch {
                guard !Task.isCancelled, starts == mine else { return }
                fail(friendlyMessage(error))
            }
        }
    }

    /// The status publisher emits the current value too, so an item that failed before it was watched counts.
    private func watchFailure(_ item: AVPlayerItem?) {
        failureWatch = item.map { item in
            item.publisher(for: \.status).filter { $0 == .failed }.map { _ in }
                .merge(with: NotificationCenter.default
                    .publisher(for: AVPlayerItem.failedToPlayToEndTimeNotification, object: item).map { _ in })
                .first()
                .receive(on: DispatchQueue.main)
                .sink { [weak self] in MainActor.assumeIsolated { self?.streamFailed(item) } }
        }
    }

    private func streamFailed(_ item: AVPlayerItem) {
        guard item === playingItem, let t = current else { return }
        let now = Date()
        // A broken download spends no budget: a new play picks the same file again, and only its online stream can help.
        if !isOffline {
            guard Self.refetchDue(refetched, t.id, now: now) else {
                fail(item.error.map { friendlyMessage($0) } ?? "This track can't be played right now.")
                return
            }
            refetched = (t.id, now)
        }
        // While a start loads, `position` follows the engine, not where the start was meant to begin.
        restart(t, at: liveEngine.loadResumeAt ?? position)
    }

    /// A dead URL fails again within seconds, an expired one hours later.
    private static func refetchDue(_ last: (id: String, at: Date)?, _ id: String, now: Date) -> Bool {
        guard let last else { return true }
        return last.id != id || now.timeIntervalSince(last.at) >= retryWindow
    }

    private static let retryWindow: TimeInterval = 60

    /// Still the same listen: its scrobble progress survives the new start.
    private func restart(_ t: Track, at secs: Double) {
        let kept = (listened, scrobbled, startedAt, artwork)
        exactResume = true
        // Not `isPlaying`: the rate observer may already have dropped it for the failure.
        start(t, resumeAt: secs, autoplay: autoplayAfterEnd(), online: true)
        (listened, scrobbled, startedAt, artwork) = kept
    }

    private func fail(_ message: String) {
        loadTask?.cancel(); loadTask = nil
        loading = false; liveEngine.loadResumeAt = nil
        error = message
        avPlayer.pause(); avPlayer.removeAllItems(); playingItem = nil
        isPlaying = false
        needsStart = true
        updateNowPlaying()
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
        return try await online(id)
    }

    /// The stream's best variant only: given the master playlist, AVPlayer starts a queued item on a
    /// lower variant and switches up while it plays, and a switch that changes the sample rate
    /// (Hi-Res next to 44.1 kHz) replays the track from 0.
    private static func online(_ id: String) async throws -> Stream {
        var s = try await SDAT.stream(trackId: id, quality: Preferences.shared.streamingQuality)
        if let u = await Downloads.bestVariantURL(of: s.url) { s.url = u.absoluteString }
        return s
    }

    /// Prepares the next track's AVPlayerItem behind the current one (gapless-ish start).
    private func schedulePreload() {
        preloadTask?.cancel(); preloadTask = nil
        guard !liveEngine.liveFollowing, Preferences.shared.preloadNext, repeatMode != .one,
              let (next, _, _) = following(), let cur = current else { return }
        let started = starts
        preloadTask = Task {
            guard let s = try? await Self.source(next.id),
                  !Task.isCancelled, !liveEngine.liveFollowing, starts == started,
                  current?.id == cur.id, let url = URL(string: s.url),
                  let curItem = avPlayer.currentItem, curItem === playingItem else { return }
            let item = Self.makeItem(url)
            guard avPlayer.canInsert(item, after: curItem) else { return }
            avPlayer.insert(item, after: curItem)
            preloaded = (next.id, item, s)
        }
    }

    private func clearPreload() {
        preloadTask?.cancel(); preloadTask = nil
        if let p = preloaded { avPlayer.remove(p.item) }
        preloaded = nil
    }

    private func invalidatePreload() {
        clearPreload()
        if !liveEngine.liveFollowing, current != nil, stream != nil { schedulePreload() }
    }

    private func tick(_ t: Double) {
        guard t.isFinite, !needsStart, !seeking else { return }
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
        guard let item, item === playingItem else { return }
        // Listen Together guest: wait for the host's next track instead of advancing.
        if liveEngine.liveFollowing { DispatchQueue.main.async { self.avPlayer.pause(); self.isPlaying = false }; return }
        if repeatMode == .one, let c = current { start(c); return }
        // The engine moves on to a queued preload by itself, sometimes only after this notification
        // (e.g. when the sample rate changes); `engineMoved` adopts it then. Starting the track
        // anew here would drop the preload that is already playing and replay it from 0.
        if let p = preloaded, avPlayer.items().contains(p.item) {
            // A preload that fails, is removed or never comes up loads the next track the usual way.
            let s = starts, g = seekGeneration
            Task { [weak self] in
                for _ in 0..<20 {
                    try? await Task.sleep(for: .milliseconds(500))
                    guard let self, !self.liveEngine.liveFollowing, self.playingItem === item, self.starts == s, self.seekGeneration == g else { return }
                    guard let p = self.preloaded, p.item.status != .failed,
                          self.avPlayer.items().contains(p.item) else { break }
                }
                guard let self, !self.liveEngine.liveFollowing, self.playingItem === item, self.starts == s, self.seekGeneration == g else { return }
                self.advance(auto: true)
            }
            return
        }
        DispatchQueue.main.async { self.advance(auto: true) }
    }

    /// A failed preload is left to the fallback in `trackEnded`, which loads the track anew.
    private func engineMoved(to item: AVPlayerItem?) {
        guard !liveEngine.liveFollowing else { return }
        guard let item, item === preloaded?.item, item === avPlayer.currentItem, item !== playingItem,
              item.status != .failed else { return }
        advance(auto: true)
    }

    /// Waits while a queue that ran dry may still get more: the tracks after a handoff's resumed one and the
    /// source of a Discover queue. A track to play ends it, and so does another start (`starts` is no longer
    /// `started`). The tracks have a limit, for an answer of the core that never comes; the source is asked for
    /// until it comes.
    private func awaitHandoff(manual: Bool, started: Int) async {
        let limit = Date().addingTimeInterval(Self.handoffWait)
        while !Task.isCancelled, !liveEngine.liveFollowing, starts == started, current != nil, following(manual: manual) == nil {
            let tracks = resumedVersion == queueVersion && Date() < limit
            let source = restoredInfinite && infinite == nil
            guard tracks || source else { return }
            try? await Task.sleep(for: .milliseconds(100))
        }
    }

    private func refillIfNeeded() {
        // A handoff's own tracks come first: a refill before them would take their place.
        guard !liveEngine.liveFollowing, resumedVersion != queueVersion, let source = infinite, let i = index, queue.count - i <= 5 else { return }
        Task { _ = await fetchMore(from: source) }
    }

    @ObservationIgnored private var refilling: Task<Bool, Never>?
    /// Appends a new endless batch, skipping tracks already in the queue. One fetch at a time;
    /// returns whether anything was added.
    private func fetchMore(from source: Shuffle) async -> Bool {
        if let r = refilling { return await r.value }
        let task = Task { @MainActor () -> Bool in
            defer { if !Task.isCancelled { refilling = nil } }
            for _ in 0..<3 {
                guard !Task.isCancelled, !liveEngine.liveFollowing,
                      let batch = try? await source.nextBatch(min: 10),
                      !Task.isCancelled, !liveEngine.liveFollowing, infinite === source else { return false }
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
    /// `pause()` came while a start was loading.
    @ObservationIgnored private var holdAutoplay = false
    /// The next start seeks to its position exactly (a handoff).
    @ObservationIgnored private var exactResume = false
    /// Bumped on every start: tells one playback from a later one of the same track.
    @ObservationIgnored private(set) var starts = 0
    /// `queueVersion` right after a handoff resume, until what follows its track is appended: any change to the
    /// queues since ends it, a restart or a repeat does not.
    @ObservationIgnored private var resumedVersion: Int?
    /// Bumped by every Discover resume and by a sign-out: an older request for the home mixes is dropped.
    @ObservationIgnored private var discoverWait = 0
    private static let handoffWait: TimeInterval = 20
    /// Bumped by every play or pause command, a resume of the same track too.
    @ObservationIgnored private(set) var plays = 0
    @ObservationIgnored private var restoredInfinite = false
    @ObservationIgnored private var lastSave = Date.distantPast
    @ObservationIgnored private var restoring = true

    @ObservationIgnored private var queueVersion = 0
    @ObservationIgnored private var savedQueueVersion = -1
    @ObservationIgnored private var savedQueueRange = 0..<0
    @ObservationIgnored private var queueID = 0

    /// Cheap: the small state file every call; the queue file only when the queues changed.
    /// Both are bounded windows (see PlaybackStore caps), JSON-encoded off the main thread.
    func persist() {
        guard !restoring else { return }
        lastSave = Date()
        guard let c = current else { PlaybackStore.clear(); savedQueueVersion = -1; return }
        let e = DiscoverCache.encode
        let endless = infinite != nil || restoredInfinite
        if queueVersion != savedQueueVersion || !(index.map { savedQueueRange.contains($0) } ?? true) {
            let w = PlaybackStore.window(count: queue.count, index: index, endless: endless)
            // The window's tracks in their unshuffled order, so turning shuffle off after a restart finds the current one.
            let orig = original.keeping(queue[w], by: \.id)
            queueID &+= 1
            PlaybackStore.saveQueue(.init(queue: e(Array(queue[w])), original: e(orig),
                                          userQueue: e(Array(userQueue.prefix(PlaybackStore.maxSaved))),
                                          infinite: endless, sourceTitle: sourceTitle, id: queueID))
            savedQueueVersion = queueVersion
            savedQueueRange = w
        }
        let rm: String = switch repeatMode { case .off: "off"; case .all: "all"; case .one: "one" }
        PlaybackStore.saveState(.init(current: e([c])[0], index: index.map { $0 - savedQueueRange.lowerBound },
                                      position: position, repeatMode: rm, shuffle: shuffle, queueID: queueID))
    }

    private func restore() async {
        defer { restoring = false }
        guard let (s, q) = await PlaybackStore.load(), current == nil, !liveEngine.liveFollowing,
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
        guard !liveEngine.liveFollowing, restoredInfinite, infinite == nil, !mixes.isEmpty else { return }
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
                    if self.isPlaying || (self.loading && !self.holdAutoplay && !self.pausedByUser) {
                        self.resumeAfterInterruption = true
                        self.liveEngine.noteSystemPause()
                    }
                    self.markPaused()
                case .ended:
                    let should = AVAudioSession.InterruptionOptions(rawValue: opts).contains(.shouldResume)
                    let resume = should && self.resumeAfterInterruption && !self.needsStart && self.current != nil
                    self.resumeAfterInterruption = false
                    if resume {
                        self.pausedByUser = false
                        self.liveEngine.clearSystemPause()
                        if self.liveEngine.liveFollowing {
                            if self.loading {
                                self.holdAutoplay = false
                            } else if self.liveEngine.liveHasItem, !self.liveEngine.liveSeeking,
                                      self.liveEngine.liveTarget?()?.playing == true {
                                self.liveEngine.liveResume()
                            }
                        } else if self.loading {
                            self.holdAutoplay = false
                        } else {
                            try? AVAudioSession.sharedInstance().setActive(true)
                            self.avPlayer.play()
                            self.isPlaying = true
                            self.updateNowPlaying()
                        }
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
                      let self, self.isPlaying || (self.loading && !self.holdAutoplay && !self.pausedByUser) else { return }
                self.resumeAfterInterruption = false
                self.liveEngine.noteSystemPause()
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
        if loading { holdAutoplay = true }
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
            guard let e = event as? MPChangePlaybackPositionCommandEvent,
                  e.positionTime >= 0, e.positionTime < Double(Int64.max) / 1000 else { return .commandFailed }
            MainActor.assumeIsolated {
                if !Live.shared.userSeek(e.positionTime) { self?.seek(to: e.positionTime) }
            }
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

    /// The elements also in `other`, in this array's order: as many of each id as `other` has.
    func keeping<ID: Hashable>(_ other: some Sequence<Element>, by id: (Element) -> ID) -> [Element] {
        var left: [ID: Int] = [:]
        for e in other { left[id(e), default: 0] += 1 }
        return filter { e in
            guard let n = left[id(e)], n > 0 else { return false }
            left[id(e)] = n - 1
            return true
        }
    }
}

extension Player {
    /// Stops playback and forgets the queue (used by "Erase all app data").
    func stopAndClear() {
        starts += 1
        loadTask?.cancel(); loadTask = nil
        advanceTask?.cancel(); advanceTask = nil
        refilling?.cancel(); refilling = nil
        clearPreload()
        liveEngine.cancelLiveSeek()
        avPlayer.pause(); avPlayer.removeAllItems()
        resumeAfterInterruption = false
        recentIDs = []; recentOrder = []
        savedQueueVersion = -1; savedQueueRange = 0..<0
        playingItem = nil; infinite = nil
        queue = []; original = []; userQueue = []; index = nil; current = nil
        isPlaying = false; loading = false; liveEngine.loadResumeAt = nil; position = 0; duration = 0; stream = nil; sourceTitle = nil
        MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
        needsStart = false; restoredInfinite = false; discoverWait &+= 1
        // With `current` nil any later persist() (tick, scene change) only clears again;
        // the store's serial queue keeps this delete after any save already queued.
        PlaybackStore.clear()
    }
}



extension AVPlayerItem: LivePlaybackItem {
    var liveDuration: Double { duration.seconds }
    var liveFailed: Bool { status == .failed }
    var liveBufferHealthy: Bool { isPlaybackLikelyToKeepUp || isPlaybackBufferFull }

    func liveBufferedAhead(from secs: Double) -> Double {
        for v in loadedTimeRanges {
            let r = v.timeRangeValue
            let s = r.start.seconds, e = (r.start + r.duration).seconds
            if s.isFinite, e.isFinite, s <= secs + 0.5, e > secs { return e - secs }
        }
        return 0
    }
}
