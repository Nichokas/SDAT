import SwiftUI
import UIKit

/// Square cover served from the shared image cache (no flash when cached).
struct Cover: View {
    let url: String?
    var size: CGFloat? = nil
    var radius: CGFloat = 8
    var circle = false

    var body: some View {
        CachedImage(url: url)
            .frame(width: size, height: size)
            .aspectRatio(1, contentMode: .fit)
            .clipShape(circle ? AnyShape(.circle) : AnyShape(.rect(cornerRadius: radius)))
    }
}

struct CachedImage: View {
    let url: String?
    @State private var image: UIImage?

    var body: some View {
        Rectangle().fill(Color.white.opacity(0.08))
            .overlay {
                if let img = image ?? url.flatMap(ImageStore.shared.cached) {
                    Image(uiImage: img).resizable().scaledToFill()
                } else {
                    Image(systemName: "music.note").foregroundStyle(.secondary)
                }
            }
            .clipped()
            .task(id: url) {
                guard let url else { image = nil; return }
                image = ImageStore.shared.cached(url)
                if image == nil { image = await ImageStore.shared.image(url) }
            }
    }
}

/// Animated bars shown on the playing row.
struct Equalizer: View {
    let playing: Bool
    var body: some View {
        TimelineView(.animation(minimumInterval: 1 / 30, paused: !playing)) { ctx in
            let t = ctx.date.timeIntervalSinceReferenceDate
            HStack(alignment: .bottom, spacing: 2) {
                ForEach(0..<3) { i in
                    let h = playing ? 0.35 + 0.65 * abs(sin(t * (5 + Double(i) * 1.7) + Double(i))) : 0.35
                    Capsule().frame(width: 3, height: 14 * h)
                }
            }
            .frame(width: 15, height: 14, alignment: .bottom)
        }
        .foregroundStyle(.blue)
    }
}

struct TrackRow: View {
    let track: Track
    var showCover = true
    var number: Int? = nil
    @Environment(Player.self) private var player

    var body: some View {
        let isCurrent = player.current?.id == track.id
        HStack(spacing: 12) {
            if let number {
                Group {
                    if isCurrent { Equalizer(playing: player.isPlaying) } else {
                        Text("\(number)").font(.subheadline.monospacedDigit()).foregroundStyle(.secondary)
                    }
                }
                .frame(width: 24)
            }
            if showCover {
                Cover(url: track.album?.coverUrl, size: 44, radius: 6)
                    .overlay { if isCurrent && number == nil { Color.black.opacity(0.45).overlay(Equalizer(playing: player.isPlaying)).clipShape(.rect(cornerRadius: 6)) } }
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(track.displayTitle).lineLimit(1)
                    .foregroundStyle(isCurrent ? Color.blue : .primary)
                    .fontWeight(isCurrent ? .semibold : .regular)
                HStack(spacing: 4) {
                    if track.explicit { Image(systemName: "e.square.fill").font(.caption2) }
                    Text(track.artistLine).lineLimit(1)
                }
                .font(.subheadline).foregroundStyle(isCurrent ? Color.blue.opacity(0.8) : .secondary)
            }
            Spacer(minLength: 0)
            Text(Fmt.time(Double(track.durationSecs)))
                .font(.subheadline.monospacedDigit()).foregroundStyle(.secondary)
        }
        .opacity(track.streamable ? 1 : 0.4)
        .contentShape(.rect)
    }
}

/// Long-press menu shared by every track row.
struct TrackMenu: View {
    let track: Track
    var ownPlaylistId: String? = nil
    var onRemoved: (() -> Void)? = nil
    @Environment(Player.self) private var player

