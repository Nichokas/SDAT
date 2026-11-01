import SwiftUI

/// Floating miniplayer. `.circle`: ring play button beside the expanded tab bar.
/// `.capsule`: cover, title and play beside the collapsed tab circle. `.wide`: full width on detail pages.
struct MiniPlayer: View {
    enum Style { case circle, capsule, wide }
    let style: Style
    @Environment(Player.self) private var player
    @Environment(Chrome.self) private var chrome

    @State private var holdTask: Task<Void, Never>?
    @State private var held = false

    var body: some View {
        Group {
            if style == .circle {
                // Tap: play/pause. Long-press: open the full player.
                ringIcon(size: 62)
                    .contentShape(.circle)
                    // One gesture so a press is either a tap or a hold, never both:
                    // held ≥ 0.12 s opens the player; released sooner toggles play/pause.
                    .highPriorityGesture(DragGesture(minimumDistance: 0)
                        .onChanged { _ in
                            guard holdTask == nil else { return }
                            holdTask = Task { @MainActor in
                                try? await Task.sleep(for: .milliseconds(120))
                                guard !Task.isCancelled else { return }
                                held = true
                                UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                                chrome.showPlayer = true
                            }
                        }
                        .onEnded { _ in
                            holdTask?.cancel(); holdTask = nil
                            if !held { player.toggle() }
                            held = false
                        })
                    .sensoryFeedback(.selection, trigger: player.isPlaying)
                    .transition(Self.seqFade)
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
                .frame(height: style == .capsule ? 54 : 62)
                .frame(maxWidth: .infinity)
                .contentShape(.capsule)
                .onTapGesture { chrome.showPlayer = true }
                .transition(Self.seqFade)
            }
        }
    }

    /// Old content leaves fast, new content arrives late, so they never overlap mid-morph.
    private static let seqFade: AnyTransition = .asymmetric(
        insertion: .opacity.animation(.easeIn(duration: 0.2).delay(0.18)),
        removal: .opacity.animation(.easeOut(duration: 0.12)))

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
