import SwiftUI

/// Floating miniplayer. `.circle`: ring play button beside the expanded tab bar.
/// `.capsule`: cover, title and play beside the collapsed tab circle. `.wide`: full width on detail pages.
struct MiniPlayer: View {
    enum Style { case circle, capsule, wide }
    let style: Style
    /// Collapse progress shared with the tab bar (0 = circle, 1 = capsule).
    var p: CGFloat = 0
    @State private var drifted = false
    @Environment(Player.self) private var player
    @Environment(Chrome.self) private var chrome

    @State private var holdTask: Task<Void, Never>?
    @State private var held = false

    var body: some View {
        let isCircle = style == .circle
        // ONE hierarchy; the parent sizes it from the shared progress, content opacities
        // are computed from that same value (capsule in late, circle out early).
        ZStack {
            capsuleContent
                .opacity(max(0, p * 2 - 1))
                .allowsHitTesting(!isCircle)
            circleContent
                .opacity(max(0, 1 - p * 2))
                .allowsHitTesting(isCircle)
        }
    }

    private var circleContent: some View {
                // Tap: play/pause. Long-press: open the full player.
                ringIcon(size: 62)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    // Generous hit area: 12 pt beyond the visible circle.
                    .contentShape(Circle().inset(by: -12))
                    // One gesture so a press is either a tap or a hold, never both:
                    // held ≥ 0.12 s opens the player; released sooner toggles play/pause.
                    .highPriorityGesture(DragGesture(minimumDistance: 0)
                        .onChanged { g in
                            // Finger drift up to 40 pt keeps the press alive.
                            if hypot(g.translation.width, g.translation.height) > 40 {
                                holdTask?.cancel(); drifted = true; chrome.miniPressed = false
                                return
                            }
                            if !chrome.miniPressed && !drifted { chrome.miniPressed = true }
                            guard holdTask == nil, !drifted else { return }
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
                            if !held && !drifted { player.toggle() }
                            held = false; drifted = false
                            chrome.miniPressed = false
                        })
                    .sensoryFeedback(.selection, trigger: player.isPlaying)
    }

    private var capsuleContent: some View {
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
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .contentShape(.capsule)
                .onTapGesture { chrome.showPlayer = true }
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