    var body: some View {
        let fav = Favs.shared.known(track.id) ?? false
        Button(fav ? "Remove from Favourites" : "Add to Favourites", systemImage: fav ? "star.slash" : "star") {
            Favs.shared.toggle(track.id)
        }
        Divider()
        Button("Play Next", systemImage: "text.line.first.and.arrowtriangle.forward") {
            player.playNext(track); Chrome.shared.flash("Playing next")
        }
        Button("Add to Queue", systemImage: "text.line.last.and.arrowtriangle.forward") {
            player.addToQueue(track); Chrome.shared.flash("Added to queue")
        }
        Button("Add to Playlist…", systemImage: "text.badge.plus") { Chrome.shared.playlistPickerTrack = track }
        if let pid = ownPlaylistId {
            Button("Remove from Playlist", systemImage: "minus.circle", role: .destructive) {
                Task {
                    do {
                        try await removeFromPlaylist(playlistId: pid, trackId: track.id)
                        onRemoved?()
                        Chrome.shared.flash("Removed from playlist")
                    } catch { Chrome.shared.flash(error.localizedDescription) }
                }
            }
        }
        Divider()
        Button("Start Radio", systemImage: "dot.radiowaves.left.and.right") {
            Task {
                if let pager = try? await trackRadio(trackId: track.id), let list = try? await pager.nextPage() {
                    player.play([track] + list, from: "\(track.title) Radio")
                }
            }
        }
        if let a = track.album {
            Button("Go to Album", systemImage: "square.stack") { Chrome.shared.showPlayer = false; Chrome.shared.push(.album(a.id)) }
        }
        if let ar = track.artists.first {
            Button("Go to Artist", systemImage: "music.microphone") { Chrome.shared.showPlayer = false; Chrome.shared.push(.artist(ar.id)) }
        }
    }
}

extension View {
    /// Long-press menu; favourite state is fetched lazily once the row settles on screen.
    func trackMenu(_ t: Track, ownPlaylistId: String? = nil, onRemoved: (() -> Void)? = nil) -> some View {
        contextMenu { TrackMenu(track: t, ownPlaylistId: ownPlaylistId, onRemoved: onRemoved) }
            .task(id: t.id) {
                try? await Task.sleep(for: .milliseconds(600))
                guard !Task.isCancelled else { return }
                await Favs.shared.check(t.id)
            }
    }
}

/// Plays a track list and offers per-row actions (used inside List).
struct TrackList: View {
    let tracks: [Track]
    var source: String?
    var showCover = true
    var numbered = false
    var ownPlaylistId: String? = nil
    var onRemoved: ((Track) -> Void)? = nil
    @Environment(Player.self) private var player

    var body: some View {
        ForEach(Array(tracks.enumerated()), id: \.offset) { i, t in
            Button { player.play(tracks, startAt: i, from: source) } label: {
                TrackRow(track: t, showCover: showCover, number: numbered ? i + 1 : nil)
            }
            .buttonStyle(.plain)
            .trackMenu(t, ownPlaylistId: ownPlaylistId, onRemoved: { onRemoved?(t) })
        }
    }
}

/// Loads a value once and shows progress / errors.
struct Loader<T, Content: View>: View {
    let load: () async throws -> T
    @ViewBuilder let content: (T) -> Content
    @State private var value: T?
    @State private var error: String?

    var body: some View {
        Group {
            if let value {
                content(value)
            } else if let error {
                ContentUnavailableView {
                    Label("Couldn't load", systemImage: "wifi.exclamationmark")
                } description: { Text(error) } actions: {
                    Button("Retry") { self.error = nil; Task { await run() } }.buttonStyle(.glass)
                }
            } else {
                DetailSkeleton()
            }
        }
        .task { await run() }
    }

    private func run() async {
        guard value == nil else { return }
        do { value = try await load() } catch { self.error = error.localizedDescription }
    }
}

/// Official Hi-Res (colour) / CD / AAC (white) logos.
struct QualityLogo: View {
    let quality: Quality
    var height: CGFloat = 18

    var body: some View {
        let (name, scale): (String, CGFloat) = switch quality {
        case .hiFi: ("hires", 1.25)
        case .cd: ("cd", 0.95)
        case .aac: ("aac", 1.3)
        }
        if let ui = UIImage(named: name, in: .module, with: nil) {
            Image(uiImage: ui).resizable().interpolation(.high).scaledToFit().frame(height: height * scale)
        } else {
            Text(quality.label).font(.system(size: height * 0.6, weight: .heavy))
        }
    }

    @MainActor static func uiImage(_ q: Quality) -> UIImage? {
        let r = ImageRenderer(content: QualityLogo(quality: q, height: 18))
        r.scale = 3
        return r.uiImage?.withRenderingMode(.alwaysOriginal)
    }
}

