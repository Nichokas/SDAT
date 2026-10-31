import SwiftUI

struct SearchView: View {
    enum Filter: String, CaseIterable { case top = "Top", tracks = "Tracks", artists = "Artists", albums = "Albums", playlists = "Playlists" }
    @State private var query = ""
    @State private var debounced = ""
    @State private var filter: Filter = .top
    @State private var results: SearchResults?
    @State private var searching = false
    @State private var error: String?
    @State private var topArtist: ArtistOverview?
    @State private var generation = 0

    var body: some View {
        Group {
            if debounced.isEmpty {
                ExploreContent(path: "explore", showTitle: false)
            } else {
                VStack(spacing: 0) {
                    ScrollView(.horizontal, showsIndicators: false) {
                        GlassEffectContainer(spacing: 8) { HStack(spacing: 8) {
                            ForEach(Filter.allCases, id: \.self) { f in Chip(title: f.rawValue, selected: filter == f) { withAnimation(.snappy) { filter = f } } }
                        } }
                        .padding(.horizontal, 16).padding(.vertical, 8)
                    }
                    switch filter {
                    case .top: top
                    case .tracks: PagedTracks(title: "Search", pager: searchTracks(query: debounced)).id(debounced)
                    case .artists: PagedArtists(title: "Search", pager: searchArtists(query: debounced)).id(debounced)
                    case .albums: PagedAlbums(title: "Search", pager: searchAlbums(query: debounced)).id(debounced)
                    case .playlists: PagedPlaylists(title: "Search", pager: searchPlaylists(query: debounced)).id(debounced)
                    }
                }
            }
        }
        .navigationTitle("Explore")
        .profileToolbar()
        .searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always), prompt: "Artists, songs, albums")
        .task(id: query) {
            let q = query.trimmingCharacters(in: .whitespaces)
            if q.isEmpty { debounced = ""; results = nil; topArtist = nil; error = nil; return }
            // One-letter queries return junk; wait for at least two characters.
            guard q.count >= 2 else { return }
            // Re-appearing (tab switch, chrome changes) must not search again.
            if q == debounced, results != nil || error != nil { return }
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled else { return }
            generation += 1
            let gen = generation
            searching = true
            if debounced != q, results == nil { debounced = q }
            do {
                let r = try await search(query: q)
                // Only the latest query may write results (the FFI call can't be cancelled).
                guard !Task.isCancelled, current(q, gen) else { return }
                var artist: ArtistOverview?
                if let a = r.artists.first, a.name.localizedCaseInsensitiveContains(q) || q.localizedCaseInsensitiveContains(a.name) {
                    artist = ArtistCache.overviews[a.id]
                    if artist == nil { artist = try? await artistOverview(id: a.id) }
                    if let artist { ArtistCache.overviews[a.id] = artist }
                    guard !Task.isCancelled, current(q, gen) else { return }
                }
                var t = Transaction(); t.disablesAnimations = true
                withTransaction(t) {
                    debounced = q; results = r; topArtist = artist; error = nil; searching = false
                }
            } catch {
                guard !Task.isCancelled, current(q, gen) else { return }
                debounced = q
                self.error = error.localizedDescription
                searching = false
            }
        }
    }

    private func current(_ q: String, _ gen: Int) -> Bool { gen == generation && query.trimmingCharacters(in: .whitespaces) == q }

    private var top: some View {
        List {
            if let error { ErrorBanner(text: error) }
            if let r = results {
                if r.tracks.isEmpty && r.albums.isEmpty && r.artists.isEmpty && r.playlists.isEmpty {
                    ContentUnavailableView.search(text: debounced)
                }
                if let o = topArtist {
                    Section {
                        Button { ArtistCache.overviews[o.artist.id] = o; Chrome.shared.push(.artist(o.artist.id)) } label: {
                            HStack(spacing: 14) {
                                Cover(url: o.artist.pictureUrl, size: 72, circle: true)
                                VStack(alignment: .leading) {
                                    Text(o.artist.name).font(.title3.bold())
                                    Text("Artist").font(.subheadline).foregroundStyle(.secondary)
                                }
                                Spacer()
                                Image(systemName: "chevron.right").foregroundStyle(.tertiary)
                            }
                        }
                        .buttonStyle(.plain)
                        TrackList(tracks: Array(o.topTracks.prefix(5)), source: o.artist.name)
                        if !o.albums.isEmpty { AlbumShelf(albums: o.albums) }
                    } header: { Text("Top Result") }
                }
                if !r.artists.isEmpty {
                    Section("Artists") {
                        ForEach(r.artists.prefix(4), id: \.id) { a in
                            NavigationLink(value: Route.artist(a.id)) { MediaRow(title: a.name, subtitle: "Artist", cover: a.pictureUrl, round: true) }
                                .onAppear { ArtistCache.hint(a.id, a.name, a.pictureUrl) }
                        }
                    }
                }
                if !r.tracks.isEmpty {
                    Section("Tracks") { TrackList(tracks: r.tracks, source: "Search") }
                }
                if !r.albums.isEmpty {
                    Section("Albums") {
                        ForEach(r.albums, id: \.id) { a in
                            NavigationLink(value: Route.album(a.id)) { MediaRow(title: a.title, subtitle: a.artistLine, cover: a.coverUrl) }
                        }
                    }
                }
                if !r.playlists.isEmpty {
                    Section("Playlists") {
                        ForEach(r.playlists, id: \.id) { p in
                            NavigationLink(value: Route.playlist(p.id)) { MediaRow(title: p.title, subtitle: p.creator ?? "", cover: p.coverUrl) }
                        }
                    }
                }
            }
        }
        .listStyle(.plain)
        .collapsesChrome()
        .overlay(alignment: .top) {
            if searching && results == nil {
                VStack(spacing: 14) { ForEach(0..<8, id: \.self) { _ in RowSkeleton() } }.padding(16)
            }
        }
    }
}

