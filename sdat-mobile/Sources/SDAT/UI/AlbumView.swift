import SwiftUI
import CoreImage
import UIKit

struct AlbumView: View {
    let id: String
    @Environment(Recents.self) private var recents
    @Environment(Player.self) private var player
    @State private var detail: AlbumDetail?
    @State private var offline: [Track]?
    @State private var error: String?

    var body: some View {
        Group {
            if let d = detail {
                DetailPage(
                    title: d.album.title,
                    subtitle: [d.album.artistLine, d.album.releaseDate.map { String($0.prefix(4)) }].compactMap { $0 }.joined(separator: " · "),
                    subtitleRoute: d.album.artists.first.map { Route.artist($0.id) },
                    cover: d.album.coverUrlLarge ?? d.album.coverUrl,
                    mosaic: [],
                    tracks: d.tracks,
                    loadingMore: false,
                    footer: Fmt.summary(count: d.album.trackCount, secs: d.album.durationSecs),
                    favourite: (.album, d.album.id),
                    numbered: true,
                    ownPlaylistId: nil,
                    onRemoved: { _ in }
                )
                .onAppear {
                    recents.add(.init(kind: .album, id: d.album.id, title: d.album.title,
                                      subtitle: d.album.artistLine, coverUrl: d.album.coverUrl))
                }
            } else if let t = offline, let a = t.first?.album {
                let info = Downloads.shared.albumInfo(id)
                DetailPage(
                    title: a.title,
                    subtitle: [info.artist ?? t.first?.artistLine, info.year].compactMap { $0 }.joined(separator: " · "),
                    subtitleRoute: nil,
                    cover: a.coverUrlLarge ?? a.coverUrl,
                    mosaic: [],
                    tracks: t,
                    loadingMore: false,
                    footer: "Offline – showing downloaded songs only\n" + Fmt.summary(count: UInt32(t.count), secs: t.reduce(0) { $0 + $1.durationSecs }),
                    favourite: nil,
                    numbered: false,
                    ownPlaylistId: nil,
                    onRemoved: { _ in }
                )
            } else if let error {
                ContentUnavailableView {
                    Label("Couldn't load", systemImage: "wifi.exclamationmark")
                } description: { Text(error) } actions: {
                    Button("Retry") { self.error = nil; Task { await load() } }.buttonStyle(.glass)
                }
            } else {
                DetailSkeleton()
            }
        }
        .task { await load() }
    }

    private func load() async {
        guard detail == nil else { return }
        do {
            let d = try await withDeadline(10) { try await album(id: id) }
            detail = d; offline = nil
            Downloads.shared.note(d)
        } catch {
            let local = Downloads.shared.tracks(inAlbum: id)
            if !local.isEmpty { offline = local } else { self.error = friendlyMessage(error) }
        }
    }
}

struct DeadlineError: LocalizedError { var errorDescription: String? { "The request timed out." } }

/// Returns `op`'s result, or throws after `seconds` without waiting for `op` to finish
/// (a hung network call can't keep a page on its skeleton).
@MainActor
func withDeadline<T: Sendable>(_ seconds: Double, _ op: @escaping @Sendable () async throws -> T) async throws -> T {
    try await withCheckedThrowingContinuation { (c: CheckedContinuation<T, Error>) in
        var done = false
        func finish(_ r: Result<T, Error>) { guard !done else { return }; done = true; c.resume(with: r) }
        Task { @MainActor in
            do { let v = try await op(); finish(.success(v)) } catch { finish(.failure(error)) }
        }
        Task { @MainActor in
            try? await Task.sleep(for: .seconds(seconds)); finish(.failure(DeadlineError()))
        }
    }
}

/// Header + first page render immediately; remaining pages stream in behind.
struct PlaylistView: View {
    let id: String
    @Environment(Recents.self) private var recents
    @State private var pl: Playlist?
    @State private var tracks: [Track] = []
    @State private var loadingMore = true
    @State private var error: String?
    @State private var own = OwnPlaylists.shared

