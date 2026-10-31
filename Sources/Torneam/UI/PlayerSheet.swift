import SwiftUI

/// Full player. Swipe to the right page for the reorderable queue.
struct PlayerSheet: View {
    @Environment(Player.self) private var player
    @State private var page = 0

    var body: some View {
        ZStack {
            DiscoLights(seed: player.current?.id ?? "", playing: player.isPlaying)
            TabView(selection: $page) {
                NowPlayingPage(showQueue: { withAnimation { page = 1 } }).tag(0)
                QueuePage().tag(1)
            }
            .tabViewStyle(.page(indexDisplayMode: .never))
        }
        .presentationDragIndicator(.visible)
        .preferredColorScheme(.dark)
    }
}

private struct NowPlayingPage: View {
    @Environment(Player.self) private var player
    @Environment(Preferences.self) private var prefs
    let showQueue: () -> Void
    @State private var scrub: Double?
    @State private var isFav = false

    var body: some View {
        @Bindable var prefs = prefs
        VStack(spacing: 20) {
            HStack {
                GlassEffectContainer {
                    HStack(spacing: 0) {
                        if let t = player.current {
                            ShareLink(item: URL(string: "https://tidal.com/track/\(t.id)")!) {
                                Image(systemName: "square.and.arrow.up").frame(width: 40, height: 40)
                            }
                        }
                        Button { toggleFav() } label: {
                            Image(systemName: isFav ? "star.fill" : "star").frame(width: 40, height: 40)
                        }
                    }
                    .glassEffect()
                }
                Spacer()
            }
            .foregroundStyle(.white)

            Cover(url: player.current?.album?.coverUrlLarge, radius: 14)
                .shadow(radius: 20)
                .scaleEffect(player.isPlaying ? 1 : 0.9)
                .animation(.spring, value: player.isPlaying)

            VStack(spacing: 4) {
                Text(player.current?.displayTitle ?? "").font(.title3.bold()).lineLimit(1)
                Text(player.current?.artistLine ?? "").foregroundStyle(.secondary).lineLimit(1)
            }

            VStack(spacing: 4) {
                Slider(
                    value: Binding(get: { scrub ?? player.position }, set: { scrub = $0 }),
                    in: 0...max(player.duration, 1)
                ) { editing in
                    if !editing, let s = scrub { player.seek(to: s); scrub = nil }
                }
                HStack {
                    Text(Fmt.time(scrub ?? player.position))
                    Spacer()
                    Text("-" + Fmt.time(player.duration - (scrub ?? player.position)))
                }
                .font(.caption.monospacedDigit()).foregroundStyle(.secondary)
            }

            QualityMenu(selection: Binding(
                get: { prefs.streamingQuality },
                set: { prefs.streamingQuality = $0; player.reloadQuality() }
            )) {
                QualityBadge(quality: prefs.streamingQuality)
                    .padding(.horizontal, 12).padding(.vertical, 6)
                    .glassEffect()
            }
            .foregroundStyle(.white)

            HStack {
                Button { player.shuffle.toggle() } label: {
                    Image(systemName: "shuffle").foregroundStyle(player.shuffle ? Color.accentColor : .white)
                }
                Spacer()
                Button { player.previous() } label: { Image(systemName: "backward.fill").font(.title) }
                Spacer()
                Button { player.toggle() } label: {
                    Image(systemName: player.isPlaying ? "pause.fill" : "play.fill").font(.largeTitle)
                        .contentTransition(.symbolEffect(.replace))
                }
                Spacer()
                Button { player.next() } label: { Image(systemName: "forward.fill").font(.title) }
                Spacer()
                Button { player.cycleRepeat() } label: {
                    Image(systemName: player.repeatMode == .one ? "repeat.1" : "repeat")
                        .foregroundStyle(player.repeatMode == .off ? .white : Color.accentColor)
                }
            }
            .foregroundStyle(.white)
            .buttonStyle(.plain)

            Spacer(minLength: 0)
            HStack {
                Spacer()
                Button(action: showQueue) {
                    Label("Queue", systemImage: "list.bullet").labelStyle(.iconOnly)
                }
                .foregroundStyle(.white.opacity(0.6))
            }
        }
        .padding(24)
        .task(id: player.current?.id) {
            guard let id = player.current?.id else { return }
            isFav = (try? await isFavourite(kind: .track, id: id)) ?? false
        }
    }

    private func toggleFav() {
        guard let id = player.current?.id else { return }
        let target = !isFav
        isFav = target
        Task { do { try await setFavourite(kind: .track, id: id, favourite: target) } catch { isFav = !target } }
    }
}

private struct QueuePage: View {
    @Environment(Player.self) private var player

    var body: some View {
        NavigationStack {
            List {
                if let t = player.current {
                    Section("Now Playing") { TrackRow(track: t) }
                }
                Section(player.sourceTitle.map { "Playing from \($0)" } ?? "Up Next") {
                    ForEach(Array(player.upNext.enumerated()), id: \.offset) { i, t in
                        Button { player.jump(toUpNext: i) } label: { TrackRow(track: t) }
                            .buttonStyle(.plain)
                    }
                    .onMove { player.moveUpNext(from: $0, to: $1) }
                    .onDelete { player.removeUpNext(at: $0) }
                }
            }
            .scrollContentBackground(.hidden)
            .environment(\.editMode, .constant(.active))
            .navigationTitle("Queue")
            .navigationBarTitleDisplayMode(.inline)
        }
        .background(.clear)
    }
}

/// Slowly drifting coloured lights behind the player; brighter while playing.
struct DiscoLights: View {
    let seed: String
    let playing: Bool

    var body: some View {
        TimelineView(.animation(paused: !playing)) { ctx in
            let t = ctx.date.timeIntervalSinceReferenceDate
            let h = Double(abs(seed.hashValue % 360)) / 360
            Canvas { gc, size in
                for i in 0..<4 {
                    let fi = Double(i)
                    let x = size.width * (0.5 + 0.4 * sin(t * (0.21 + fi * 0.07) + fi * 1.7))
                    let y = size.height * (0.45 + 0.4 * cos(t * (0.17 + fi * 0.05) + fi * 2.3))
                    let r = size.width * (0.55 + 0.1 * sin(t * 0.9 + fi))
                    let color = Color(hue: (h + fi * 0.18).truncatingRemainder(dividingBy: 1), saturation: 0.8, brightness: 0.9)
                    gc.fill(Path(ellipseIn: CGRect(x: x - r / 2, y: y - r / 2, width: r, height: r)),
                            with: .color(color.opacity(playing ? 0.55 : 0.3)))
                }
            }
            .blur(radius: 70)
        }
        .background(.black)
        .ignoresSafeArea()
    }
}
