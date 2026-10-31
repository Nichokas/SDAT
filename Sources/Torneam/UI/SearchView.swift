import SwiftUI

struct SearchView: View {
    @State private var query = ""
    @State private var results: SearchResults?
    @State private var searching = false

    var body: some View {
        List {
            if let r = results {
                if !r.tracks.isEmpty {
                    Section("Tracks") { TrackList(tracks: r.tracks, source: "Search") }
                }
                if !r.artists.isEmpty {
                    Section("Artists") {
                        ForEach(r.artists, id: \.id) { a in
                            NavigationLink(value: Route.artist(a.id)) {
                                HStack {
                                    Cover(url: a.pictureUrl, size: 44, radius: 22)
                                    Text(a.name)
                                }
                            }
                        }
                    }
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
        .overlay { if searching { ProgressView() } }
        .navigationTitle("Search")
        .profileToolbar()
        .searchable(text: $query, prompt: "Artists, songs, albums")
        .task(id: query) {
            let q = query.trimmingCharacters(in: .whitespaces)
            guard !q.isEmpty else { results = nil; return }
            try? await Task.sleep(for: .milliseconds(350))
            guard !Task.isCancelled else { return }
            searching = true
            results = try? await search(query: q)
            searching = false
        }
    }
}

struct MediaRow: View {
    let title: String
    let subtitle: String
    let cover: String?

    var body: some View {
        HStack(spacing: 12) {
            Cover(url: cover, size: 52, radius: 6)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).lineLimit(1)
                Text(subtitle).font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
            }
        }
    }
}