    var body: some View {
        Group {
            if let pl {
                DetailPage(
                    title: pl.title,
                    subtitle: own.contains(pl.id) ? "My playlist" : [pl.creator, pl.isMix ? "Mix" : "Playlist"].compactMap { $0 }.joined(separator: " · "),
                    subtitleRoute: nil,
                    cover: pl.coverUrlLarge ?? pl.coverUrl,
                    mosaic: mosaicCovers,
                    tracks: tracks,
                    loadingMore: loadingMore,
                    footer: [Fmt.summary(count: pl.trackCount, secs: pl.durationSecs), Fmt.updated(pl.updatedAt)].compactMap { $0 }.joined(separator: "\n"),
                    favourite: own.contains(pl.id) ? nil : (pl.isMix ? .mix : .playlist, pl.id),
                    numbered: false,
                    ownPlaylistId: own.contains(pl.id) ? pl.id : nil,
                    onRemoved: { t in if let i = tracks.firstIndex(where: { $0.id == t.id }) { tracks.remove(at: i) } }
                )
            } else if let error {
                ContentUnavailableView("Couldn't load", systemImage: "wifi.exclamationmark", description: Text(error))
            } else {
                DetailSkeleton()
            }
        }
        .task { await load() }
    }

    private var mosaicCovers: [String] {
        var seen = Set<String>()
        let c = tracks.compactMap { $0.album?.coverUrl }.filter { seen.insert($0).inserted }.prefix(4)
        return c.count == 4 ? Array(c) : []
    }

    private func load() async {
        guard pl == nil else { return }
        let pager = playlistTracks(id: id)
        async let header = withDeadline(12) { try await playlist(id: id) }
        async let first = pager.nextPage()
        do {
            let p = try await header
            pl = p
            recents.add(.init(kind: p.isMix ? .mix : .playlist, id: p.id, title: p.title, subtitle: p.creator ?? "", coverUrl: p.coverUrl))
            tracks = (try? await first) ?? []
        } catch { self.error = friendlyMessage(error); return }
        while !Task.isCancelled, tracks.count < 5000 {
            guard let page = try? await pager.nextPage(), !page.isEmpty else { break }
            tracks += page
        }
        loadingMore = false
    }
}

/// Album / playlist page per the dark design with a backdrop derived from the cover.
struct DetailPage: View {
    let title: String
    let subtitle: String
    let subtitleRoute: Route?
    let cover: String?
    let mosaic: [String]
    let tracks: [Track]
    let loadingMore: Bool
    let footer: String
    let favourite: (FavouriteKind, String)?
    let numbered: Bool
    let ownPlaylistId: String?
    let onRemoved: (Track) -> Void
    @Environment(Player.self) private var player
    @State private var isFav = false

