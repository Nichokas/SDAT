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
        // Native iOS 26 tab bar: system Liquid Glass, sliding selection capsule,
        // minimizes on scroll, miniplayer as the bottom accessory (inline when minimized).
        TabView(selection: $chrome.tab) {
            Tab("Discover", systemImage: "signpost.right.and.left", value: AppTab.discover) {
                NavigationStack(path: chrome.binding(.discover)) { DiscoverView().routes() }
            }
            Tab("My Collection", systemImage: "music.note.house", value: AppTab.collection) {
                NavigationStack(path: chrome.binding(.collection)) { CollectionView().routes() }
            }
            Tab("Search", systemImage: "magnifyingglass", value: AppTab.search, role: .search) {
                NavigationStack(path: chrome.binding(.search)) { SearchView().routes() }
            }
        }
        .tabBarMinimizeBehavior(.onScrollDown)
        .miniAccessory(player.current != nil)
        .overlay(alignment: .top) {
            if let t = chrome.toast {
                Text(t).font(.subheadline.weight(.semibold)).padding(.horizontal, 16).padding(.vertical, 10)
                    .glassEffect().padding(.top, 8).transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.spring(duration: 0.35), value: chrome.toast)
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

/// Miniplayer inside the system accessory (the system draws the glass).
/// Tap opens the full player; the play button toggles.
private struct AccessoryPlayer: View {
    @Environment(Player.self) private var player
    @Environment(Chrome.self) private var chrome
    @Environment(\.tabViewBottomAccessoryPlacement) private var placement

    var body: some View {
        let inline = placement == .inline
        HStack(spacing: 10) {
            Cover(url: player.current?.album?.coverUrl, size: inline ? 26 : 32, radius: 6)
            VStack(alignment: .leading, spacing: 0) {
                Text(player.current?.displayTitle ?? "").font(.subheadline.weight(.semibold)).lineLimit(1)
                if !inline {
                    Text(player.current?.artistLine ?? "").font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
            }
            Spacer(minLength: 0)
            Button { player.toggle() } label: {
                Group {
                    if player.loading { ProgressView().controlSize(.small) } else {
                        Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
                            .contentTransition(.symbolEffect(.replace))
                    }
                }
                .frame(width: 32, height: 32)
                .overlay { ProgressRing(progress: player.progress, lineWidth: 2).padding(1) }
            }
            .buttonStyle(.plain)
            if !inline {
                Button { player.next() } label: { Image(systemName: "forward.fill").frame(width: 32, height: 32) }
                    .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 12)
        .contentShape(.rect)
        .onTapGesture { chrome.showPlayer = true }
        .sensoryFeedback(.selection, trigger: player.isPlaying)
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
        self
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
            ToolbarItem(placement: .topBarTrailing) { ProfileToolbarButton() }
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

private extension View {
    /// System bottom accessory; hidden when nothing is playing (iOS 26.1+).
    @ViewBuilder func miniAccessory(_ on: Bool) -> some View {
        if #available(iOS 26.1, *) {
            tabViewBottomAccessory(isEnabled: on) { AccessoryPlayer() }
        } else if on {
            tabViewBottomAccessory { AccessoryPlayer() }
        } else {
            self
        }
    }
}