struct ExploreView: View {
    let path: String
    let title: String
    var body: some View {
        ExploreContent(path: path, showTitle: true).navigationTitle(title)
    }
}

/// TIDAL Explore page: chip clouds (Genres, Moods & Activities, Decades...) and media rows.
struct ExploreContent: View {
    let path: String
    let showTitle: Bool
    @State private var page: ExplorePage?
    @State private var error: String?
    @State private var asList: Set<String> = []
    @Environment(Player.self) private var player

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 24) {
                if let error {
                    ErrorBanner(text: error)
                    Button("Retry") { self.error = nil; Task { await load() } }.buttonStyle(.glass)
                }
                if page == nil && error == nil {
                    FlowLayout { ForEach(0..<14, id: \.self) { i in Skeleton(width: CGFloat(60 + (i * 37) % 70), height: 34, radius: 17) } }
                }
                ForEach(Array((page?.sections ?? []).enumerated()), id: \.offset) { _, s in section(s) }
            }
            .padding(16)
        }
        .collapsesChrome()
        .task { if page == nil { await load() } }
    }

    private func load() async {
        do { page = try await explorePage(path: path) } catch { self.error = error.localizedDescription }
    }

    @ViewBuilder
    private func section(_ s: ExploreSection) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            if !s.title.isEmpty {
                HStack {
                    Text(s.title).font(.title3.bold())
                    Spacer()
                    if !s.links.isEmpty {
                        Button(asList.contains(s.title) ? "View as chips" : "View as list") {
                            if asList.contains(s.title) { asList.remove(s.title) } else { asList.insert(s.title) }
                        }
                        .font(.subheadline)
                    }
                }
            }
            if !s.links.isEmpty {
                if asList.contains(s.title) {
                    VStack(spacing: 0) {
                        ForEach(s.links, id: \.path) { l in
                            Button { Chrome.shared.push(.explore(path: l.path, title: l.title)) } label: {
                                HStack { Text(l.title); Spacer(); Image(systemName: "chevron.right").foregroundStyle(.tertiary) }
                                    .frame(height: 44).contentShape(.rect)
                            }
                            .buttonStyle(.plain)
                            Divider()
                        }
                    }
                } else {
                    GlassEffectContainer(spacing: 8) { FlowLayout {
                        ForEach(s.links, id: \.path) { l in
                            Chip(title: l.title) { Chrome.shared.push(.explore(path: l.path, title: l.title)) }
                        }
                    } }
                }
            }
            if !s.playlists.isEmpty {
                shelf(s.playlists.map { ($0.id, $0.title, $0.creator ?? "", $0.coverUrl, Route.playlist($0.id), false) })
            }
            if !s.albums.isEmpty {
                shelf(s.albums.map { ($0.id, $0.title, $0.artistLine, $0.coverUrl, Route.album($0.id), false) })
            }
            if !s.artists.isEmpty {
                shelf(s.artists.map { ($0.id, $0.name, "", $0.pictureUrl, Route.artist($0.id), true) })
            }
            if !s.tracks.isEmpty {
                ForEach(Array(s.tracks.prefix(8).enumerated()), id: \.offset) { i, t in
                    Button { player.play(s.tracks, startAt: i, from: s.title) } label: { TrackRow(track: t) }
                        .buttonStyle(.plain).trackMenu(t)
                }
            }
        }
    }

    private func shelf(_ items: [(String, String, String, String?, Route, Bool)]) -> some View {
        ScrollView(.horizontal, showsIndicators: false) {
            LazyHStack(alignment: .top, spacing: 12) {
                ForEach(items, id: \.0) { id, title, sub, cover, route, round in
                    Button { Chrome.shared.push(route) } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Cover(url: cover, size: 140, radius: 10, circle: round)
                            Text(title).font(.subheadline.weight(.semibold)).lineLimit(1)
                            if !sub.isEmpty { Text(sub).font(.caption).foregroundStyle(.secondary).lineLimit(1) }
                        }
                        .frame(width: 140, alignment: .leading)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .padding(.horizontal, -16).contentMargins(.horizontal, 16)
    }
}
