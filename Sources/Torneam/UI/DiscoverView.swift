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
            LinearGradient(colors: [.black.opacity(0.75), .clear], startPoint: .top, endPoint: .bottom)
                .frame(height: 170).ignoresSafeArea().allowsHitTesting(false)
            header
            if let error, tracks.isEmpty {
                ContentUnavailableView {
                    Label("Couldn't load Discover", systemImage: "wifi.exclamationmark")
                } description: { Text(error) } actions: {
                    Button("Retry") { self.error = nil; Task { await load() } }
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
            Text("Discover").font(.system(size: 34, weight: .bold))
            Spacer()
            GlassEffectContainer(spacing: 10) {
                HStack(spacing: 10) {
                    Button { player.playInfinite(mixes: mixes) } label: {
                        Image(systemName: "shuffle").font(.system(size: 17, weight: .semibold)).frame(width: 44, height: 44)
                    }
                    .disabled(mixes.isEmpty)
                    .glassEffect(.regular.interactive(), in: .circle)
                    Button { Chrome.shared.push(.settings) } label: {
                        Image(systemName: "person.crop.circle").font(.system(size: 20)).frame(width: 44, height: 44)
                    }
                    .glassEffect(.regular.interactive(), in: .circle)
                }
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 20)
        .padding(.top, 4)
    }

    private func load() async {
        guard tracks.isEmpty else { return }
        do {
            mixes = try await homeMixes()
            let first = try await Shuffle(mixes: mixes).nextBatch(min: 80)
            var seen = Set<String>()
            tracks = first.filter { $0.album?.coverUrl != nil && seen.insert($0.album!.coverUrl!).inserted }
        } catch {
            self.error = error.localizedDescription
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

    private let tile: CGFloat = 148
    private let gap: CGFloat = 12
    private var step: CGFloat { tile + gap }
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
        // Rows shift by column so neighbours differ; the pattern repeats seamlessly.
        let h = (cell.c &* 73_856_093) ^ (cell.r &* 19_349_663)
        return tracks[Int(UInt(bitPattern: h) % UInt(tracks.count))]
    }

    private func frame(for cell: Cell) -> CGRect {
        let stagger = (cell.c & 1) == 0 ? 0 : step / 2
        return CGRect(x: CGFloat(cell.c) * step, y: CGFloat(cell.r) * step + stagger, width: tile, height: tile)
    }

    private func cells(in rect: CGRect) -> [Cell] {
        let c0 = Int(floor(rect.minX / step)), c1 = Int(floor(rect.maxX / step))
        let r0 = Int(floor((rect.minY - step / 2) / step)), r1 = Int(floor(rect.maxY / step))
        var out: [Cell] = []
        out.reserveCapacity((c1 - c0 + 1) * (r1 - r0 + 1))
        for c in c0...c1 { for r in r0...r1 { out.append(Cell(c: c, r: r)) } }
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
        v.layer.cornerRadius = 14
        v.layer.cornerCurve = .continuous
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
