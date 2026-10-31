import AVFoundation
import MediaPlayer
import Observation
import UIKit

/// Queue + AVPlayer (HLS from TIDAL trackManifests), lock-screen controls and scrobbling.
@MainActor @Observable
final class Player {
    static let shared = Player()

    enum Repeat { case off, all, one }

    /// Base queue (album, playlist, mix...) and position in it.
    private(set) var queue: [Track] = []
    private(set) var index: Int?
    /// "Play next" / "Add to queue" tracks; they play before the rest of the base queue.
    private(set) var userQueue: [Track] = []
    /// Track currently playing (may come from the user queue).
    private(set) var current: Track?
    private(set) var isPlaying = false
    /// Fetching the current track's stream.
    private(set) var loading = false
    private(set) var position: Double = 0
    private(set) var duration: Double = 0
    private(set) var stream: Stream? {
        didSet { if let q = stream?.actualQuality, q != shownQuality { shownQuality = q } }
    }
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
    private var original: [Track] = []

    private let avPlayer = AVQueuePlayer()
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
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
    }

    // MARK: queue

    func play(_ tracks: [Track], startAt: Int = 0, from source: String? = nil) {
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
            } catch { self.error = error.localizedDescription }
        }
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
        if current == nil { play([track]); return }
        userQueue.insert(track, at: 0)
        invalidatePreload()
    }

    func addToQueue(_ track: Track) {
        if current == nil { play([track]); return }
        userQueue.append(track)
        invalidatePreload()
    }

    func removeUserQueue(at offsets: IndexSet) {
        userQueue.remove(atOffsets: offsets)
        invalidatePreload()
    }

    func moveUserQueue(from: IndexSet, to: Int) {
        userQueue.move(fromOffsets: from, toOffset: to)
        invalidatePreload()
    }

    func moveUpNext(from: IndexSet, to: Int) {
        guard let i = index else { return }
        let offset = i + 1
        queue.move(fromOffsets: IndexSet(from.map { $0 + offset }), toOffset: to + offset)
        invalidatePreload()
    }

    func removeUpNext(at offsets: IndexSet) {
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
        if isPlaying { avPlayer.pause() } else { avPlayer.play() }
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

    func cycleRepeat() {
        repeatMode = switch repeatMode { case .off: .all; case .all: .one; case .one: .off }
        invalidatePreload()
    }

    /// Re-requests the current track's manifest after a quality change, keeping the position.
    func reloadQuality() {
        guard let t = current else { return }
        start(t, resumeAt: position)
    }

    // MARK: internals

    /// What plays after the current track: (track, comes from user queue, base index).
    private func following() -> (Track, Bool, Int?)? {
        if repeatMode == .one, let c = current { return (c, false, index) }
        if let u = userQueue.first { return (u, true, index) }
        guard let i = index else { return nil }
        if i + 1 < queue.count { return (queue[i + 1], false, i + 1) }
        if repeatMode == .all, !queue.isEmpty { return (queue[0], false, 0) }
        return nil
    }

    private func advance(auto: Bool) {
        guard let (t, fromUser, newIndex) = following() else {
            isPlaying = false
            updateNowPlaying()
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

    private func load(_ i: Int, resumeAt: Double = 0) {
        guard queue.indices.contains(i) else { return }
        index = i
        start(queue[i], resumeAt: resumeAt)
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

    private func start(_ track: Track, resumeAt: Double = 0) {
        adopt(track, stream: nil)
        loading = true
        position = resumeAt
        invalidatePreload()
        loadTask?.cancel()
        loadTask = Task {
            do {
                let s = try await Torneam.stream(trackId: track.id, quality: Preferences.shared.streamingQuality)
                guard !Task.isCancelled, current?.id == track.id, let url = URL(string: s.url) else { return }
                stream = s
                avPlayer.removeAllItems()
                avPlayer.insert(AVPlayerItem(url: url), after: nil)
                if resumeAt > 0 { await avPlayer.seek(to: CMTime(seconds: resumeAt, preferredTimescale: 600)) }
                avPlayer.play()
                isPlaying = true
                loading = false
                afterStart(track)
            } catch {
                if !Task.isCancelled { loading = false }
                if !Task.isCancelled { self.error = error.localizedDescription }
            }
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

    /// Prepares the next track's AVPlayerItem behind the current one (gapless-ish start).
    private func schedulePreload() {
        preloadTask?.cancel()
        guard Preferences.shared.preloadNext, repeatMode != .one,
              let (next, _, _) = following(), let cur = current else { return }
        preloadTask = Task {
            guard let s = try? await Torneam.stream(trackId: next.id, quality: Preferences.shared.streamingQuality),
                  !Task.isCancelled, current?.id == cur.id, let url = URL(string: s.url),
                  let curItem = avPlayer.currentItem else { return }
            let item = AVPlayerItem(url: url)
            item.preferredForwardBufferDuration = 10
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
        guard t.isFinite else { return }
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
        if repeatMode == .one, let c = current { start(c); return }
        DispatchQueue.main.async { self.advance(auto: true) }
    }

    private func refillIfNeeded() {
        guard let source = infinite, let i = index, queue.count - i <= 3 else { return }
        Task {
            guard let more = try? await source.nextBatch(min: 10), infinite === source else { return }
            queue.append(contentsOf: more)
            original.append(contentsOf: more)
        }
    }

    // MARK: system integration

    private func configureSession() {
        let s = AVAudioSession.sharedInstance()
        try? s.setCategory(.playback, mode: .default, policy: .longFormAudio)
        try? s.setActive(true)
    }

    private func configureRemote() {
        let c = MPRemoteCommandCenter.shared()
        c.playCommand.addTarget { [weak self] _ in
            MainActor.assumeIsolated { if self?.isPlaying == false { self?.toggle() } }
            return .success
        }
        c.pauseCommand.addTarget { [weak self] _ in
            MainActor.assumeIsolated { if self?.isPlaying == true { self?.toggle() } }
            return .success
        }
        c.togglePlayPauseCommand.addTarget { [weak self] _ in
            MainActor.assumeIsolated { self?.toggle() }
            return .success
        }
        c.nextTrackCommand.addTarget { [weak self] _ in
            MainActor.assumeIsolated { self?.next() }
            return .success
        }
        c.previousTrackCommand.addTarget { [weak self] _ in
            MainActor.assumeIsolated { self?.previous() }
            return .success
        }
        c.changePlaybackPositionCommand.addTarget { [weak self] event in
            guard let e = event as? MPChangePlaybackPositionCommandEvent else { return .commandFailed }
            MainActor.assumeIsolated { self?.seek(to: e.positionTime) }
            return .success
        }
    }

    private func updateNowPlaying() {
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
        guard let url = track.coverURLLarge,
              let (data, _) = try? await URLSession.shared.data(from: url),
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
        loadTask?.cancel(); preloadTask?.cancel()
        avPlayer.pause(); avPlayer.removeAllItems()
        preloaded = nil; infinite = nil
        queue = []; original = []; userQueue = []; index = nil; current = nil
        isPlaying = false; loading = false; position = 0; duration = 0; stream = nil; sourceTitle = nil
        MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
    }
}
