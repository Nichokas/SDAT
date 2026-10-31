import SwiftUI

/// Full player. Swipe left (or tap the queue hint) for the queue.
struct PlayerSheet: View {
    @Environment(Player.self) private var player
    @State private var page = 0

    var body: some View {
        ZStack {
            CoverBackdrop(url: player.current?.album?.coverUrlLarge ?? player.current?.album?.coverUrl, animated: player.isPlaying)
            TabView(selection: $page) {
                NowPlayingPage(showQueue: { withAnimation { page = 1 } }).tag(0)
                QueuePage().tag(1)
            }
            .tabViewStyle(.page(indexDisplayMode: .never))
        }
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
                .scaleEffect(player.isPlaying ? 1 : 0.88)
                .animation(.spring(duration: 0.45, bounce: 0.25), value: player.isPlaying)
            Spacer(minLength: 0)

            VStack(alignment: .leading, spacing: 4) {
                Text(player.current?.displayTitle ?? "").font(.title3.bold()).lineLimit(1)
                Text(player.current?.artistLine ?? "").foregroundStyle(.white.opacity(0.65)).lineLimit(1)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            VStack(spacing: 4) {
                Slider(
                    value: Binding(get: { scrub ?? player.position }, set: { scrub = $0 }),
                    in: 0...max(player.duration, 1)
                ) { editing in
                    if !editing, let s = scrub { player.seek(to: s); scrub = nil }
                }
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
                QualityLogo(quality: player.stream?.actualQuality ?? prefs.streamingQuality, height: 16)
                    .padding(.horizontal, 14).padding(.vertical, 8)
                    .glassEffect(.regular.interactive(), in: .capsule)
            }
            .foregroundStyle(.white)

            HStack {
                Button { player.setShuffle(!player.shuffle) } label: {
                    Image(systemName: "shuffle").font(.title3).foregroundStyle(player.shuffle ? Color.blue : .white.opacity(0.8))
                }
                Spacer()
                Button { player.previous() } label: { Image(systemName: "backward.fill").font(.system(size: 30)) }
                Spacer()
                Button { player.toggle() } label: {
                    Image(systemName: player.isPlaying ? "pause.fill" : "play.fill").font(.system(size: 46))
                        .contentTransition(.symbolEffect(.replace))
                        .frame(width: 64, height: 64)
                }
                Spacer()
                Button { player.next() } label: { Image(systemName: "forward.fill").font(.system(size: 30)) }
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
                if !player.userQueue.isEmpty {
                    Section("Next in Queue") {
                        ForEach(Array(player.userQueue.enumerated()), id: \.offset) { i, t in
                            Button { player.jump(toUser: i) } label: { TrackRow(track: t) }
                                .buttonStyle(.plain)
                                .trackMenu(t)
                        }
                        .onMove { player.moveUserQueue(from: $0, to: $1) }
                        .onDelete { player.removeUserQueue(at: $0) }
                        .listRowBackground(Color.white.opacity(0.06))
                    }
                }
                Section(player.sourceTitle.map { "Playing from \($0)" } ?? "Up Next") {
                    ForEach(Array(player.upNext.enumerated()), id: \.offset) { i, t in
                        Button { player.jump(toUpNext: i) } label: { TrackRow(track: t) }
                            .buttonStyle(.plain)
                            .trackMenu(t)
                    }
                    .onMove { player.moveUpNext(from: $0, to: $1) }
                    .onDelete { player.removeUpNext(at: $0) }
                    .listRowBackground(Color.white.opacity(0.06))
                }
            }
            .scrollContentBackground(.hidden)
            .environment(\.editMode, .constant(.active))
            .navigationTitle("Queue")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                Button { player.setShuffle(!player.shuffle) } label: {
                    Image(systemName: "shuffle").foregroundStyle(player.shuffle ? Color.blue : .white)
                }
            }
            .background(.ultraThinMaterial.opacity(0.9))
            .background(Color.black.opacity(0.55))
        }
    }
}