    var body: some View {
        List {
            VStack(spacing: 12) {
                artwork.frame(maxWidth: 270).shadow(color: .black.opacity(0.5), radius: 24, y: 12)
                Text(title).font(.title2.bold()).multilineTextAlignment(.center).padding(.top, 6)
                if let subtitleRoute {
                    Button(subtitle) { Chrome.shared.push(subtitleRoute) }.buttonStyle(.plain).foregroundStyle(.blue)
                } else {
                    Text(subtitle).foregroundStyle(.secondary)
                }
                PlayShuffleButtons(
                    play: { if player.shuffle { player.setShuffle(false) }; player.play(tracks, from: title) },
                    shuffle: { player.playShuffled(tracks, from: title) }
                )
                .disabled(tracks.isEmpty)
                .padding(.top, 6)
            }
            .frame(maxWidth: .infinity)
            .listRowSeparator(.hidden)
            .listRowBackground(Color.clear)

            TrackList(tracks: tracks, source: title, showCover: !numbered, numbered: numbered,
                      ownPlaylistId: ownPlaylistId, onRemoved: onRemoved)
                .listRowBackground(Color.clear)

            if loadingMore { ProgressView().frame(maxWidth: .infinity).listRowBackground(Color.clear).listRowSeparator(.hidden) }
            Text(footer).font(.footnote).foregroundStyle(.secondary)
                .listRowSeparator(.hidden).listRowBackground(Color.clear)
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
        .background { CoverBackdrop(url: cover) }
        .scrollEdgeEffectStyle(.soft, for: .all)
        .toolbar {
            if let favourite {
                Button { toggleFav(favourite) } label: { Image(systemName: isFav ? "checkmark" : "plus") }
            }
            Menu {
                Button("Play Next", systemImage: "text.line.first.and.arrowtriangle.forward") {
                    if !Live.shared.userPlayNext(tracks) { for t in tracks.reversed() { player.playNext(t) } }
                }
                Button("Add to Queue", systemImage: "text.line.last.and.arrowtriangle.forward") {
                    if !Live.shared.userEnqueue(tracks) { for t in tracks { player.addToQueue(t) } }
                }
                DownloadButton(tracks: tracks)
                if let favourite {
                    Button(isFav ? "Remove from Collection" : "Add to Collection", systemImage: isFav ? "star.slash" : "star") { toggleFav(favourite) }
                }
            } label: { Image(systemName: "ellipsis").foregroundStyle(.white) }
            .tint(.white)
        }
        .task {
            guard let favourite else { return }
            isFav = (try? await isFavourite(kind: favourite.0, id: favourite.1)) ?? false
        }
    }

    @ViewBuilder private var artwork: some View {
        if mosaic.count == 4 {
            Grid(horizontalSpacing: 0, verticalSpacing: 0) {
                GridRow { Cover(url: mosaic[0], radius: 0); Cover(url: mosaic[1], radius: 0) }
                GridRow { Cover(url: mosaic[2], radius: 0); Cover(url: mosaic[3], radius: 0) }
            }
            .clipShape(.rect(cornerRadius: 14))
        } else {
            Cover(url: cover, radius: 14)
        }
    }

    private func toggleFav(_ f: (FavouriteKind, String)) {
        let target = !isFav
        isFav = target
        Task {
            do { try await setFavourite(kind: f.0, id: f.1, favourite: target) } catch { isFav = !target; Chrome.shared.flash(friendlyMessage(error)) }
        }
    }
}

/// The cover itself, zoomed in and blurred into colour fields that turn and breathe with the
/// track's tempo while `animated`. `light` washes it lighter instead of darker.
struct CoverBackdrop: View {
    let url: String?
    var animated = false
    var bpm: Double?
    var light = false
    @State private var art: UIImage?
    // Phase is counted in beats and only advances while animated, so pausing or a new tempo
    // changes the speed instead of making the backdrop jump.
    @State private var beats: Double = 0
    @State private var run: (start: Date, beatsPerSecond: Double)?

    var body: some View {
        ZStack {
            Color.black
            if let art {
                GeometryReader { g in
                    // Well past the screen diagonal: only the cover's centre shows, never a corner.
                    let side = hypot(g.size.width, g.size.height) * 2.2
                    TimelineView(.animation(minimumInterval: 1 / 30, paused: !animated)) { ctx in
                        let b = beats + (run.map { max(0, ctx.date.timeIntervalSince($0.start)) * $0.beatsPerSecond } ?? 0)
                        // One slow breath per 4-beat bar: TIDAL gives the tempo but not where the beats fall.
                        let breath = 0.5 - 0.5 * cos(b * .pi / 2)
                        ZStack {
                            Image(uiImage: art).resizable().frame(width: side, height: side)
                                .rotationEffect(.degrees(b * 4.5))
                            Image(uiImage: art).resizable().frame(width: side * 0.9, height: side * 0.9)
                                .rotationEffect(.degrees(180 - b * 7))
                                .offset(x: sin(b * 0.18) * 40, y: cos(b * 0.15) * 60)
                                .opacity(0.6)
                        }
                        .scaleEffect(1 + 0.06 * breath)
                        .frame(width: g.size.width, height: g.size.height)
                        .clipped()
                        .blur(radius: 50, opaque: true)
                        .overlay(light ? Color.white.opacity(0.14 + 0.05 * breath) : Color.black.opacity(0.32 - 0.07 * breath))
                    }
                }
                .transition(.opacity)
            }
            LinearGradient(colors: [.clear, .black.opacity(light ? 0.35 : 0.55)], startPoint: .center, endPoint: .bottom)
        }
        .ignoresSafeArea()
        .animation(.easeInOut(duration: 0.8), value: art)
        .onChange(of: animated, initial: true) { fold() }
        .onChange(of: bpm) { fold() }
        .task(id: url) {
            // The old art stays up while the new one loads, so covers crossfade instead of flashing black.
            var next: UIImage?
            if let url, let img = await ImageStore.shared.image(url) {
                next = await Task.detached { BackdropArt.render(img) }.value
            }
            // A load for the previous cover can finish after this task was replaced.
            if !Task.isCancelled { art = next }
        }
    }

