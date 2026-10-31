import SwiftUI

struct CollectionView: View {
    @Environment(Recents.self) private var recents

    var body: some View {
        List {
            Section {
                NavigationLink { PagedTracks(title: "Tracks", pager: collectionTracks()) } label: {
                    Label("Tracks", systemImage: "music.note")
                }
                NavigationLink { PagedAlbums(pager: collectionAlbums()) } label: {
                    Label("Albums", systemImage: "square.stack")
                }
                NavigationLink { PagedArtists(pager: collectionArtists()) } label: {
                    Label("Artists", systemImage: "music.microphone")
                }
                NavigationLink { PagedPlaylists(pager: collectionPlaylists()) } label: {
                    Label("Playlists", systemImage: "music.note.list")
                }
            }
            if !recents.items.isEmpty {
                Section("Recents") {
                    ForEach(recents.items) { item in
                        NavigationLink(value: item.kind == .album ? Route.album(item.id) : Route.playlist(item.id)) {
                            MediaRow(title: item.title, subtitle: item.subtitle, cover: item.coverUrl)
                        }
                    }
                }
            }
        }
        .navigationTitle("My Collection")
        .profileToolbar()
    }
}

/// Generic infinite list over a tonearm pager.
struct Paged<Item, Row: View>: View {
    let title: String
    let next: () async throws -> [Item]?
    @ViewBuilder let row: ([Item], Int) -> Row
    @State private var items: [Item] = []
    @State private var done = false
    @State private var loading = false

    var body: some View {
        List {
            ForEach(items.indices, id: \.self) { i in
                row(items, i).onAppear { if i == items.count - 1 { Task { await more() } } }
            }
            if !done { ProgressView().frame(maxWidth: .infinity).task { await more() } }
        }
        .listStyle(.plain)
        .navigationTitle(title)
    }

    private func more() async {
        guard !loading, !done else { return }
        loading = true
        defer { loading = false }
        if let page = try? await next(), !page.isEmpty { items += page } else { done = true }
    }
}

struct PagedTracks: View {
    let title: String
    let pager: TrackPager
    @Environment(Player.self) private var player

    var body: some View {
        Paged(title: title, next: pager.nextPage) { items, i in
            Button { player.play(items, startAt: i, from: title) } label: { TrackRow(track: items[i]) }
                .buttonStyle(.plain)
        }
    }
}

struct PagedAlbums: View {
    let pager: AlbumPager
    var body: some View {
        Paged(title: "Albums", next: pager.nextPage) { items, i in
            NavigationLink(value: Route.album(items[i].id)) {
                MediaRow(title: items[i].title, subtitle: items[i].artistLine, cover: items[i].coverUrl)
            }
        }
    }
}

struct PagedArtists: View {
    let pager: ArtistPager
    var body: some View {
        Paged(title: "Artists", next: pager.nextPage) { items, i in
            NavigationLink(value: Route.artist(items[i].id)) {
                HStack { Cover(url: items[i].pictureUrl, size: 44, radius: 22); Text(items[i].name) }
            }
        }
    }
}

struct PagedPlaylists: View {
    let pager: PlaylistPager
    var body: some View {
        Paged(title: "Playlists", next: pager.nextPage) { items, i in
            NavigationLink(value: Route.playlist(items[i].id)) {
                MediaRow(title: items[i].title, subtitle: items[i].creator ?? "", cover: items[i].coverUrl)
            }
        }
    }
}
