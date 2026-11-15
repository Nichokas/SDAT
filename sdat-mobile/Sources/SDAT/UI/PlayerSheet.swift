import AVKit
import SwiftUI

/// Full player. Swipe left (or tap the queue button) for the queue.
struct PlayerSheet: View {
    @Environment(Player.self) private var player
    @Environment(\.scenePhase) private var phase
    @State private var page = 0
    @State private var systemLight = false

    var body: some View {
        // Read in body so Observation re-renders this view when either flag changes.
        let showLive = Live.shared.showSheet && Chrome.shared.showPlayer
        ZStack {
            CoverBackdrop(url: player.current?.album?.coverUrlLarge ?? player.current?.album?.coverUrl,
                          animated: player.isPlaying, bpm: player.current?.bpm, light: systemLight)
                .ignoresSafeArea()
            TabView(selection: $page) {
                NowPlayingPage(showQueue: { withAnimation { page = 1 } }).tag(0)
                QueuePage().tag(1)
            }
            .tabViewStyle(.page(indexDisplayMode: .never))
            .ignoresSafeArea(edges: .bottom)
        }
        // Listen Together from the queue (or a join link opened while the player is up):
        // the root-level sheet can't present over this one, so present it from here.
        .sheet(isPresented: Binding(get: { showLive },
                                    set: { if !$0 { Live.shared.showSheet = false } })) { LiveSheet() }
        .sheet(item: Binding(get: { Chrome.shared.showPlayer ? Chrome.shared.playlistPickerTrack : nil },
                             set: { Chrome.shared.playlistPickerTrack = $0 })) { t in PlaylistPicker(track: t) }
        .toastOverlay(Chrome.shared.toast)
        .presentationDragIndicator(.visible)
        .presentationBackground(.black)
        .preferredColorScheme(.dark)
        // The app forces dark, which hides the iPhone's setting from the environment; the screen's
        // traits still carry it. Toggling it in Control Center makes the app inactive, then active.
        .onChange(of: phase, initial: true) {
            let screen = UIApplication.shared.connectedScenes.compactMap { ($0 as? UIWindowScene)?.screen }.first
            systemLight = screen?.traitCollection.userInterfaceStyle == .light
        }
    }
}

private struct NowPlayingPage: View {
    @Environment(Player.self) private var player
    @Environment(Preferences.self) private var prefs
    let showQueue: () -> Void
    @State private var forward = true
    @State private var nextTap = 0
    @State private var prevTap = 0

    private var slide: AnyTransition {
        .asymmetric(insertion: .move(edge: forward ? .trailing : .leading).combined(with: .opacity),
                    removal: .move(edge: forward ? .leading : .trailing).combined(with: .opacity))
    }

    var body: some View {
        let fav = player.current.flatMap { Favs.shared.known($0.id) } ?? false
        VStack(spacing: 0) {
            Spacer(minLength: 0)
            Cover(url: player.current?.album?.coverUrlLarge ?? player.current?.album?.coverUrl, radius: 16)
                .shadow(color: .black.opacity(0.45), radius: 28, y: 14)
                .id(player.current?.id)
                .transition(slide)
                .scaleEffect(player.isPlaying ? 1 : 0.88)
                .animation(.spring(duration: 0.45, bounce: 0.25), value: player.isPlaying)
            Spacer(minLength: 0)

            HStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(player.current?.displayTitle ?? "").font(.title3.bold()).lineLimit(1)
                    HStack(spacing: 6) {
                        if player.isOffline {
                            Image(systemName: "arrow.down.circle.fill").font(.footnote)
                                .foregroundStyle(.white.opacity(0.65))
                                .accessibilityLabel("Playing downloaded copy")
                        }
                        Text(player.current?.artistLine ?? "").foregroundStyle(.white.opacity(0.65)).lineLimit(1)
                    }
                }
                .id(player.current?.id)
                .transition(slide)
                .frame(maxWidth: .infinity, alignment: .leading)
                Button { if let id = player.current?.id { Favs.shared.toggle(id) } } label: {
                    Image(systemName: fav ? "star.fill" : "star").font(.title3)
                        .frame(width: 44, height: 44).contentShape(.rect)
                }
                .accessibilityLabel(fav ? "Unfavourite" : "Favourite")
                Menu {
                    if let t = player.current {
                        ShareLink(item: URL(string: "https://tidal.com/track/\(t.id)")!) {
                            Label("Share", systemImage: "square.and.arrow.up")
                        }
                        Divider()
                        TrackMenu(track: t)
                    }
                } label: {
                    Image(systemName: "ellipsis").font(.title3)
                        .frame(width: 44, height: 44).contentShape(.rect)
                }
                .accessibilityLabel("More")
            }
            .foregroundStyle(.white)
            .buttonStyle(.plain)
            .padding(.top, 24)