    /// Clamped so very fast or very slow tracks still move calmly; unknown tempo moves like 100 BPM.
    private static func beatsPerSecond(_ bpm: Double?) -> Double { min(max(bpm ?? 100, 60), 160) / 60 }

    /// Banks the beats played so far at the tempo they ran at, then restarts the clock if still animated.
    private func fold() {
        if let run { beats += Date.now.timeIntervalSince(run.start) * run.beatsPerSecond }
        run = animated ? (Date.now, Self.beatsPerSecond(bpm)) : nil
    }
}

enum BackdropArt {
    /// A 32 px blurred copy: scaled up it reads as smooth colour and is cheap to redraw every frame.
    static func render(_ image: UIImage) -> UIImage? {
        guard let cg = image.cgImage else { return nil }
        let ci = CIImage(cgImage: cg)
        let small = ci.applyingFilter("CILanczosScaleTransform", parameters: [
            kCIInputScaleKey: 32 / max(ci.extent.width, ci.extent.height),
            kCIInputAspectRatioKey: 1,
        ])
        let out = small.clampedToExtent()
            .applyingGaussianBlur(sigma: 2)
            .applyingFilter("CIColorControls", parameters: [kCIInputSaturationKey: 1.8])
            .cropped(to: small.extent)
        guard let res = CIContext().createCGImage(out, from: out.extent) else { return nil }
        return UIImage(cgImage: res)
    }
}

/// In-memory artist data so artist pages open instantly.
@MainActor enum ArtistCache {
    static var overviews: [String: ArtistOverview] = [:]
    static var hints: [String: (name: String, picture: String?)] = [:]
    static func hint(_ id: String, _ name: String, _ picture: String?) { hints[id] = (name, picture) }
}

/// Opens immediately (header from the cached overview or the tapped row), then loads in the background.
struct ArtistView: View {
    let id: String
    @Environment(Player.self) private var player
    @State private var overview: ArtistOverview?
    @State private var error: String?

