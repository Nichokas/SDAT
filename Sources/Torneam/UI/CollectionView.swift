import SwiftUI

struct CollectionView: View {
    @Environment(Recents.self) private var local
    @State private var recents: [RecentItem] = []
    @State private var recentsError: String?

    private let cols = [GridItem(.flexible(), spacing: 14), GridItem(.flexible(), spacing: 14)]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                VStack(spacing: 0) {
                    row("Playlists", "music.note.list", .collectionPlaylists)
                    Divider().padding(.leading, 52)
                    row("Albums", "square.stack", .collectionAlbums)
                    Divider().padding(.leading, 52)
                    row("Tracks", "music.note", .collectionTracks)
                    Divider().padding(.leading, 52)
                    row("Artists", "music.microphone", .collectionArtists)
                }
                .padding(.horizontal, 16)

                Text("Recent activity").font(.title2.bold())
                    .padding(.horizontal, 16).padding(.top, 28).padding(.bottom, 12)
                if let recentsError, tiles.isEmpty { ErrorBanner(text: recentsError).padding(.horizontal, 16) }
                LazyVGrid(columns: cols, spacing: 18) {
                    ForEach(tiles, id: \.self) { t in
                        Button { Chrome.shared.push(t.route) } label: { RecentTile(item: t) }
                            .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 16)
            }
        }
        .collapsesChrome()
        .navigationTitle("My Collection")
        .scrollEdgeEffectStyle(.soft, for: .all)
        .profileToolbar()
        .refreshable { await load() }
        .task { if recents.isEmpty { await load() } }
    }

    struct Tile: Hashable {
        var title, subtitle: String
        var cover: String?
        var round: Bool
        var route: Route
    }

    /// TIDAL listening history, falling back to what was opened on this device.
    private var tiles: [Tile] {
        if !recents.isEmpty {
            return recents.map { r in
                let route: Route = switch r.kind {
                case .album: .album(r.id)
                case .artist: .artist(r.id)
                case .playlist, .mix: .playlist(r.id)
                }
                return Tile(title: r.title, subtitle: r.subtitle, cover: r.coverUrl, round: r.kind == .artist, route: route)
            }
        }
        return local.items.map {
            Tile(title: $0.title, subtitle: $0.subtitle, cover: $0.coverUrl, round: false,
                 route: $0.kind == .album ? .album($0.id) : .playlist($0.id))
        }
    }

    private func load() async {
        do { recents = try await Torneam.recents(); recentsError = nil } catch { recentsError = error.localizedDescription }
    }

    private func row(_ title: String, _ icon: String, _ r: Route) -> some View {
        Button { Chrome.shared.push(r) } label: {
            HStack(spacing: 16) {
                Image(systemName: icon).font(.title3).foregroundStyle(.blue).frame(width: 36)
                Text(title).font(.title3)
                Spacer()
                Image(systemName: "chevron.right").font(.footnote.weight(.semibold)).foregroundStyle(.tertiary)
            }
            .frame(height: 52)
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
    }
}

private struct RecentTile: View {
    let item: CollectionView.Tile
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Cover(url: item.cover, radius: 10, circle: item.round)
            Text(item.title).font(.subheadline.weight(.semibold)).lineLimit(1)
            if !item.subtitle.isEmpty {
                Text(item.subtitle).font(.caption).foregroundStyle(.secondary).lineLimit(1)
            }
        }
    }
}

/// Generic infinite list over a tonearm pager; errors are shown, not swallowed.
struct Paged<Item, Row: View>: View {
    let title: String
    let next: () async throws -> [Item]?
    var onLoaded: (([Item]) -> Void)? = nil
    var header: AnyView? = nil
    @ViewBuilder let row: ([Item], Int) -> Row
    @State private var items: [Item] = []
    @State private var done = false
    @State private var loading = false
    @State private var error: String?

    var body: some View {
        List {
            if let header { header.listRowSeparator(.hidden) }
            ForEach(items.indices, id: \.self) { i in
                row(items, i).onAppear { if i >= items.count - 5 { Task { await more() } } }
            }
            if let error {
                VStack(alignment: .leading, spacing: 8) {
                    ErrorBanner(text: error)
                    Button("Retry") { self.error = nil; Task { await more() } }.buttonStyle(.glass)
                }
            } else if !done {
                ProgressView().frame(maxWidth: .infinity).task { await more() }
            } else if items.isEmpty {
                ContentUnavailableView("Nothing here yet", systemImage: "tray")
            }
        }
        .listStyle(.plain)
        .navigationTitle(title)
        .scrollEdgeEffectStyle(.soft, for: .all)
    }

    private func more() async {
        guard !loading, !done, error == nil else { return }
        loading = true
        defer { loading = false }
        do {
            if let page = try await next(), !page.isEmpty { items += page; onLoaded?(items) } else { done = true }
        } catch { self.error = error.localizedDescription }
    }
}

struct PagedTracks: View {
    let title: String
    let pager: TrackPager
    var favourites = false
    @Environment(Player.self) private var player
    @State private var loaded: [Track] = []

    var body: some View {
        Paged(title: title, next: pager.nextPage, onLoaded: { all in
            loaded = all
            if favourites { Favs.shared.mark(all.map(\.id), true) }
        }, header: AnyView(PlayShuffleButtons(
            play: { player.shuffle ? player.setShuffle(false) : (); player.play(loaded, from: title) },
            shuffle: { player.playShuffled(loaded, from: title) }
        ).disabled(loaded.isEmpty))) { items, i in
            Button { player.play(items, startAt: i, from: title) } label: { TrackRow(track: items[i]) }
                .buttonStyle(.plain)
                .trackMenu(items[i])
        }
    }
}

struct PagedAlbums: View {
    let title: String
    let pager: AlbumPager
    var body: some View {
        Paged(title: title, next: pager.nextPage) { items, i in
            NavigationLink(value: Route.album(items[i].id)) {
                MediaRow(title: items[i].title, subtitle: items[i].artistLine, cover: items[i].coverUrl)
            }
        }
    }
}

struct PagedArtists: View {
    let title: String
    let pager: ArtistPager
    var body: some View {
        Paged(title: title, next: pager.nextPage) { items, i in
            NavigationLink(value: Route.artist(items[i].id)) {
                MediaRow(title: items[i].name, subtitle: "", cover: items[i].pictureUrl, round: true)
            }
        }
    }
}

struct PagedPlaylists: View {
    let title: String
    let pager: PlaylistPager
    var body: some View {
        Paged(title: title, next: pager.nextPage) { items, i in
            NavigationLink(value: Route.playlist(items[i].id)) {
                MediaRow(title: items[i].title, subtitle: items[i].creator ?? "", cover: items[i].coverUrl)
            }
        }
    }
}

/// Play (prominent blue glass) + Shuffle (glass), as in the design.
struct PlayShuffleButtons: View {
    let play: () -> Void
    let shuffle: () -> Void
    var body: some View {
        GlassEffectContainer(spacing: 12) {
            HStack(spacing: 12) {
                Button(action: play) {
                    Label("Play", systemImage: "play.fill").font(.headline).frame(maxWidth: .infinity, minHeight: 34)
                }
                .buttonStyle(.glassProminent)
                .tint(.blue)
                Button(action: shuffle) {
                    Label("Shuffle", systemImage: "shuffle").font(.headline).frame(maxWidth: .infinity, minHeight: 34)
                }
                .buttonStyle(.glass)
            }
            .controlSize(.large)
        }
    }
}