            Scrubber().padding(.top, 14)

            QualityMenu(selection: Binding(
                get: { prefs.streamingQuality },
                set: { prefs.streamingQuality = $0; player.reloadQuality() }
            )) {
                let q = player.shownQuality ?? prefs.streamingQuality
                HStack(spacing: 8) {
                    QualityLogo(quality: q, height: 16)
                    Text(q.badgeText).font(.caption.weight(.semibold)).foregroundStyle(.white.opacity(0.7))
                        .lineLimit(1).fixedSize()
                    Image(systemName: "chevron.up.chevron.down").font(.caption2.weight(.semibold)).foregroundStyle(.white.opacity(0.5))
                }
                .frame(minHeight: 44)
                .contentShape(.rect)
                .animation(.smooth, value: q)
            }
            .foregroundStyle(.white)
            .padding(.top, 2)

            HStack(spacing: 46) {
                Button { forward = false; prevTap += 1; if !Live.shared.guestRefuses() { withAnimation(.spring(duration: 0.4, bounce: 0.2)) { player.previous() } } } label: {
                    Image(systemName: "backward.fill").font(.system(size: 30))
                        .symbolEffect(.wiggle.left, value: prevTap)
                }
                .opacity(Live.shared.isGuest ? 0.4 : 1)
                .sensoryFeedback(.impact(weight: .light), trigger: prevTap)
                .accessibilityLabel("Previous")
                Button { Live.shared.userToggle() } label: {
                    Group {
                        if player.loading { ProgressView().controlSize(.large).tint(.white) } else {
                            Image(systemName: player.isPlaying ? "pause.fill" : "play.fill").font(.system(size: 46))
                                .contentTransition(.symbolEffect(.replace))
                        }
                    }
                    .frame(width: 64, height: 64)
                }
                .opacity(Live.shared.playbackLocked ? 0.4 : 1)
                .accessibilityLabel(player.isPlaying ? "Pause" : "Play")
                Button { forward = true; nextTap += 1; if Live.shared.isGuest { Live.shared.userNext() } else { withAnimation(.spring(duration: 0.4, bounce: 0.2)) { player.next() } } } label: {
                    Image(systemName: "forward.fill").font(.system(size: 30))
                        .symbolEffect(.wiggle.right, value: nextTap)
                }
                .opacity(Live.shared.playbackLocked ? 0.4 : 1)
                .sensoryFeedback(.impact(weight: .light), trigger: nextTap)
                .accessibilityLabel("Next")
            }
            .foregroundStyle(.white)
            .buttonStyle(.plain)
            .padding(.top, 20)

            HStack {
                RoutePicker().frame(width: 44, height: 44)
                Spacer()
                Button(action: showQueue) {
                    Image(systemName: "list.bullet").font(.title3)
                        .frame(width: 44, height: 44).contentShape(.rect)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Queue")
            }
            .foregroundStyle(.white.opacity(0.85))
            .padding(.top, 16)
        }
        .padding(.horizontal, 24)
        .padding(.top, 24)
        .padding(.bottom, 12)
        .task(id: player.current?.id) {
            if let id = player.current?.id { await Favs.shared.check(id) }
        }
    }
}

/// The system AirPlay picker, in the player's white instead of the system blue.
private struct RoutePicker: UIViewRepresentable {
    func makeUIView(context: Context) -> AVRoutePickerView {
        let view = AVRoutePickerView()
        view.prioritizesVideoDevices = false
        view.tintColor = UIColor.white.withAlphaComponent(0.85)
        view.activeTintColor = .white
        return view
    }

    func updateUIView(_ uiView: AVRoutePickerView, context: Context) {}
}

/// Seek slider and times. Its own view so the 4 Hz position ticks re-render only this,
/// not the whole page.
private struct Scrubber: View {
    @Environment(Player.self) private var player
    @State private var scrub: Double?

