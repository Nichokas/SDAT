import SwiftUI
import os

let chromeLog = Logger(subsystem: "com.nichokas.Torneam", category: "chrome")

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
        .onChange(of: chrome.tab) { if chrome.collapsed { withAnimation(Motion.bar) { chrome.collapsed = false } } }
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
        // Spacing 0: bar and miniplayer stay separate shapes (no pinched neck);
        // glassEffectID still morphs each between its states.
        GlassEffectContainer(spacing: 0) {
            HStack(spacing: 12) {
                if !chrome.inDetail {
                    // ONE view whose sizes/opacities come from `collapsed` (no branch swap),
                    // so the scroll flip interpolates exactly like the detail-page capsule.
                    TabBar(collapsed: chrome.collapsed && hasMini)
                        .glassEffect(.regular.interactive(), in: .capsule)
                        .glassEffectID("tabs", in: ns)
                }
                if hasMini {
                    let style: MiniPlayer.Style = chrome.inDetail ? .wide : (chrome.collapsed ? .capsule : .circle)
                    MiniPlayer(style: style)
                        .glassEffect(.regular, in: .capsule)
                        .glassEffectID("mini", in: ns)
                }
            }
        }
        .padding(.horizontal, 24)
        .padding(.bottom, 12)
        .animation(Motion.bar, value: ChromeState(collapsed: chrome.collapsed, detail: chrome.inDetail, mini: hasMini))
    }
}

/// Custom tab bar styled like the native iOS 26 one: glass capsule, sliding selection pill,
/// SF Symbols + 10 pt labels, bounce + haptic on selection.
private struct TabBar: View {
    let collapsed: Bool
    @Environment(Chrome.self) private var chrome
    @State private var bounce = 0
    /// Pill position, animated on its own so the tab content swap stays un-animated.
    @State private var selected: AppTab?
    @State private var barWidth: CGFloat = 0
    @State private var dragX: CGFloat?
    @State private var hovered: Int?
    @State private var pressed = false
    @State private var pillL: CGFloat = 0
    @State private var pillR: CGFloat = 0

    /// Front edge (direction of travel) springs fast, back edge lags: the pill stretches
    /// toward the target, then the back edge catches up with a small overshoot.
    private func movePill(to x: CGFloat, width w: CGFloat, fast: Double, slow: Double) {
        let right = x > pillL
        let front = Animation.spring(response: fast, dampingFraction: 0.72)
        let back = Animation.spring(response: slow, dampingFraction: 0.7)
        withAnimation(right ? front : back) { pillR = x + w }
        withAnimation(right ? back : front) { pillL = x }
    }

    private func select(_ tab: AppTab) {
        if tab == (selected ?? chrome.tab) {
            if chrome.tab == tab { chrome.paths[tab] = [] }
            return
        }
        chrome.tab = tab
        withAnimation(Motion.pill) { selected = tab }
        bounce += 1
    }

    private let items: [(AppTab, String, String)] = [
        (.discover, "Discover", "signpost.right.and.left.fill"),
        (.search, "Search", "magnifyingglass"),
        (.collection, "My Collection", "music.note.house.fill"),
    ]

    var body: some View {
        ZStack {
            expanded
                .frame(width: collapsed && barWidth > 0 ? barWidth + 8 : nil)
                .opacity(collapsed ? 0 : 1)
                // Sequenced cross-fade: out fast while shrinking, in late while growing.
                .animation(collapsed ? Motion.fadeOut : Motion.fadeIn, value: collapsed)
                .allowsHitTesting(!collapsed)
            circle
                .opacity(collapsed ? 1 : 0)
                .animation(collapsed ? Motion.fadeIn : Motion.fadeOut, value: collapsed)
                .allowsHitTesting(collapsed)
        }
        .frame(maxWidth: collapsed ? 54 : .infinity)
        .frame(height: collapsed ? 54 : 62)
        .clipShape(.capsule)
    }

    @ViewBuilder private var circle: some View {
            let cur = items.first { $0.0 == chrome.tab } ?? items[0]
            Button { withAnimation(Motion.bar) { chrome.collapsed = false } } label: {
                Image(systemName: cur.2).font(.system(size: 21, weight: .medium)).foregroundStyle(.blue)
                    .frame(width: 54, height: 54)
                    .contentShape(.circle)
            }
            .buttonStyle(.plain)
    }

