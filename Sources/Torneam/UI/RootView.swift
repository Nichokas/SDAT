import SwiftUI

enum Route: Hashable {
    case album(String)
    case playlist(String)
    case artist(String)
    case settings
}

/// Tabs with the iOS 26 Liquid Glass tab bar. Scrolling down minimises the bar to a circle
/// and the miniplayer accessory becomes a capsule beside it.
struct RootView: View {
    @Environment(Player.self) private var player
    @State private var showPlayer = false

    var body: some View {
        TabView {
            Tab("Discover", systemImage: "signpost.right.and.left") {
                NavigationStack { DiscoverView().routes() }
            }
            Tab("Search", systemImage: "magnifyingglass") {
                NavigationStack { SearchView().routes() }
            }
            Tab("My Collection", systemImage: "music.note.house") {
                NavigationStack { CollectionView().routes() }
            }
        }
        .tabBarMinimizeBehavior(.onScrollDown)
        .tabViewBottomAccessory {
            if player.current != nil {
                MiniPlayer { showPlayer = true }
            }
        }
        .sheet(isPresented: $showPlayer) {
            PlayerSheet()
        }
        .alert("Playback error", isPresented: Binding(
            get: { player.error != nil },
            set: { if !$0 { player.error = nil } }
        )) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(player.error ?? "")
        }
    }
}

extension View {
    func routes() -> some View {
        navigationDestination(for: Route.self) { route in
            switch route {
            case .album(let id): AlbumView(id: id)
            case .playlist(let id): PlaylistView(id: id)
            case .artist(let id): ArtistView(id: id)
            case .settings: SettingsView()
            }
        }
    }

    /// Profile button that opens Settings, used in every tab's toolbar.
    func profileToolbar() -> some View {
        toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink(value: Route.settings) {
                    Image(systemName: "person.crop.circle")
                }
            }
        }
    }
}
