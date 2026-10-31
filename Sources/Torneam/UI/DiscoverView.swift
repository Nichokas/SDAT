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
            CoverPlane(tracks: tracks) { t in player.play([t], from: "Discover") }
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
    }

    private func load() async {
        if tracks.isEmpty { tracks = DiscoverCache.load() }
        let hadCache = !tracks.isEmpty
        do {
            mixes = try await homeMixes()
            let source = Shuffle(mixes: mixes)
            var pool: [Track] = []
            var ids = Set<String>(), albums = Set<String>(), covers = Set<String>()
            for _ in 0..<3 {
                for t in try await source.nextBatch(min: 80) {
                    guard let a = t.album, let c = a.coverUrl else { continue }
                    if ids.insert(t.id).inserted, albums.insert(a.id).inserted, covers.insert(c).inserted { pool.append(t) }
                }
                if pool.count >= 70 { break }
            }
            DiscoverCache.save(pool)
            // Keep the cached plane on screen this session; the fresh pool shows next launch,
            // unless there was nothing to show.
            if !hadCache { tracks = pool }
        } catch {
            if !hadCache { self.error = error.localizedDescription }
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

final class PlaneScrollView: UIScrollView, UIScrollViewDelegate {
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
    }

    required init?(coder: NSCoder) { fatalError() }

    private func track(_ cell: Cell) -> Track? {
        guard !tracks.isEmpty else { return nil }
        // Each row walks the pool consecutively, and consecutive rows start N/rows apart, so a
        // cover only repeats after the whole pool (within a row) or a full period of rows.
        let n = tracks.count
        let k = max(n / sizes.count, 1)
        let i = ((cell.c + cell.r * k) % n + n) % n
        return tracks[i]
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
            if let url = track(cell)?.album?.coverUrl {
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
            if let url = track(cell)?.album?.coverUrl { ImageStore.shared.prefetch(url) }
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
        let list = tracks.map { t in
            T(id: t.id, title: t.title, version: t.version, dur: t.durationSecs, explicit: t.explicit, streamable: t.streamable,
              artists: t.artists.map { [$0.id, $0.name] }, albumId: t.album?.id, albumTitle: t.album?.title,
              cover: t.album?.coverUrl, coverLarge: t.album?.coverUrlLarge)
        }
        try? JSONEncoder().encode(list).write(to: url, options: .atomic)
    }
    static func load() -> [Track] {
        guard let d = try? Data(contentsOf: url), let list = try? JSONDecoder().decode([T].self, from: d) else { return [] }
        return list.map { c in
            Track(id: c.id, title: c.title, version: c.version, durationSecs: c.dur, explicit: c.explicit, streamable: c.streamable,
                  artists: c.artists.map { ArtistRef(id: $0[0], name: $0.count > 1 ? $0[1] : "", pictureUrl: nil) },
                  album: c.albumId.map { AlbumRef(id: $0, title: c.albumTitle ?? "", coverUrl: c.cover, coverUrlLarge: c.coverLarge) })
        }
    }
}
