import SwiftUI
import UIKit

/// Square cover served from the shared image cache (no flash when cached).
struct Cover: View {
    let url: String?
    var size: CGFloat? = nil
    var radius: CGFloat = 8
    var circle = false

    var body: some View {
        CachedImage(url: url, kind: circle ? .artist : .album)
            .frame(width: size, height: size)
            .aspectRatio(1, contentMode: .fit)
            .clipShape(circle ? AnyShape(.circle) : AnyShape(.rect(cornerRadius: radius)))
    }
}

/// Picks the TIDAL image size for a display size: covers come as /<uuid>/<N>x<N>.jpg.
enum CoverURL {
    /// TIDAL serves different square sizes per resource type.
    enum Kind { case album, artist, profile
        var sizes: [Int] {
            switch self {
            case .album: [80, 160, 320, 640, 1280]
            case .artist: [160, 320, 480, 750]
            case .profile: [320]
            }
        }
    }
    static func sized(_ url: String, points: CGFloat, kind: Kind = .album) -> String {
        // Parsed by hand: this runs for every cover in every row render, and a regex
        // is compiled anew on each call.
        guard url.contains("resources.tidal.com/images/"), points > 0,
              let slash = url.lastIndex(of: "/"), let dot = url.lastIndex(of: "."), slash < dot else { return url }
        let ext = url[url.index(after: dot)...]
        let dims = url[url.index(after: slash)..<dot].split(separator: "x", omittingEmptySubsequences: false)
        guard ext == "jpg" || ext == "png", dims.count == 2,
              dims.allSatisfy({ !$0.isEmpty && $0.allSatisfy(\.isASCII) && $0.allSatisfy(\.isNumber) }) else { return url }
        let px = Int(points * 3)
        let sizes = ext == "png" ? Kind.profile.sizes : kind.sizes
        let size = sizes.first { $0 >= px } ?? sizes.last!
        return "\(url[...slash])\(size)x\(size).\(ext)"
    }
}

/// Image sized for its frame (× screen scale). Shows any lower-res copy already in memory
/// while the right size loads, then swaps without flashing.
struct CachedImage: View {
    let url: String?
    var kind: CoverURL.Kind = .album
    @State private var image: UIImage?
    @State private var width: CGFloat = 0

    private var target: String? { url.map { CoverURL.sized($0, points: width, kind: kind) } }

    var body: some View {
        Rectangle().fill(Color.white.opacity(0.08))
            .overlay {
                if let img = image ?? target.flatMap(ImageStore.shared.cached) ?? url.flatMap(ImageStore.shared.cached) {
                    Image(uiImage: img).resizable().interpolation(.high).scaledToFill()
                } else {
                    Image(systemName: "music.note").foregroundStyle(.secondary)
                }
            }
            .clipped()
            .onGeometryChange(for: CGFloat.self) { max($0.size.width, $0.size.height) } action: { w in
                // Only grow: avoid re-requesting while a view animates smaller.
                if w > width { width = w }
            }
            .task(id: target) {
                guard let t = target, width > 0 else { if url == nil { image = nil }; return }
                // Downloaded artwork first: no network and works offline. Through the store, so
                // the file is decoded off the main thread and only once, at the size shown.
                let px = kind.sizes.first { $0 >= Int(width * 3) } ?? kind.sizes.last!
                // A cancelled load keeps running past its awaits; it must not overwrite the
                // artwork of the target that replaced it.
                if let u = url, let f = Downloads.shared.coverFile(forRemote: u),
                   let img = await ImageStore.shared.image(f.absoluteString, maxPixel: px) {
                    if !Task.isCancelled { image = img }
                    return
                }
                if Task.isCancelled { return }
                if let c = ImageStore.shared.cached(t) { image = c; return }
                if image == nil, let u = url { image = ImageStore.shared.cached(u) }
                var loaded = await ImageStore.shared.image(t)
                if Task.isCancelled { return }
                // Size missing on the server: fall back to the original URL.
                if loaded == nil, let u = url, u != t { loaded = await ImageStore.shared.image(u) }
                // Offline, and the download index was not loaded yet on the first check above.
                if loaded == nil, !Task.isCancelled, let u = url,
                   let f = Downloads.shared.coverFile(forRemote: u) {
                    loaded = await ImageStore.shared.image(f.absoluteString, maxPixel: px)
                }
                if let loaded, !Task.isCancelled { image = loaded }
            }
    }
}

