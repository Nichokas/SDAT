import SwiftUI

/// Tabs with a custom Liquid Glass bottom chrome:
/// - root, not scrolled: expanded tab bar + circular miniplayer
/// - root, scrolled down: tab circle + capsule miniplayer
/// - detail pages: full-width capsule miniplayer, no tab bar
/// - Settings: nothing
struct RootView: View {
    @Environment(Player.self) private var player
    @Environment(Chrome.self) private var chrome

    var body: some View {
        @Bindable var chrome = chrome
        TabView(selection: $chrome.tab) {
            Tab(value: AppTab.discover) {
                NavigationStack(path: chrome.binding(.discover)) { DiscoverView().routes() }
                    .toolbar(.hidden, for: .tabBar)
            }
            Tab(value: AppTab.search) {
                NavigationStack(path: chrome.binding(.search)) { SearchView().routes() }
                    .toolbar(.hidden, for: .tabBar)
            }
            Tab(value: AppTab.collection) {
                NavigationStack(path: chrome.binding(.collection)) { CollectionView().routes() }
                    .toolbar(.hidden, for: .tabBar)
            }
        }
        .overlay(alignment: .bottom) { BottomChrome().ignoresSafeArea(.keyboard) }
        .overlay(alignment: .top) {
            if let t = chrome.toast {
                Text(t).font(.subheadline.weight(.semibold)).padding(.horizontal, 16).padding(.vertical, 10)
                    .glassEffect().padding(.top, 8).transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.spring(duration: 0.35), value: chrome.toast)
        .onChange(of: chrome.tab) { chrome.collapsed = false }
        .sheet(isPresented: $chrome.showPlayer) { PlayerSheet() }
        .sheet(item: $chrome.playlistPickerTrack) { t in PlaylistPicker(track: t) }
        .alert("Playback error", isPresented: Binding(
            get: { player.error != nil },
            set: { if !$0 { player.error = nil } }
        )) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(player.error ?? "")
        }
        .task { await OwnPlaylists.shared.load() }
    }
}

extension Track: Identifiable {}

private struct BottomChrome: View {
    @Environment(Player.self) private var player
    @Environment(Chrome.self) private var chrome
    @Namespace private var ns

    var body: some View {
        let hasMini = player.current != nil && !chrome.inSettings
        GlassEffectContainer(spacing: 10) {
            HStack(spacing: 10) {
                if !chrome.inDetail {
                    TabBar(collapsed: chrome.collapsed && hasMini)
                        .glassEffectID("tabs", in: ns)
                }
                if hasMini {
                    MiniPlayer(style: chrome.inDetail ? .wide : (chrome.collapsed ? .capsule : .circle))
                        .glassEffectID("mini", in: ns)
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, 4)
        .animation(.spring(duration: 0.4, bounce: 0.15), value: chrome.collapsed)
        .animation(.spring(duration: 0.4, bounce: 0.15), value: chrome.inDetail)
        .animation(.spring(duration: 0.4), value: hasMini)
    }
}

private struct TabBar: View {
    let collapsed: Bool
    @Environment(Chrome.self) private var chrome

    private let items: [(AppTab, String, String)] = [
        (.discover, "Discover", "signpost.right.and.left"),
        (.search, "Search", "magnifyingglass"),
        (.collection, "My Collection", "music.note.house"),
    ]

    var body: some View {
        if collapsed {
            let cur = items.first { $0.0 == chrome.tab } ?? items[0]
            Button { chrome.collapsed = false } label: {
                Image(systemName: cur.2).font(.title3).foregroundStyle(.blue).frame(width: 56, height: 56)
            }
            .buttonStyle(.plain)
            .glassEffect(.regular.interactive(), in: .circle)
        } else {
            HStack(spacing: 0) {
                ForEach(items, id: \.0) { tab, title, icon in
                    Button {
                        if chrome.tab == tab { chrome.paths[tab] = [] } else { chrome.tab = tab }
                    } label: {
                        VStack(spacing: 2) {
                            Image(systemName: icon).font(.system(size: 19))
                            Text(title).font(.system(size: 10, weight: .medium)).lineLimit(1)
                        }
                        .foregroundStyle(chrome.tab == tab ? Color.blue : .primary)
                        .frame(maxWidth: .infinity, minHeight: 56)
                        .contentShape(.rect)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 6)
            .glassEffect(.regular.interactive(), in: .capsule)
        }
    }
}

extension View {
    func routes() -> some View {
        navigationDestination(for: Route.self) { route in
            Group {
                switch route {
                case .album(let id): AlbumView(id: id)
                case .playlist(let id): PlaylistView(id: id)
                case .artist(let id): ArtistView(id: id)
                case .settings: SettingsView()
                case .collectionTracks: PagedTracks(title: "Tracks", pager: collectionTracks(), favourites: true)
                case .collectionAlbums: PagedAlbums(title: "Albums", pager: collectionAlbums())
                case .collectionArtists: PagedArtists(title: "Artists", pager: collectionArtists())
                case .collectionPlaylists: PagedPlaylists(title: "Playlists", pager: collectionPlaylists())
                case .explore(let path, let title): ExploreView(path: path, title: title)
                }
            }
            .bottomSpace()
        }
        .bottomSpace()
    }

    /// Room for the floating bottom chrome.
    func bottomSpace() -> some View {
        safeAreaInset(edge: .bottom, spacing: 0) { Color.clear.frame(height: 66) }
    }

    /// Collapses the bottom chrome when a root list scrolls down.
    func collapsesChrome() -> some View {
        onScrollGeometryChange(for: Bool.self) { g in
            g.contentOffset.y + g.contentInsets.top > 40
        } action: { _, down in
            if Chrome.shared.collapsed != down { Chrome.shared.collapsed = down }
        }
    }

    /// Profile button that opens Settings.
    func profileToolbar() -> some View {
        toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button { Chrome.shared.push(.settings) } label: { Image(systemName: "person.crop.circle") }
            }
        }
    }
}

/// Picker of the user's own playlists for "Add to playlist".
struct PlaylistPicker: View {
    let track: Track
    @Environment(\.dismiss) private var dismiss
    @State private var store = OwnPlaylists.shared

    var body: some View {
        NavigationStack {
            List(store.list, id: \.id) { p in
                Button {
                    dismiss()
                    Task {
                        do {
                            try await addToPlaylist(playlistId: p.id, trackId: track.id)
                            Chrome.shared.flash("Added to \(p.name)")
                        } catch { Chrome.shared.flash(error.localizedDescription) }
                    }
                } label: {
                    LabeledContent(p.name, value: "\(p.trackCount)")
                }
                .foregroundStyle(.primary)
            }
            .overlay { if store.list.isEmpty { ContentUnavailableView("No playlists", systemImage: "music.note.list") } }
            .navigationTitle("Add to Playlist")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("Cancel") { dismiss() } }
        }
        .presentationDetents([.medium, .large])
        .task { await store.load(force: true) }
    }
}
