import AVFoundation
import MediaPlayer
import Observation
import UIKit

/// Queue + AVPlayer (HLS from TIDAL trackManifests), lock-screen controls and scrobbling.
@MainActor @Observable
final class Player {
    static let shared = Player()

    enum Repeat { case off, all, one }

    private(set) var queue: [Track] = []
    private(set) var index: Int?
    private(set) var isPlaying = false
    private(set) var position: Double = 0
    private(set) var duration: Double = 0
    private(set) var stream: Stream?
    private(set) var sourceTitle: String?
    var shuffle = false
    var repeatMode: Repeat = .off
    var error: String?

    var current: Track? { index.flatMap { queue.indices.contains($0) ? queue[$0] : nil } }
    var progress: Double { duration > 0 ? min(position / duration, 1) : 0 }
    /// Tracks after the current one.
    var upNext: ArraySlice<Track> { index.map { queue.dropFirst($0 + 1) } ?? queue[...] }

    private let avPlayer = AVPlayer()
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
    private var infinite: Shuffle?
    private var loadTask: Task<Void, Never>?
    private var startedAt: Date?
    private var listened: Double = 0
    private var scrobbled = false
    private var artwork: MPMediaItemArtwork?

    private init() {
        configureSession()
        configureRemote()
        timeObserver = avPlayer.addPeriodicTimeObserver(
            forInterval: CMTime(seconds: 0.5, preferredTimescale: 600), queue: .main
        ) { [weak self] time in
            MainActor.assumeIsolated { self?.tick(time.seconds) }
        }
        endObserver = NotificationCenter.default.addObserver(
            forName: AVPlayerItem.didPlayToEndTimeNotification, object: nil, queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated { self?.trackEnded() }
        }
    }

    // MARK: queue

    func play(_ tracks: [Track], startAt: Int = 0, from source: String? = nil) {
        infinite = nil
        var list = tracks
        var start = startAt
        if shuffle, !list.isEmpty {
            let first = list.remove(at: startAt)
            list.shuffle()
            list.insert(first, at: 0)
            start = 0
        }
        queue = list
        sourceTitle = source
        load(start)
    }

    /// Discover's shuffle: endless queue mixing mixes, radios and discoveries.
    func playInfinite(mixes: [Mix]) {
        let source = Shuffle(mixes: mixes)
        infinite = source
        queue = []
        sourceTitle = "Discover"
        Task {
            do {
                let batch = try await source.nextBatch(min: 10)
                guard infinite === source else { return }
                queue = batch
                load(0)
            } catch { self.error = error.localizedDescription }
        }
    }

    func playNext(_ track: Track) {
        let at = (index ?? -1) + 1
        queue.insert(track, at: min(at, queue.count))
        if index == nil { load(0) }
    }

    func moveUpNext(from: IndexSet, to: Int) {
        guard let i = index else { return }
        let offset = i + 1
        queue.move(fromOffsets: IndexSet(from.map { $0 + offset }), toOffset: to + offset)
    }

    func removeUpNext(at offsets: IndexSet) {
        guard let i = index else { return }
        queue.remove(atOffsets: IndexSet(offsets.map { $0 + i + 1 }))
    }

    func jump(toUpNext offset: Int) {
        guard let i = index else { return }
        load(i + 1 + offset)
    }

    // MARK: transport

    func toggle() {
        if isPlaying { avPlayer.pause() } else { avPlayer.play() }
        isPlaying.toggle()
        updateNowPlaying()
    }

    func next() {
        guard let i = index else { return }
        if i + 1 < queue.count { load(i + 1) }
        else if repeatMode == .all, !queue.isEmpty { load(0) }
    }

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
    }

    /// Re-requests the current track's manifest after a quality change, keeping the position.
    func reloadQuality() {
        guard let i = index else { return }
        load(i, resumeAt: position)
    }

    // MARK: internals

    private func load(_ i: Int, resumeAt: Double = 0) {
        guard queue.indices.contains(i) else { return }
        index = i
        let track = queue[i]
        position = resumeAt
        duration = Double(track.durationSecs)
        listened = 0
        scrobbled = false
        startedAt = Date()
        artwork = nil
        loadTask?.cancel()
        loadTask = Task {
            do {
                let s = try await Torneam.stream(trackId: track.id, quality: Preferences.shared.streamingQuality)
                guard !Task.isCancelled, current?.id == track.id, let url = URL(string: s.url) else { return }
                stream = s
                let item = AVPlayerItem(url: url)
                avPlayer.replaceCurrentItem(with: item)
                if resumeAt > 0 { await avPlayer.seek(to: CMTime(seconds: resumeAt, preferredTimescale: 600)) }
                avPlayer.play()
                isPlaying = true
                updateNowPlaying()
                await loadArtwork(track)
                nowPlayingScrobble(track)
            } catch {
                self.error = error.localizedDescription
            }
        }
        refillIfNeeded()
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

    private func trackEnded() {
        if repeatMode == .one { seek(to: 0); avPlayer.play(); return }
        guard let i = index else { return }
        if i + 1 < queue.count { load(i + 1) }
        else if repeatMode == .all, !queue.isEmpty { load(0) }
        else { isPlaying = false; updateNowPlaying() }
    }

    private func refillIfNeeded() {
        guard let source = infinite, let i = index, queue.count - i <= 3 else { return }
        Task {
            guard let more = try? await source.nextBatch(min: 10), infinite === source else { return }
            queue.append(contentsOf: more)
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
