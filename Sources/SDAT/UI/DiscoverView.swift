import SwiftUI
import UIKit

/// Discover: a pannable, endless 2D plane of covers (UIScrollView: 1:1 tracking, native
/// inertia, recycled tiles, cached + prefetched artwork). Shuffle starts the endless queue.
struct DiscoverView: View {
    @Environment(Player.self) private var player
    @State private var mixes: [Mix] = []
    @State private var tracks: [Track] = []
    @State private var error: String?

    var body: some View {
        ZStack(alignment: .top) {
            CoverPlane(tracks: tracks) { t in player.playDiscover(t, pool: tracks, mixes: mixes) }
                .ignoresSafeArea()
            // Smooth scrims so the title, buttons and bottom bar stay legible over bright covers.
            LinearGradient(stops: [
                .init(color: .black, location: 0), .init(color: .black.opacity(0.92), location: 0.35),
                .init(color: .black.opacity(0.55), location: 0.7), .init(color: .clear, location: 1),
            ], startPoint: .top, endPoint: .bottom)
            .frame(height: 170).ignoresSafeArea(edges: .top).allowsHitTesting(false)
            VStack {
                Spacer()
                LinearGradient(stops: [
                    .init(color: .clear, location: 0), .init(color: .black.opacity(0.5), location: 0.5), .init(color: .black.opacity(0.85), location: 1),
                ], startPoint: .top, endPoint: .bottom)
                .frame(height: 150)
            }
            .ignoresSafeArea().allowsHitTesting(false)
            header
            if let error, tracks.isEmpty {
                ContentUnavailableView {
                    Label("Couldn't load Discover", systemImage: "wifi.exclamationmark")
                } description: { Text(error) } actions: {
                    Button("Retry") { self.error = nil; Task { await load() } }.buttonStyle(.glass)
                }
                .frame(maxHeight: .infinity)
            } else if tracks.isEmpty {
                ProgressView().controlSize(.large).frame(maxHeight: .infinity)
            }
        }
        .toolbar(.hidden, for: .navigationBar)
        .task { await load() }
    }

    private var header: some View {
        HStack(alignment: .center) {
            Text("Discover").font(.system(size: 34, weight: .bold)).frame(height: 44)
            Spacer()
            GlassEffectContainer(spacing: 10) {
                HStack(spacing: 10) {
                    Button { player.playInfinite(mixes: mixes) } label: {
                        Image(systemName: "shuffle").font(.system(size: 17, weight: .semibold)).frame(width: 44, height: 44)
                    }
                    .disabled(mixes.isEmpty)
                    .glassEffect(.regular.interactive(), in: .circle)
                    ProfileButton()
                }
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 20)
        .padding(.top, 4)
        .padding(.bottom, 8)
        .contentShape(.rect)
    }

    private func load() async {
        if tracks.isEmpty { tracks = DiscoverCache.load() }
        let hadCache = !tracks.isEmpty
        do {
            mixes = try await homeMixes()
            player.resumeInfinite(mixes: mixes)
            let source = Shuffle(mixes: mixes)
            var pool: [Track] = []
            var ids = Set<String>(), albums = Set<String>(), covers = Set<String>()
            for _ in 0..<10 {
                for t in try await source.nextBatch(min: hadCache || !pool.isEmpty ? 80 : 30) {
                    guard let a = t.album, let c = a.coverUrl else { continue }
                    if ids.insert(t.id).inserted, albums.insert(a.id).inserted, covers.insert(c).inserted { pool.append(t) }
                }
                // First launch: show covers as soon as the first batch arrives.
                if !hadCache, tracks.isEmpty, !pool.isEmpty { tracks = pool }
                // First launch: show covers as soon as the first batch arrives.
                if !hadCache, tracks.isEmpty, !pool.isEmpty { tracks = pool }
                if pool.count >= 320 { break }
            }
            DiscoverCache.save(pool)
            // Keep the cached plane on screen this session; the fresh pool shows next launch,
            // unless there was nothing to show.
            if !hadCache { tracks = pool }
        } catch {
            if !hadCache { self.error = friendlyMessage(error) }
        }
    }
}

struct CoverPlane: UIViewRepresentable {
    let tracks: [Track]
    let onTap: (Track) -> Void