    var body: some View {
        let a = overview?.artist
        let name = a?.name ?? ArtistCache.hints[id]?.name ?? ""
        let pic = [a?.pictureUrl, ArtistCache.hints[id]?.picture].compactMap { $0 }.first { !$0.isEmpty }
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                VStack(spacing: 10) {
                    Cover(url: pic, size: 180, circle: true)
                    if name.isEmpty { Skeleton(width: 160, height: 28) } else { Text(name).font(.title.bold()).multilineTextAlignment(.center) }
                    FollowButton(artistId: id)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)

                if let error, overview == nil {
                    ErrorBanner(text: error).padding(16)
                    Button("Retry") { self.error = nil; Task { await load() } }.buttonStyle(.glass).frame(maxWidth: .infinity)
                } else if let o = overview {
                    if !o.topTracks.isEmpty {
                        header("Top Tracks")
                        PlayShuffleButtons(
                            play: { player.play(o.topTracks, from: o.artist.name) },
                            shuffle: { player.playShuffled(o.topTracks, from: o.artist.name) },
                            compact: true
                        )
                        .padding(.horizontal, 16).padding(.bottom, 8)
                        VStack(spacing: 0) {
                            ForEach(Array(o.topTracks.enumerated()), id: \.offset) { i, t in
                                Button { player.play(o.topTracks, startAt: i, from: o.artist.name) } label: {
                                    TrackRow(track: t, coverSize: 56).frame(minHeight: 72)
                                }
                                .buttonStyle(.plain)
                                .trackMenu(t, ownPlaylistId: nil, onRemoved: {})
                                if i < o.topTracks.count - 1 { Divider().padding(.leading, 68) }
                            }
                        }
                        .padding(.horizontal, 16)
                    }
                    if !o.albums.isEmpty {
                        header("Albums")
                        AlbumShelf(albums: o.albums).contentMargins(.horizontal, 16, for: .scrollContent)
                    }
                    if !o.artist.bio.isEmpty {
                        header("About")
                        BioText(text: o.artist.bio).padding(.horizontal, 16)
                    }
                } else {
                    ForEach(0..<6, id: \.self) { _ in RowSkeleton().padding(.horizontal, 16) }
                }
            }
        }
        .scrollEdgeEffectStyle(.soft, for: .all)
        .background { CoverBackdrop(url: pic).ignoresSafeArea() }
        .task { await load() }
    }

    private func header(_ t: String) -> some View {
        Text(t).font(.title2.bold()).padding(.horizontal, 16).padding(.top, 24).padding(.bottom, 10)
    }

    private func load() async {
        if let o = ArtistCache.overviews[id] { overview = o }
        do {
            let o = try await artistOverview(id: id)
            ArtistCache.overviews[id] = o
            overview = o
        } catch {
            if overview == nil { self.error = friendlyMessage(error) }
        }
    }
}

/// Follow / Following: optimistic, persisted with setFavourite(.artist), reverts on error.
struct FollowButton: View {
    let artistId: String
    @State private var following: Bool?
    @State private var busy = false

    var body: some View {
        Group {
            if following == true {
                Button(action: toggle) { Label("Following", systemImage: "checkmark").font(.subheadline.weight(.semibold)).padding(.horizontal, 6) }
                    .buttonStyle(.glassProminent)
            } else {
                Button(action: toggle) { Label("Follow", systemImage: "plus").font(.subheadline.weight(.semibold)).padding(.horizontal, 6) }
                    .buttonStyle(.glass)
            }
        }
        .disabled(following == nil || busy)
        .animation(.snappy, value: following)
        .sensoryFeedback(.success, trigger: following == true)
        .task { following = (try? await isFavourite(kind: .artist, id: artistId)) ?? false }
    }

    private func toggle() {
        let target = !(following ?? false)
        following = target
        busy = true
        Task {
            do {
                try await setFavourite(kind: .artist, id: artistId, favourite: target)
                Chrome.shared.flash(target ? "Following" : "Unfollowed")
            } catch {
                following = !target
                Chrome.shared.flash("Couldn't update: \(friendlyMessage(error))")
            }
            busy = false
        }
    }
}

/// Horizontal carousel of albums.
struct AlbumShelf: View {
    let albums: [Album]
    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            LazyHStack(alignment: .top, spacing: 12) {
                ForEach(albums, id: \.id) { al in
                    Button { Chrome.shared.push(.album(al.id)) } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Cover(url: al.coverUrl, size: 140, radius: 10)
                            Text(al.title).font(.subheadline.weight(.semibold)).lineLimit(1)
                            Text(al.releaseDate.map { String($0.prefix(4)) } ?? al.artistLine).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                        .frame(width: 140, alignment: .leading)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }
}

private struct BioText: View {
    let text: String
    @State private var expanded = false
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(text).font(.subheadline).foregroundStyle(.secondary).lineLimit(expanded ? nil : 6)
            Button(expanded ? "Less" : "More") { withAnimation { expanded.toggle() } }.font(.subheadline.weight(.semibold))
        }
    }
}
