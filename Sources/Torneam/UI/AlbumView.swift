import SwiftUI

struct AlbumView: View {
    let id: String
    @Environment(Recents.self) private var recents

    var body: some View {
        Loader(load: { try await album(id: id) }) { detail in
            CollectionPage(
                title: detail.album.title,
                subtitle: detail.album.artistLine,
                artistId: detail.album.artists.first?.id,
                header: AnyView(Cover(url: detail.album.coverUrlLarge ?? detail.album.coverUrl, radius: 12)),
                tracks: detail.tracks,
                footer: [detail.album.releaseDate, Fmt.summary(count: detail.album.trackCount, secs: detail.album.durationSecs)]
                    .compactMap { $0 }.joined(separator: "\n"),
                favourite: (.album, detail.album.id),
                showCovers: false
            )
            .onAppear {
                recents.add(.init(kind: .album, id: detail.album.id, title: detail.album.title,
                                  subtitle: detail.album.artistLine, coverUrl: detail.album.coverUrl))
            }
        }
    }
}

struct PlaylistView: View {
    let id: String
    @Environment(Recents.self) private var recents

    var body: some View {
        Loader(load: { try await loadAll() }) { (pl, tracks) in
            CollectionPage(
                title: pl.title,
                subtitle: pl.creator ?? "",
                artistId: nil,
                header: AnyView(Mosaic(tracks: tracks, fallback: pl.coverUrlLarge)),
                tracks: tracks,
                footer: [Fmt.summary(count: pl.trackCount, secs: pl.durationSecs), Fmt.updated(pl.updatedAt)]
                    .compactMap { $0 }.joined(separator: "\n"),
                favourite: (pl.isMix ? .mix : .playlist, pl.id),
                showCovers: true
            )
            .onAppear {
                recents.add(.init(kind: pl.isMix ? .mix : .playlist, id: pl.id, title: pl.title,
                                  subtitle: pl.creator ?? "", coverUrl: pl.coverUrl))
            }
        }
    }

    private func loadAll() async throws -> (Playlist, [Track]) {
        let pl = try await playlist(id: id)
        let pager = playlistTracks(id: id)
        var all: [Track] = []
        while let page = try await pager.nextPage(), !page.isEmpty, all.count < 2000 { all += page }
        return (pl, all)
    }
}

/// 2x2 cover mosaic from the first four distinct albums.
struct Mosaic: View {
    let tracks: [Track]
    let fallback: String?

    var body: some View {
        var seen = Set<String>()
        let covers = tracks.compactMap { $0.album?.coverUrlLarge ?? $0.album?.coverUrl }
            .filter { seen.insert($0).inserted }
            .prefix(4)
        if covers.count < 4 {
            Cover(url: fallback ?? covers.first, radius: 12)
        } else {
            let c = Array(covers)
            Grid(horizontalSpacing: 0, verticalSpacing: 0) {
                GridRow { Cover(url: c[0], radius: 0); Cover(url: c[1], radius: 0) }
                GridRow { Cover(url: c[2], radius: 0); Cover(url: c[3], radius: 0) }
            }
            .clipShape(.rect(cornerRadius: 12))
        }
    }
}

/// Shared layout for Album and Playlist.
struct CollectionPage: View {
    let title: String
    let subtitle: String
    let artistId: String?
    let header: AnyView
    let tracks: [Track]
    let footer: String
    let favourite: (FavouriteKind, String)
    let showCovers: Bool
    @Environment(Player.self) private var player
    @State private var isFav = false

    var body: some View {
        List {
            VStack(spacing: 10) {
                header.frame(maxWidth: 260)
                Text(title).font(.title2.bold()).multilineTextAlignment(.center)
                if let artistId {
                    NavigationLink(value: Route.artist(artistId)) { Text(subtitle) }
                        .buttonStyle(.plain).foregroundStyle(.tint)
                } else {
                    Text(subtitle).foregroundStyle(.secondary)
                }
                HStack(spacing: 12) {
                    Button { player.shuffle = false; player.play(tracks, from: title) } label: {
                        Label("Play", systemImage: "play.fill").frame(maxWidth: .infinity)
                    }
                    Button { player.shuffle = true; player.play(tracks, startAt: tracks.indices.randomElement() ?? 0, from: title) } label: {
                        Label("Shuffle", systemImage: "shuffle").frame(maxWidth: .infinity)
                    }
                }
                .buttonStyle(.glass)
                .controlSize(.large)
            }
            .frame(maxWidth: .infinity)
            .listRowSeparator(.hidden)

            TrackList(tracks: tracks, source: title, showCover: showCovers)

            Text(footer).font(.footnote).foregroundStyle(.secondary).listRowSeparator(.hidden)
        }
        .listStyle(.plain)
        .toolbar {
            Button { toggleFav() } label: { Image(systemName: isFav ? "star.fill" : "star") }
        }
        .task { isFav = (try? await isFavourite(kind: favourite.0, id: favourite.1)) ?? false }
    }

    private func toggleFav() {
        let target = !isFav
        isFav = target
        Task {
            do { try await setFavourite(kind: favourite.0, id: favourite.1, favourite: target) } catch { isFav = !target }
        }
    }
}

struct ArtistView: View {
    let id: String

    var body: some View {
        Loader(load: { try await artist(id: id) }) { a in
            List {
                VStack(spacing: 10) {
                    Cover(url: a.pictureUrl, size: 180, radius: 90)
                    Text(a.name).font(.title.bold())
                }
                .frame(maxWidth: .infinity)
                .listRowSeparator(.hidden)
                if !a.bio.isEmpty {
                    Text(a.bio).font(.subheadline).foregroundStyle(.secondary).lineLimit(6)
                }
            }
            .listStyle(.plain)
        }
    }
}
