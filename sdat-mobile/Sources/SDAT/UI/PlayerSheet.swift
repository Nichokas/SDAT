import SwiftUI

/// Full player. Swipe left (or tap the queue hint) for the queue.
struct PlayerSheet: View {
    @Environment(Player.self) private var player
    @State private var page = 0

    var body: some View {
        // Read in body so Observation re-renders this view when either flag changes.
        let showLive = Live.shared.showSheet && Chrome.shared.showPlayer
        ZStack {
            CoverBackdrop(url: player.current?.album?.coverUrlLarge ?? player.current?.album?.coverUrl,
                          animated: player.isPlaying, bpm: player.current?.bpm)
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
        .presentationDragIndicator(.visible)
        .presentationBackground(.black)
        .preferredColorScheme(.dark)
    }
}

private struct NowPlayingPage: View {
    @Environment(Player.self) private var player
    @Environment(Preferences.self) private var prefs
    let showQueue: () -> Void
    @State private var scrub: Double?
    @State private var forward = true
    @State private var nextTap = 0
    @State private var prevTap = 0

    private var slide: AnyTransition {
        .asymmetric(insertion: .move(edge: forward ? .trailing : .leading).combined(with: .opacity),
                    removal: .move(edge: forward ? .leading : .trailing).combined(with: .opacity))
    }

    var body: some View {
        let fav = player.current.flatMap { Favs.shared.known($0.id) } ?? false
        VStack(spacing: 18) {
            HStack {
                GlassEffectContainer {
                    HStack(spacing: 0) {
                        if let t = player.current {
                            ShareLink(item: URL(string: "https://tidal.com/track/\(t.id)")!) {
                                Image(systemName: "square.and.arrow.up").frame(width: 44, height: 40)
                            }
                        }
                        Button { if let id = player.current?.id { Favs.shared.toggle(id) } } label: {
                            Image(systemName: fav ? "star.fill" : "star").frame(width: 44, height: 40)
                                .foregroundStyle(fav ? Color.yellow : .white)
                        }
                    }
                    .glassEffect(.regular.interactive(), in: .capsule)
                }
                Spacer()
                Menu {
                    if let t = player.current { TrackMenu(track: t) }
                } label: {
                    Image(systemName: "ellipsis").frame(width: 44, height: 44)
                        .glassEffect(.regular.interactive(), in: .circle)
                }
            }
            .foregroundStyle(.white)
            .padding(.top, 14)

            Spacer(minLength: 0)
            Cover(url: player.current?.album?.coverUrlLarge ?? player.current?.album?.coverUrl, radius: 16)
                .shadow(color: .black.opacity(0.45), radius: 28, y: 14)
                .id(player.current?.id)
                .transition(slide)
                .scaleEffect(player.isPlaying ? 1 : 0.88)
                .animation(.spring(duration: 0.45, bounce: 0.25), value: player.isPlaying)
            Spacer(minLength: 0)

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

            QualityMenu(selection: Binding(
                get: { prefs.streamingQuality },
                set: { prefs.streamingQuality = $0; player.reloadQuality() }
            )) {
                let q = player.shownQuality ?? prefs.streamingQuality
                HStack(spacing: 10) {
                    QualityLogo(quality: q, height: 22)
                    Text(q.badgeText).font(.subheadline.weight(.semibold)).foregroundStyle(.white.opacity(0.9))
                        .lineLimit(1).fixedSize()
                    Image(systemName: "chevron.up.chevron.down").font(.caption.weight(.semibold)).foregroundStyle(.white.opacity(0.6))
                }
                .padding(.horizontal, 18).frame(minHeight: 44)
                .contentShape(.capsule)
                .glassEffect(.regular.interactive(), in: .capsule)
                .animation(.smooth, value: q)
            }
            .foregroundStyle(.white)

            HStack {
                Button { player.setShuffle(!player.shuffle) } label: {
                    Image(systemName: "shuffle").font(.title3).foregroundStyle(player.shuffle ? Color.blue : .white.opacity(0.8))
                }
                Spacer()
                Button { forward = false; prevTap += 1; if !Live.shared.guestRefuses() { withAnimation(.spring(duration: 0.4, bounce: 0.2)) { player.previous() } } } label: {
                    Image(systemName: "backward.fill").font(.system(size: 30))
                        .symbolEffect(.wiggle.left, value: prevTap)
                }
                .opacity(Live.shared.isGuest ? 0.4 : 1)
                .sensoryFeedback(.impact(weight: .light), trigger: prevTap)
                Spacer()
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
                Spacer()
                Button { forward = true; nextTap += 1; if Live.shared.isGuest { Live.shared.userNext() } else { withAnimation(.spring(duration: 0.4, bounce: 0.2)) { player.next() } } } label: {
                    Image(systemName: "forward.fill").font(.system(size: 30))
                        .symbolEffect(.wiggle.right, value: nextTap)
                }
                .opacity(Live.shared.playbackLocked ? 0.4 : 1)
                .sensoryFeedback(.impact(weight: .light), trigger: nextTap)
                Spacer()
                Button { player.cycleRepeat() } label: {
                    Image(systemName: player.repeatMode == .one ? "repeat.1" : "repeat").font(.title3)
                        .foregroundStyle(player.repeatMode == .off ? .white.opacity(0.8) : Color.blue)
                }
            }
            .foregroundStyle(.white)
            .buttonStyle(.plain)

            HStack {
                Spacer()
                Button(action: showQueue) {
                    HStack(spacing: 4) {
                        Image(systemName: "list.bullet")
                        Image(systemName: "chevron.right").font(.caption.weight(.bold))
                    }
                    .padding(.horizontal, 14).padding(.vertical, 9)
                    .glassEffect(.regular.interactive(), in: .capsule)
                }
                .buttonStyle(.plain)
                .foregroundStyle(.white.opacity(0.85))
            }
        }
        .padding(.horizontal, 24)
        .padding(.bottom, 12)
        .task(id: player.current?.id) {
            if let id = player.current?.id { await Favs.shared.check(id) }
        }
    }
}

private struct QueuePage: View {
    @Environment(Player.self) private var player

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
                            .foregroundStyle(Live.shared.active ? Color.blue : .white)
                    }
                    .accessibilityLabel("Listen Together")
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button { player.setShuffle(!player.shuffle) } label: {
                        Image(systemName: "shuffle").foregroundStyle(player.shuffle ? Color.blue : .white)
                    }
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