    @ViewBuilder private var expanded: some View {
            let idx = items.firstIndex { $0.0 == (selected ?? chrome.tab) } ?? 0
            let shown = hovered ?? idx
            let w3 = barWidth / CGFloat(items.count)
            HStack(spacing: 0) {
                ForEach(Array(items.enumerated()), id: \.offset) { i, item in
                    let on = shown == i
                    VStack(spacing: 1) {
                        Image(systemName: item.2)
                            .symbolVariant(on ? .fill : .none)
                            .font(.system(size: 19, weight: on ? .semibold : .medium))
                            .symbolEffect(.bounce, value: i == idx ? bounce : 0)
                            .frame(height: 26)
                        Text(item.1).font(.system(size: 10, weight: .semibold)).lineLimit(1)
                    }
                    .foregroundStyle(on ? Color.blue : .primary)
                    .frame(maxWidth: .infinity, minHeight: 54)
                }
            }
            .background(alignment: .leading) {
                // Liquid pill: its two edges spring separately (the leading edge in the
                // direction of travel is quicker), so it stretches while moving and
                // settles with a slight overshoot. Real interactive glass, no flat fill.
                // Clear lens in its own container so it never metaball-merges with the
                // miniplayer; slightly taller than the bar, more so while pressed.
                // At rest: a faint, nearly clear capsule inside the bar (like native).
                // Pressed/dragged: a clear glass lens that grows past the bar edges.
                ZStack {
                    if pressed {
                        GlassEffectContainer(spacing: 0) {
                            Color.clear
                                .frame(width: max(pillR - pillL + 6, 0), height: 70)
                                .glassEffect(.clear.interactive(), in: .capsule)
                        }
                        .transition(.scale(scale: 0.85).combined(with: .opacity))
                    } else {
                        Capsule().fill(.white.opacity(0.1))
                            .frame(width: max(pillR - pillL, 0), height: 54)
                            .transition(.opacity)
                    }
                }
                .frame(width: max(pillR - pillL, 0))
                    .offset(x: pillL)
            }
            .contentShape(.capsule)
            .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { w in
                barWidth = w
                let w3 = w / CGFloat(items.count)
                pillL = CGFloat(idx) * w3; pillR = pillL + w3
            }
            .gesture(DragGesture(minimumDistance: 0)
                .onChanged { g in
                    guard w3 > 0 else { return }
                    if !pressed { withAnimation(.snappy(duration: 0.2)) { pressed = true } }
                    // Tiny movement = tap: keep the pill put until release.
                    guard abs(g.translation.width) > 6 || dragX != nil else { return }
                    let x = min(max(g.location.x - w3 / 2, 0), barWidth - w3)
                    dragX = x
                    movePill(to: x, width: w3, fast: 0.18, slow: 0.32)
                    let h = min(max(Int(g.location.x / w3), 0), items.count - 1)
                    if hovered != h { hovered = h }
                }
                .onEnded { g in
                    let i = w3 > 0 ? min(max(Int(g.location.x / w3), 0), items.count - 1) : idx
                    hovered = nil
                    dragX = nil
                    withAnimation(Motion.pill) { pressed = false }
                    select(items[i].0)
                    movePill(to: CGFloat(i) * w3, width: w3, fast: 0.3, slow: 0.45)
                })
            .padding(4)
            .sensoryFeedback(.selection, trigger: chrome.tab)
            .sensoryFeedback(.selection, trigger: hovered) { _, n in n != nil }
            .onAppear { if selected == nil { selected = chrome.tab } }
            .onChange(of: chrome.tab) { _, t in
                if selected != t { withAnimation(Motion.pill) { selected = t } }
                if dragX == nil, let i = items.firstIndex(where: { $0.0 == t }), barWidth > 0 {
                    let w3 = barWidth / CGFloat(items.count)
                    movePill(to: CGFloat(i) * w3, width: w3, fast: 0.3, slow: 0.45)
                }
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
        // Offset only (no state per frame); the flip happens once per crossing, with
        // hysteresis, and is deferred out of the scroll update's transaction (which
        // dropped the animation) into the same animated path as the album transition.
        onScrollGeometryChange(for: CGFloat.self) { g in
            g.contentOffset.y + g.contentInsets.top
        } action: { _, y in
            let c = Chrome.shared
            let want: Bool? = y > 60 ? true : (y < 20 ? false : nil)
            guard let want, want != c.collapsed, !c.collapsePending else { return }
            c.collapsePending = true
            Task { @MainActor in
                c.collapsePending = false
                if c.collapsed != want {
                    chromeLog.debug("collapse -> \(want) at y=\(y, format: .fixed(precision: 0))")
                    withTransaction(Transaction(animation: Motion.bar)) { c.collapsed = want }
                }
            }
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

/// Bottom-bar motion: native ease-in/ease-out springs, a little slower through the middle.
enum Motion {
    static let pill = Animation.spring(response: 0.4, dampingFraction: 0.86)
    static let bar = Animation.smooth(duration: 0.4)
    /// Shared content fades for BOTH tab bar and miniplayer: out early, in late, inside the 0.4 s morph.
    static let fadeOut = Animation.easeOut(duration: 0.15)
    static let fadeIn = Animation.easeIn(duration: 0.2).delay(0.2)
}