    var body: some View {
        VStack(spacing: 4) {
            Slider(
                value: Binding(get: { scrub ?? player.position }, set: { scrub = $0 }),
                in: 0...max(player.duration, 1)
            ) { editing in
                if !editing, let s = scrub {
                    scrub = nil
                    // Guests with control ask the host to seek everyone.
                    if !Live.shared.userSeek(s) { player.seek(to: s) }
                }
            }
            .disabled(Live.shared.isGuest && !Live.shared.guestControl)
            .tint(.white)
            HStack {
                Text(Fmt.time(scrub ?? player.position))
                Spacer()
                Text("-" + Fmt.time(player.duration - (scrub ?? player.position)))
            }
            .font(.caption.monospacedDigit()).foregroundStyle(.white.opacity(0.6))
        }
    }
}

private struct QueuePage: View {
    @Environment(Player.self) private var player

    private var repeatLabel: String {
        switch player.repeatMode {
        case .off: "Repeat: Off"
        case .all: "Repeat: All"
        case .one: "Repeat: One"
        }
    }

    var body: some View {
        NavigationStack {
            List {
                if let t = player.current {
                    Section("Now Playing") { TrackRow(track: t).listRowBackground(Color.white.opacity(0.06)) }
                }
                if Live.shared.isGuest {
                    // Guests follow the host's queue; the header opens the session sheet.
                    Section {
                        ForEach(Live.shared.upNext) { row in
                            TrackRow(track: row.track).trackMenu(row.track)
                        }
                        .listRowBackground(Color.white.opacity(0.06))
                    } header: {
                        Button { Live.shared.showSheet = true } label: {
                            Label("Playing from Listen Together", systemImage: "dot.radiowaves.left.and.right")
                                .foregroundStyle(.white)
                        }
                    }
                } else if !player.userQueue.isEmpty {
                    Section("Next in Queue") {
                        ForEach(queueRows(player.userQueue), id: \.key) { i, _, t in
                            Button { player.jump(toUser: i) } label: { TrackRow(track: t) }
                                .buttonStyle(.plain)
                                .trackMenu(t)
                        }
                        .onMove { player.moveUserQueue(from: $0, to: $1) }
                        .onDelete { player.removeUserQueue(at: $0) }
                        .listRowBackground(Color.white.opacity(0.06))
                    }
                }
                if !Live.shared.isGuest {
                Section(player.sourceTitle.map { "Playing from \($0)" } ?? "Up Next") {
                    ForEach(queueRows(player.upNext), id: \.key) { i, _, t in
                        Button { player.jump(toUpNext: i) } label: { TrackRow(track: t) }
                            .buttonStyle(.plain)
                            .trackMenu(t)
                    }
                    .onMove { player.moveUpNext(from: $0, to: $1) }
                    .onDelete { player.removeUpNext(at: $0) }
                    .listRowBackground(Color.white.opacity(0.06))
                }
                }
            }
            .scrollContentBackground(.hidden)
            .contentMargins(.bottom, 24, for: .scrollContent)
            .environment(\.editMode, .constant(.active))
            .navigationTitle("Queue")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button { Live.shared.showSheet = true } label: {
                        Image(systemName: "dot.radiowaves.left.and.right")
                            .foregroundStyle(Live.shared.active ? .white : .white.opacity(0.45))
                    }
                    .accessibilityLabel("Listen Together")
                }
                ToolbarItemGroup(placement: .topBarTrailing) {
                    Button { player.setShuffle(!player.shuffle) } label: {
                        Image(systemName: "shuffle").foregroundStyle(player.shuffle ? .white : .white.opacity(0.45))
                    }
                    .accessibilityLabel(player.shuffle ? "Shuffle: On" : "Shuffle: Off")
                    Button { player.cycleRepeat() } label: {
                        Image(systemName: player.repeatMode == .one ? "repeat.1" : "repeat")
                            .foregroundStyle(player.repeatMode == .off ? .white.opacity(0.45) : .white)
                    }
                    .accessibilityLabel(repeatLabel)
                }
            }
            .background { ZStack { Color.black.opacity(0.55); Rectangle().fill(.ultraThinMaterial).opacity(0.9) }.ignoresSafeArea() }
        }
    }
}

/// Stable row identity for the queue List: track id + occurrence number. Using the array
/// offset as id made every row below a moved one "change identity" mid-drag, so SwiftUI
/// re-diffed the rows under the drag (overlapping text, gaps, detached row backgrounds).
func queueRows<C: Collection>(_ tracks: C) -> [(offset: Int, key: String, track: Track)] where C.Element == Track {
    var seen: [String: Int] = [:]
    return tracks.enumerated().map { i, t in
        let key = "\(t.id)"
        let n = seen[key, default: 0]; seen[key] = n + 1
        return (i, "\(key)#\(n)", t)
    }
}
