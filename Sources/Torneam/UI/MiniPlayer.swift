import SwiftUI

/// Floating miniplayer. `.circle`: ring play button beside the expanded tab bar.
/// `.capsule`: cover, title and play beside the collapsed tab circle. `.wide`: full width on detail pages.
struct MiniPlayer: View {
    enum Style { case circle, capsule, wide }
    let style: Style
    @Environment(Player.self) private var player
    @Environment(Chrome.self) private var chrome

    var body: some View {
        Group {
            if style == .circle {
                // Tap: play/pause. Long-press: open the full player.
                ringIcon(size: 62)
                    .contentShape(.circle)
                    .glassEffect(.regular.interactive(), in: .circle)
                    .onTapGesture { player.toggle() }
                    .onLongPressGesture(minimumDuration: 0.25) {
                        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                        chrome.showPlayer = true
                    }
                    .sensoryFeedback(.selection, trigger: player.isPlaying)
            } else {
                HStack(spacing: 10) {
                    Cover(url: player.current?.album?.coverUrl, size: 38, radius: 8)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(player.current?.displayTitle ?? "").font(.subheadline.weight(.semibold)).lineLimit(1)
                        Text(player.current?.artistLine ?? "").font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }
                    Spacer(minLength: 0)
                    playButton(size: 40)
                    if style == .wide {
                        Button { player.next() } label: { Image(systemName: "forward.fill").frame(width: 36, height: 40) }
                            .buttonStyle(.plain)
                    }
                }
                .padding(.leading, 9).padding(.trailing, 10)
                .frame(height: 62)
                .frame(maxWidth: .infinity)
                .contentShape(.capsule)
                .onTapGesture { chrome.showPlayer = true }
                .glassEffect(.regular.interactive(), in: .capsule)
            }
        }
    }

    private func playButton(size: CGFloat) -> some View {
        Button { player.toggle() } label: { ringIcon(size: size) }
            .buttonStyle(.plain)
    }

    private func ringIcon(size: CGFloat) -> some View {
            Group {
                if player.loading { ProgressView().controlSize(.small) } else {
                    Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
                        .font(.system(size: size * 0.32))
                        .contentTransition(.symbolEffect(.replace))
                }
            }
                .frame(width: size, height: size)
                .overlay { ProgressRing(progress: player.progress).padding(size * 0.14) }
                .contentShape(.circle)
    }
}

/// Blue arc from 12 o'clock clockwise; the rest of the circle dashed white at 40%.
struct ProgressRing: View {
    let progress: Double
    var lineWidth: CGFloat = 2.5

    var body: some View {
        let p = min(max(progress, 0), 1)
        ZStack {
            if p < 0.999 {
                Circle()
                    .trim(from: p, to: 1)
                    .stroke(.white.opacity(0.4), style: StrokeStyle(lineWidth: lineWidth * 0.8, lineCap: .butt, dash: [2.5, 3]))
            }
            if p > 0.002 {
                Circle()
                    .trim(from: 0, to: p)
                    .stroke(.blue, style: StrokeStyle(lineWidth: lineWidth, lineCap: .butt))
            }
        }
        .rotationEffect(.degrees(-90))
        .animation(.linear(duration: 0.25), value: p)
    }
}
