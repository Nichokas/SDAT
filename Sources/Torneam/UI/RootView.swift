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
        .onChange(of: chrome.tab) { if chrome.collapsed { chrome.collapsed = false } }
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
        .padding(.horizontal, 24)
        .padding(.bottom, 12)
        .animation(.smooth(duration: 0.38), value: ChromeState(collapsed: chrome.collapsed, detail: chrome.inDetail, mini: hasMini))
    }
}

/// Custom tab bar styled like the native iOS 26 one: glass capsule, sliding selection pill,
/// SF Symbols + 10 pt labels, bounce + haptic on selection.
private struct TabBar: View {
    let collapsed: Bool
    @Environment(Chrome.self) private var chrome
    @Namespace private var pill
    @State private var bounce = 0
    /// Pill position, animated on its own so the tab content swap stays un-animated.
    @State private var selected: AppTab?

    private let items: [(AppTab, String, String)] = [
        (.discover, "Discover", "signpost.right.and.left.fill"),
        (.search, "Search", "magnifyingglass"),
        (.collection, "My Collection", "music.note.house.fill"),
    ]

    var body: some View {
        if collapsed {
            let cur = items.first { $0.0 == chrome.tab } ?? items[0]
            Button { chrome.collapsed = false } label: {
                Image(systemName: cur.2).font(.system(size: 21, weight: .medium)).foregroundStyle(.blue)
                    .frame(width: 62, height: 62)
                    .contentShape(.circle)
            }
            .buttonStyle(.plain)
            .glassEffect(.regular.interactive(), in: .circle)
        } else {
            HStack(spacing: 0) {
                ForEach(items, id: \.0) { tab, title, icon in
                    let on = (selected ?? chrome.tab) == tab
                    Button {
                        if on { chrome.paths[tab] = [] } else {
                            // Swap the tab content instantly (no animated rebuild of the
                            // whole page); only the pill and icon animate.
                            var t = Transaction(); t.disablesAnimations = true
                            withTransaction(t) { chrome.tab = tab }
                            withAnimation(.snappy(duration: 0.32)) { selected = tab }
                            bounce += 1
                        }
                    } label: {
                        VStack(spacing: 1) {
                            Image(systemName: icon)
                                .symbolVariant(on ? .fill : .none)
                                .font(.system(size: 19, weight: on ? .semibold : .medium))
                                .symbolEffect(.bounce, value: on ? bounce : 0)
                                .frame(height: 26)
                            Text(title).font(.system(size: 10, weight: .semibold)).lineLimit(1)
                        }
                        .foregroundStyle(on ? Color.blue : .primary)
                        .frame(maxWidth: .infinity, minHeight: 54)
                        .background {
                            if on {
                                Capsule().fill(.white.opacity(0.12))
                                    .matchedGeometryEffect(id: "pill", in: pill)
                            }
                        }
                        .contentShape(.capsule)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(4)
            .glassEffect(.regular.interactive(), in: .capsule)
            .sensoryFeedback(.selection, trigger: chrome.tab)
            .onChange(of: chrome.tab) { _, t in if selected != t { selected = t } }
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
        safeAreaInset(edge: .bottom, spacing: 0) { Color.clear.frame(height: 78) }
    }

    /// Collapses the bottom chrome when a root list scrolls down.
    func collapsesChrome() -> some View {
        onScrollGeometryChange(for: Bool.self) { g in
            g.contentOffset.y + g.contentInsets.top > 40
        } action: { _, down in
            if Chrome.shared.collapsed != down { withAnimation(.smooth(duration: 0.38)) { Chrome.shared.collapsed = down } }
        }
    }

    /// Profile button that opens Settings.
    func profileToolbar() -> some View {
        // Large title on the same row as the profile button (like the Discover header).
        toolbarTitleDisplayMode(.inlineLarge)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) { ProfileButton(size: 44) }
                .sharedBackgroundVisibility(.hidden)
        }
    }
}

/// Toolbar profile photo (the toolbar supplies the native glass).
struct ProfileToolbarButton: View {
    @State private var profile = Profile.shared
    var body: some View {
        Button { Chrome.shared.push(.settings) } label: {
            Avatar(url: profile.pictureURL).frame(width: 30, height: 30)
        }
        .task { await profile.load() }
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
                        } catch { Chrome.shared.flash(friendlyMessage(error)) }
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

private struct ChromeState: Equatable { var collapsed, detail, mini: Bool }
