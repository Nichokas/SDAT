import SwiftUI

/// Infinite 2D canvas of covers you can drag in any direction. Tiles wrap around,
/// so the canvas never ends. Shuffle starts the endless Discover queue.
struct DiscoverView: View {
    @Environment(Player.self) private var player
    @State private var mixes: [Mix] = []
    @State private var tracks: [Track] = []
    @State private var offset: CGSize = .zero
    @State private var dragStart: CGSize = .zero
    @State private var error: String?

    private let tile: CGFloat = 132
    private let gap: CGFloat = 10

    var body: some View {
        GeometryReader { geo in
            canvas(in: geo.size)
        }
        .ignoresSafeArea(edges: .bottom)
        .navigationTitle("Discover")
        .profileToolbar()
        .overlay(alignment: .bottomTrailing) {
            Button {
                player.playInfinite(mixes: mixes)
            } label: {
                Label("Shuffle", systemImage: "shuffle")
                    .font(.headline)
                    .padding(.horizontal, 20).padding(.vertical, 14)
            }
            .buttonStyle(.glass)
            .disabled(mixes.isEmpty)
            .padding(20)
        }
        .overlay {
            if let error, tracks.isEmpty {
                ContentUnavailableView("Couldn't load Discover", systemImage: "wifi.exclamationmark", description: Text(error))
            }
        }
        .task { await load() }
    }

    @ViewBuilder
    private func canvas(in size: CGSize) -> some View {
        let step = tile + gap
        let cols = Int(size.width / step) + 3
        let rows = Int(size.height / step) + 3
        let ox = offset.width.truncatingRemainder(dividingBy: step)
        let oy = offset.height.truncatingRemainder(dividingBy: step)
        let baseCol = Int((-offset.width / step).rounded(.down))
        let baseRow = Int((-offset.height / step).rounded(.down))

        ZStack(alignment: .topLeading) {
            ForEach(0..<rows, id: \.self) { r in
                ForEach(0..<cols, id: \.self) { c in
                    let track = item(col: baseCol + c - 1, row: baseRow + r - 1)
                    Button {
                        if let track { player.play([track], from: "Discover") }
                    } label: {
                        Cover(url: track?.album?.coverUrl, size: tile, radius: 14)
                    }
                    .buttonStyle(.plain)
                    .offset(
                        x: CGFloat(c - 1) * step + ox + (offset.width < 0 && ox != 0 ? step : 0),
                        y: CGFloat(r - 1) * step + oy + (offset.height < 0 && oy != 0 ? step : 0)
                            + (((baseCol + c) & 1) == 0 ? 0 : step / 2)
                    )
                }
            }
        }
        .frame(width: size.width, height: size.height, alignment: .topLeading)
        .clipped()
        .contentShape(.rect)
        .gesture(
            DragGesture()
                .onChanged { v in
                    offset = CGSize(width: dragStart.width + v.translation.width,
                                    height: dragStart.height + v.translation.height)
                }
                .onEnded { _ in dragStart = offset }
        )
    }

    /// Deterministic track for a grid cell so the canvas repeats seamlessly.
    private func item(col: Int, row: Int) -> Track? {
        guard !tracks.isEmpty else { return nil }
        let h = (col &* 73_856_093) ^ (row &* 19_349_663)
        return tracks[abs(h % tracks.count)]
    }

    private func load() async {
        guard tracks.isEmpty else { return }
        do {
            mixes = try await homeMixes()
            tracks = try await Shuffle(mixes: mixes).nextBatch(min: 60)
        } catch {
            self.error = error.localizedDescription
        }
    }
}