/// Shimmering placeholder block.
struct Skeleton: View {
    var width: CGFloat? = nil
    var height: CGFloat = 14
    var radius: CGFloat = 6
    @State private var phase = false
    var body: some View {
        RoundedRectangle(cornerRadius: radius)
            .fill(Color.white.opacity(phase ? 0.14 : 0.06))
            .frame(width: width, height: height)
            .onAppear { withAnimation(.easeInOut(duration: 0.9).repeatForever()) { phase = true } }
    }
}

/// Skeleton page shown while an album/playlist/artist loads.
struct DetailSkeleton: View {
    var round = false
    var body: some View {
        ScrollView {
            VStack(spacing: 14) {
                Skeleton(width: 250, height: 250, radius: round ? 125 : 14).padding(.top, 20)
                Skeleton(width: 200, height: 22)
                Skeleton(width: 130, height: 14)
                HStack(spacing: 12) { Skeleton(height: 48, radius: 14); Skeleton(height: 48, radius: 14) }.padding(.vertical, 6)
                ForEach(0..<8, id: \.self) { _ in RowSkeleton() }
            }
            .padding(.horizontal, 16)
        }
        .scrollDisabled(true)
    }
}

struct RowSkeleton: View {
    var body: some View {
        HStack(spacing: 12) {
            Skeleton(width: 44, height: 44)
            VStack(alignment: .leading, spacing: 6) { Skeleton(width: 180, height: 12); Skeleton(width: 110, height: 10) }
            Spacer()
        }
    }
}

/// Quality menu shared by the player and Settings. The selection is a maximum.
struct QualityMenu<Label: View>: View {
    @Binding var selection: Quality
    @ViewBuilder let label: () -> Label

    var body: some View {
        Menu {
            Picker("Maximum quality", selection: $selection) {
                ForEach(Quality.allCases) { q in
                    Button { } label: {
                        if let img = QualityLogo.uiImage(q) { Image(uiImage: img) }
                        Text(q.label)
                        Text(q.detail)
                    }
                    .tag(q)
                }
            }
        } label: { label() }
    }
}

struct MediaRow: View {
    let title: String
    let subtitle: String
    let cover: String?
    var round = false

    var body: some View {
        HStack(spacing: 12) {
            Cover(url: cover, size: 52, radius: 6, circle: round)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).lineLimit(1)
                if !subtitle.isEmpty {
                    Text(subtitle).font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
                }
            }
        }
    }
}

/// Wrapping row layout for chips.
struct FlowLayout: Layout {
    var spacing: CGFloat = 8
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let w = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowH: CGFloat = 0, maxX: CGFloat = 0
        for s in subviews {
            let sz = s.sizeThatFits(.unspecified)
            if x + sz.width > w, x > 0 { x = 0; y += rowH + spacing; rowH = 0 }
            x += sz.width + spacing; rowH = max(rowH, sz.height); maxX = max(maxX, x)
        }
        return CGSize(width: min(maxX, w), height: y + rowH)
    }
    func placeSubviews(in b: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = b.minX, y = b.minY, rowH: CGFloat = 0
        for s in subviews {
            let sz = s.sizeThatFits(.unspecified)
            if x + sz.width > b.maxX, x > b.minX { x = b.minX; y += rowH + spacing; rowH = 0 }
            s.place(at: CGPoint(x: x, y: y), proposal: .unspecified)
            x += sz.width + spacing; rowH = max(rowH, sz.height)
        }
    }
}

struct Chip: View {
    let title: String
    var selected = false
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            Text(title).font(.subheadline.weight(.medium)).padding(.horizontal, 4)
        }
        .buttonStyle(GlassChipStyle(selected: selected))
    }
}

/// Real Liquid Glass chip: interactive glass, tinted blue when selected.
struct GlassChipStyle: ButtonStyle {
    let selected: Bool
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .padding(.horizontal, 12).padding(.vertical, 8)
            .foregroundStyle(.white)
            .glassEffect(selected ? .regular.tint(.blue).interactive() : .regular.interactive(), in: .capsule)
            .scaleEffect(configuration.isPressed ? 0.96 : 1)
    }
}

struct ErrorBanner: View {
    let text: String
    var body: some View {
        Label(text, systemImage: "exclamationmark.triangle.fill")
            .font(.footnote).foregroundStyle(.orange)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}
