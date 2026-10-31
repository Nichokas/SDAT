import SwiftUI
import CoreImage
import UIKit

struct AlbumView: View {
    let id: String
    @Environment(Recents.self) private var recents
    @Environment(Player.self) private var player

    var body: some View {
        Loader(load: { try await album(id: id) }) { d in
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
        async let header = playlist(id: id)
        async let first = pager.nextPage()
        do {
            let p = try await header
            pl = p
            recents.add(.init(kind: p.isMix ? .mix : .playlist, id: p.id, title: p.title, subtitle: p.creator ?? "", coverUrl: p.coverUrl))
            tracks = (try? await first) ?? []
        } catch { self.error = error.localizedDescription; return }
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
                    for t in tracks.reversed() { player.playNext(t) }
                }
                Button("Add to Queue", systemImage: "text.line.last.and.arrowtriangle.forward") {
                    for t in tracks { player.addToQueue(t) }
                }
                if let favourite {
                    Button(isFav ? "Remove from Collection" : "Add to Collection", systemImage: isFav ? "star.slash" : "star") { toggleFav(favourite) }
                }
            } label: { Image(systemName: "ellipsis") }
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
            do { try await setFavourite(kind: f.0, id: f.1, favourite: target) } catch { isFav = !target; Chrome.shared.flash(error.localizedDescription) }
        }
    }
}

/// Smooth, dark backdrop with the cover's colours.
struct CoverBackdrop: View {
    let url: String?
    var animated = false
    @State private var colors: [Color] = []

    var body: some View {
        ZStack {
            Color.black
            if colors.count == 4 {
                TimelineView(.animation(minimumInterval: 1 / 30, paused: !animated)) { ctx in
                    let t = animated ? ctx.date.timeIntervalSinceReferenceDate : 0
                    let dx = Float(sin(t * 0.25)) * 0.12, dy = Float(cos(t * 0.2)) * 0.12
                    MeshGradient(width: 3, height: 3, points: [
                        [0, 0], [0.5, 0], [1, 0],
                        [0, 0.5], [0.5 + dx, 0.45 + dy], [1, 0.5],
                        [0, 1], [0.5, 1], [1, 1],
                    ], colors: [
                        colors[0], colors[1], colors[1],
                        colors[2], mix(colors[0], colors[3]), colors[1],
                        colors[3], colors[2], colors[3],
                    ], smoothsColors: true)
                }
                .transition(.opacity)
            }
            LinearGradient(colors: [.black.opacity(0.15), .black.opacity(0.55), .black.opacity(0.9)], startPoint: .top, endPoint: .bottom)
        }
        .ignoresSafeArea()
        .animation(.easeInOut(duration: 0.8), value: colors)
        .task(id: url) {
            guard let url, let img = await ImageStore.shared.image(url) else { return }
            let c = await Task.detached { CoverColors.palette(img) }.value
            colors = c.map { Color(uiColor: $0) }
        }
    }

    private func mix(_ a: Color, _ b: Color) -> Color { a.mix(with: b, by: 0.5) }
}

enum CoverColors {
    /// Four quadrant averages, saturation/brightness clamped so they stay rich but dark.
    static func palette(_ image: UIImage) -> [UIColor] {
        guard let cg = image.cgImage else { return [] }
        let ci = CIImage(cgImage: cg)
        let ctx = CIContext(options: [.workingColorSpace: NSNull()])
        let e = ci.extent
        let rects = [
            CGRect(x: e.minX, y: e.midY, width: e.width / 2, height: e.height / 2),
            CGRect(x: e.midX, y: e.midY, width: e.width / 2, height: e.height / 2),
            CGRect(x: e.minX, y: e.minY, width: e.width / 2, height: e.height / 2),
            CGRect(x: e.midX, y: e.minY, width: e.width / 2, height: e.height / 2),
        ]
        return rects.map { r in
            let f = CIFilter(name: "CIAreaAverage", parameters: [kCIInputImageKey: ci, kCIInputExtentKey: CIVector(cgRect: r)])!
            var px = [UInt8](repeating: 0, count: 4)
            ctx.render(f.outputImage!, toBitmap: &px, rowBytes: 4, bounds: CGRect(x: 0, y: 0, width: 1, height: 1), format: .RGBA8, colorSpace: nil)
            var h: CGFloat = 0, s: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
            UIColor(red: CGFloat(px[0]) / 255, green: CGFloat(px[1]) / 255, blue: CGFloat(px[2]) / 255, alpha: 1)
                .getHue(&h, saturation: &s, brightness: &b, alpha: &a)
            return UIColor(hue: h, saturation: min(max(s * 1.1, 0.25), 0.75), brightness: min(max(b, 0.3), 0.6), alpha: 1)
        }
    }
}

struct ArtistView: View {
    let id: String
    @Environment(Player.self) private var player
    @State private var following: Bool?

    var body: some View {
        Loader(load: { try await artistOverview(id: id) }) { o in
            let a = o.artist
            List {
                VStack(spacing: 10) {
                    Cover(url: a.pictureUrl, size: 180, circle: true)
                    Text(a.name).font(.title.bold())
                    FollowButton(following: following) { toggleFollow(a) }
                }
                .frame(maxWidth: .infinity)
                .listRowSeparator(.hidden)
                .listRowBackground(Color.clear)
                if !o.topTracks.isEmpty {
                    Section("Top Tracks") {
                        TrackList(tracks: o.topTracks, source: a.name).listRowBackground(Color.clear)
                    }
                }
                if !o.albums.isEmpty {
                    Section("Albums") { AlbumShelf(albums: o.albums).listRowBackground(Color.clear) }
                }
                if !a.bio.isEmpty {
                    Section("About") { BioText(text: a.bio).listRowBackground(Color.clear) }
                }
            }
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
            .scrollEdgeEffectStyle(.soft, for: .top)
            .background { CoverBackdrop(url: a.pictureUrl) }
        }
        .task { following = (try? await isFavourite(kind: .artist, id: id)) ?? false }
    }

    private func toggleFollow(_ a: Artist) {
        let target = !(following ?? false)
        following = target
        Task {
            do { try await setFavourite(kind: .artist, id: a.id, favourite: target) } catch {
                following = !target
                Chrome.shared.flash(error.localizedDescription)
            }
        }
    }
}

struct FollowButton: View {
    let following: Bool?
    let action: () -> Void
    var body: some View {
        Group {
            if following == true {
                Button(action: action) { Label("Following", systemImage: "checkmark").font(.subheadline.weight(.semibold)) }
                    .buttonStyle(.glassProminent)
            } else {
                Button(action: action) { Label("Follow", systemImage: "plus").font(.subheadline.weight(.semibold)) }
                    .buttonStyle(.glass)
            }
        }
        .disabled(following == nil)
        .animation(.snappy, value: following)
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