/// Animated bars shown on the playing row.
struct Equalizer: View {
    let playing: Bool
    var body: some View {
        TimelineView(.animation(paused: !playing)) { ctx in
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
    var coverSize: CGFloat = 44
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
                Cover(url: track.album?.coverUrl, size: coverSize, radius: 6)
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
            DownloadBadge(id: track.id)
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
            if !Live.shared.userPlayNext([track]) { player.playNext(track); Chrome.shared.flash("Playing next") }
        }
        Button("Add to Queue", systemImage: "text.line.last.and.arrowtriangle.forward") {
            if !Live.shared.userEnqueue([track]) { player.addToQueue(track); Chrome.shared.flash("Added to queue") }
        }
        Button("Add to Playlist…", systemImage: "text.badge.plus") { Chrome.shared.playlistPickerTrack = track }
        DownloadButton(tracks: [track])
        if let pid = ownPlaylistId {
            Button("Remove from Playlist", systemImage: "minus.circle", role: .destructive) {
                Task {
                    do {
                        try await removeFromPlaylist(playlistId: pid, trackId: track.id)
                        onRemoved?()
                        Chrome.shared.flash("Removed from playlist")
                    } catch { Chrome.shared.flash(friendlyMessage(error)) }
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
        // A range, not an enumerated copy: a long playlist grows page by page, and each
        // page would rebuild an array of every row.
        ForEach(tracks.indices, id: \.self) { i in
            let t = tracks[i]
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
        do { value = try await load() } catch { self.error = friendlyMessage(error) }
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

/// Download / cancel / remove for one or more tracks (track menu, album & playlist menus).
struct DownloadButton: View {
    let tracks: [Track]
    var body: some View {
        let dl = Downloads.shared
        let ids = tracks.map(\.id)
        let states = ids.map(dl.state)
        let done = !ids.isEmpty && ids.allSatisfy(dl.isDownloaded)
        let busy = states.contains { if case .downloading = $0 { true } else { $0 == .queued } }
        let failed: String? = states.lazy.compactMap { if case .failed(let m) = $0 { m } else { nil } }.first
        if busy {
            if tracks.count == 1, case .downloading(let p) = states[0] {
                Label("Downloading… \(Int(p * 100))%", systemImage: "arrow.down.circle.dotted")
            } else {
                Label("Downloading…", systemImage: "arrow.down.circle.dotted")
            }
            Button("Cancel Download", systemImage: "xmark.circle") { ids.forEach(dl.cancel) }
        } else if done {
            Label(tracks.count > 1 ? "All Downloaded" : "Downloaded", systemImage: "arrow.down.circle.fill")
        } else {
            Button(failed != nil ? "Retry Download" : (tracks.count > 1 ? "Download All" : "Download"),
                   systemImage: failed != nil ? "arrow.clockwise.circle" : "arrow.down.circle") {
                dl.download(tracks); Chrome.shared.flash("Downloading")
            }
            if let failed { Text("Download failed: \(failed)") }
        }
        if ids.contains(where: dl.isDownloaded) {
            Button(tracks.count > 1 ? "Remove Downloads" : "Remove Download", systemImage: "trash", role: .destructive) {
                dl.remove(tracks)
            }
        }
    }
}

/// Small state mark on track rows: progress ring while downloading, filled arrow when saved.
struct DownloadBadge: View {
    let id: String
    var body: some View {
        switch Downloads.shared.state(id) {
        case .done:
            Image(systemName: "arrow.down.circle.fill").font(.caption).foregroundStyle(.secondary)
        case .downloading(let p):
            DownloadRing(value: p).frame(width: 13, height: 13)
        case .queued:
            DownloadRing(value: 0).frame(width: 13, height: 13)
        case .failed:
            Image(systemName: "exclamationmark.circle").font(.caption).foregroundStyle(.orange)
        case .none:
            EmptyView()
        }
    }
}

struct DownloadRing: View {
    let value: Double
    var body: some View {
        ZStack {
            Circle().stroke(Color.white.opacity(0.2), lineWidth: 2)
            Circle().trim(from: 0, to: max(0.02, value)).stroke(Color.white, style: .init(lineWidth: 2, lineCap: .round))
                .rotationEffect(.degrees(-90))
        }
        .animation(.linear(duration: 0.2), value: value)
    }
}