    func makeUIView(context: Context) -> PlaneScrollView {
        let v = PlaneScrollView()
        v.onTap = onTap
        return v
    }

    func updateUIView(_ v: PlaneScrollView, context: Context) {
        v.onTap = onTap
        if v.tracks.map(\.id) != tracks.map(\.id) { v.tracks = tracks }
    }
}

final class PlaneScrollView: UIScrollView, UIScrollViewDelegate, UIContextMenuInteractionDelegate {
    var onTap: ((Track) -> Void)?
    var tracks: [Track] = [] { didSet { visible.values.forEach { $0.removeFromSuperview() }; visible = [:]; setNeedsLayout() } }

    private let gap: CGFloat = 12
    private let step: CGFloat = 150
    private let span: CGFloat = 4_000_000
    private struct Cell: Hashable { let c: Int, r: Int }
    private var visible: [Cell: UIImageView] = [:]
    private var pool: [UIImageView] = []
    private var prefetched = Set<Cell>()

    override init(frame: CGRect) {
        super.init(frame: frame)
        showsVerticalScrollIndicator = false
        showsHorizontalScrollIndicator = false
        contentInsetAdjustmentBehavior = .never
        decelerationRate = .normal
        backgroundColor = .black
        contentSize = CGSize(width: span, height: span)
        contentOffset = CGPoint(x: span / 2, y: span / 2)
        delaysContentTouches = false
        addGestureRecognizer(UITapGestureRecognizer(target: self, action: #selector(tapped(_:))))
        addInteraction(UIContextMenuInteraction(delegate: self))
    }

    // MARK: long-press menu (system Liquid Glass context menu, tile lifted as preview)

    private var menuCell: Cell?

    private func cell(at p: CGPoint) -> Cell? {
        visible.first(where: { $0.value.frame.contains(p) })?.key
    }

    func contextMenuInteraction(_ i: UIContextMenuInteraction, configurationForMenuAtLocation p: CGPoint) -> UIContextMenuConfiguration? {
        let onScreen = convert(p, to: window)
        let h = window?.bounds.height ?? bounds.height
        guard onScreen.y > (window?.safeAreaInsets.top ?? 0) + 64,
              onScreen.y < h - (window?.safeAreaInsets.bottom ?? 0) - 90,
              let c = cell(at: p), let t = track(c) else { return nil }
        menuCell = c
        return UIContextMenuConfiguration(identifier: nil, previewProvider: nil) { _ in Self.menu(for: t) }
    }

    func contextMenuInteraction(_ i: UIContextMenuInteraction, configuration: UIContextMenuConfiguration,
                                highlightPreviewForItemWithIdentifier id: NSCopying) -> UITargetedPreview? {
        guard let c = menuCell, let v = visible[c] else { return nil }
        let params = UIPreviewParameters()
        params.visiblePath = UIBezierPath(roundedRect: v.bounds, cornerRadius: 10)
        return UITargetedPreview(view: v, parameters: params)
    }

    func contextMenuInteraction(_ i: UIContextMenuInteraction, willEndFor configuration: UIContextMenuConfiguration,
                                animator: UIContextMenuInteractionAnimating?) {
        menuCell = nil
    }

    /// Same actions as the app's track "…" menu (TrackMenu), in UIKit form.
    @MainActor static func menu(for t: Track) -> UIMenu {
        let player = Player.shared, chrome = Chrome.shared, dl = Downloads.shared
        let fav = Favs.shared.known(t.id) ?? false
        var main: [UIMenuElement] = [
            UIAction(title: "Play Next", image: UIImage(systemName: "text.line.first.and.arrowtriangle.forward")) { _ in
                player.playNext(t); chrome.flash("Playing next")
            },
            UIAction(title: "Add to Queue", image: UIImage(systemName: "text.line.last.and.arrowtriangle.forward")) { _ in
                player.addToQueue(t); chrome.flash("Added to queue")
            },
            UIAction(title: "Add to Playlist…", image: UIImage(systemName: "text.badge.plus")) { _ in
                chrome.playlistPickerTrack = t
            },
        ]
        switch dl.state(t.id) {
        case .done:
            main.append(UIAction(title: "Remove Download", image: UIImage(systemName: "trash"), attributes: .destructive) { _ in dl.remove([t]) })
        case .queued, .downloading:
            main.append(UIAction(title: "Cancel Download", image: UIImage(systemName: "xmark.circle")) { _ in dl.cancel(t.id) })
        default:
            main.append(UIAction(title: "Download", image: UIImage(systemName: "arrow.down.circle")) { _ in
                dl.download([t]); chrome.flash("Downloading")
            })
        }
        let favItem = UIAction(title: fav ? "Remove from Favourites" : "Add to Favourites",
                               image: UIImage(systemName: fav ? "star.slash" : "star")) { _ in Favs.shared.toggle(t.id) }
        var nav: [UIMenuElement] = []
        if let a = t.album {
            nav.append(UIAction(title: "Go to Album", image: UIImage(systemName: "square.stack")) { _ in chrome.push(.album(a.id)) })
        }
        if let ar = t.artists.first {
            nav.append(UIAction(title: "Go to Artist", image: UIImage(systemName: "music.microphone")) { _ in chrome.push(.artist(ar.id)) })
        }
        return UIMenu(title: "\(t.displayTitle) · \(t.artistLine)", children: [
            UIMenu(options: .displayInline, children: [favItem]),
            UIMenu(options: .displayInline, children: main),
            UIMenu(options: .displayInline, children: nav),
        ])
    }

    required init?(coder: NSCoder) { fatalError() }

    private var block: (Int, Int) {
        let n = tracks.count
        let rows = max(12, Int((Double(n) / 1.4).squareRoot()))
        return (max(1, n / rows), rows)
    }

    private func track(_ cell: Cell) -> Track? {
        guard !tracks.isEmpty else { return nil }
        // The pool is laid out as one C×R block that tiles the plane: every cell in a block is a
        // different cover, and a repeat is C columns or R rows away (both larger than the screen).
        let (cols, rows) = block
        let c = ((cell.c % cols) + cols) % cols, r = ((cell.r % rows) + rows) % rows
        return tracks[min(r * cols + c, tracks.count - 1)]
    }

    /// Rows of flat covers: each row has one size, sizes vary between rows, and every row
    /// is shifted horizontally (staggered). The pattern repeats every `sizes.count` rows.
    private let sizes: [CGFloat] = [96, 140, 118, 100, 132, 150, 108]
    private var period: CGFloat { sizes.reduce(0) { $0 + $1 + gap } }

    private func rowSize(_ r: Int) -> CGFloat { sizes[((r % sizes.count) + sizes.count) % sizes.count] }

    private func rowY(_ r: Int) -> CGFloat {
        let n = sizes.count
        let k = Int(floor(Double(r) / Double(n)))
        var y = CGFloat(k) * period
        for i in 0..<(r - k * n) { y += sizes[i] + gap }
        return y
    }

    private func rowOffset(_ r: Int) -> CGFloat {
        let step = rowSize(r) + gap
        let h = CGFloat(UInt(bitPattern: r &* 2_654_435_761) % 1000) / 1000
        return h * step
    }

    private func frame(for cell: Cell) -> CGRect {
        let size = rowSize(cell.r)
        return CGRect(x: CGFloat(cell.c) * (size + gap) - rowOffset(cell.r), y: rowY(cell.r), width: size, height: size)
    }

    private func cells(in rect: CGRect) -> [Cell] {
        var out: [Cell] = []
        let n = sizes.count
        var r = Int(floor(Double(rect.minY / period))) * n - 1
        while rowY(r) <= rect.maxY {
            let size = rowSize(r), step = size + gap, off = rowOffset(r)
            if rowY(r) + size >= rect.minY {
                let c0 = Int(floor((rect.minX + off - size) / step)), c1 = Int(floor((rect.maxX + off) / step))
                for c in c0...max(c0, c1) { out.append(Cell(c: c, r: r)) }
            }
            r += 1
        }
        return out
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        guard !tracks.isEmpty else { return }
        let vis = bounds.insetBy(dx: -step * 0.5, dy: -step * 0.5)
        let needed = Set(cells(in: vis).filter { frame(for: $0).intersects(vis) })
        for (cell, v) in visible where !needed.contains(cell) {
            v.removeFromSuperview(); v.image = nil; pool.append(v); visible[cell] = nil
        }
        for cell in needed where visible[cell] == nil {
            let v = pool.popLast() ?? makeTile()
            v.frame = frame(for: cell)
            if let small = track(cell)?.album?.coverUrl {
                let url = CoverURL.sized(small, points: rowSize(cell.r))
                if let img = ImageStore.shared.cached(url) { v.image = img } else {
                    v.image = nil
                    Task { @MainActor [weak self, weak v] in
                        let img = await ImageStore.shared.image(url)
                        guard let self, let v, self.visible[cell] === v else { return }
                        UIView.transition(with: v, duration: 0.15, options: .transitionCrossDissolve) { v.image = img }
                    }
                }
            }
            addSubview(v)
            visible[cell] = v
        }
        // Warm a ring around the viewport (in the direction of travel too) so tiles never pop in.
        let v = panGestureRecognizer.velocity(in: self)
        let ahead = bounds.offsetBy(dx: -v.x * 0.35, dy: -v.y * 0.35).union(bounds).insetBy(dx: -step * 2, dy: -step * 2)
        for cell in cells(in: ahead) where !prefetched.contains(cell) {
            prefetched.insert(cell)
            if let url = track(cell)?.album?.coverUrl { ImageStore.shared.prefetch(CoverURL.sized(url, points: rowSize(cell.r))) }
        }
        if prefetched.count > 4000 { prefetched.removeAll() }
    }

    private func makeTile() -> UIImageView {
        let v = UIImageView()
        v.contentMode = .scaleAspectFill
        v.clipsToBounds = true
        v.backgroundColor = UIColor(white: 1, alpha: 0.07)
        return v
    }

    @objc private func tapped(_ g: UITapGestureRecognizer) {
        let p = g.location(in: self)
        // Ignore taps under the header (title + glass buttons) and the tab bar.
        let onScreen = g.location(in: window)
        let h = window?.bounds.height ?? bounds.height
        let top = (window?.safeAreaInsets.top ?? 0) + 64
        let bottom = (window?.safeAreaInsets.bottom ?? 0) + 90
        guard onScreen.y > top, onScreen.y < h - bottom else { return }
        guard let (cell, _) = visible.first(where: { $0.value.frame.contains(p) }), let t = track(cell) else { return }
        UIImpactFeedbackGenerator(style: .light).impactOccurred()
        onTap?(t)
    }
}

/// Last Discover pool on disk, so the plane appears instantly on launch.
enum DiscoverCache {
    struct T: Codable {
        var id, title, version: String
        var dur: UInt32
        var explicit, streamable: Bool
        var artists: [[String]]
        var albumId, albumTitle: String?
        var cover, coverLarge: String?
    }
    private static var url: URL {
        FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent("discover.json")
    }
    static func save(_ tracks: [Track]) {
        try? JSONEncoder().encode(encode(tracks)).write(to: url, options: .atomic)
    }
    static func encode(_ tracks: [Track]) -> [T] {
        tracks.map { t in
            T(id: t.id, title: t.title, version: t.version, dur: t.durationSecs, explicit: t.explicit, streamable: t.streamable,
              artists: t.artists.map { [$0.id, $0.name] }, albumId: t.album?.id, albumTitle: t.album?.title,
              cover: t.album?.coverUrl, coverLarge: t.album?.coverUrlLarge)
        }
    }
    static func load() -> [Track] {
        guard let d = try? Data(contentsOf: url), let list = try? JSONDecoder().decode([T].self, from: d) else { return [] }
        return decode(list)
    }
    static func decode(_ list: [T]) -> [Track] {
        list.map { c in
            Track(id: c.id, title: c.title, version: c.version, durationSecs: c.dur, explicit: c.explicit, streamable: c.streamable,
                  artists: c.artists.map { ArtistRef(id: $0[0], name: $0.count > 1 ? $0[1] : "", pictureUrl: nil) },
                  album: c.albumId.map { AlbumRef(id: $0, title: c.albumTitle ?? "", coverUrl: c.cover, coverUrlLarge: c.coverLarge) })
        }
    }
}
