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
    /// Auto-resets when the gesture ends OR is cancelled, so no press/hold can get stuck.
    @GestureState private var touching = false

    private func resetPress() {
        gestureLog.notice("mini press reset")
        holdTask?.cancel(); holdTask = nil
        held = false; drifted = false
        if chrome.miniPressed { chrome.miniPressed = false }
    }

    var body: some View {
        let isCircle = style == .circle
        // ONE hierarchy; the parent sizes it from the shared progress, content opacities
        // are computed from that same value (capsule in late, circle out early).
        // Trailing-anchored: the circle's ring sits where the capsule's own ring ends up,
        // never centred in the growing capsule. Circle content is gone within the first 15%
        // of the morph; capsule content only appears once the shape is 60% grown.
        ZStack(alignment: .trailing) {
            capsuleContent
                .opacity(min(max((p - 0.6) / 0.4, 0), 1))
                .allowsHitTesting(!isCircle)
            circleContent
                .opacity(min(max(1 - p / 0.15, 0), 1))
                .allowsHitTesting(isCircle)
        }
    }

    private var circleContent: some View {
                // Tap: open the full player. Long-press (0.06 s hold): play/pause.
                ringIcon(size: 62)
                    .frame(width: 62)
                    .frame(maxHeight: .infinity)
                    // Generous hit area: 12 pt beyond the visible circle.
                    .contentShape(Circle().inset(by: -12))
                    // One gesture so a press is either a tap or a hold, never both:
                    // held ≥ 0.06 s toggles play/pause; released sooner opens the player.
                    .highPriorityGesture(DragGesture(minimumDistance: 0)
                        .updating($touching) { _, st, _ in st = true }
                        .onChanged { g in
                            // Finger drift up to 40 pt keeps the press alive.
                            if hypot(g.translation.width, g.translation.height) > 40 {
                                holdTask?.cancel(); holdTask = nil; drifted = true
                                if chrome.miniPressed { chrome.miniPressed = false }
                                return
                            }
                            if !chrome.miniPressed && !drifted { chrome.miniPressed = true }
                            guard holdTask == nil, !drifted else { return }
                            gestureLog.notice("mini press began")
                            holdTask = Task { @MainActor in
                                try? await Task.sleep(for: .milliseconds(60))
                                guard !Task.isCancelled else { return }
                                held = true
                                UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                                player.toggle()
                            }
                        }
                        .onEnded { _ in
                            holdTask?.cancel(); holdTask = nil
                            gestureLog.notice("mini press ended held=\(held) drifted=\(drifted)")
                            if !held && !drifted { chrome.showPlayer = true }
                            held = false; drifted = false
                            chrome.miniPressed = false
                        })
                    .sensoryFeedback(.selection, trigger: player.isPlaying)
                    .onChange(of: touching) { _, t in if !t { DispatchQueue.main.async { if !touching { resetPress() } } } }
                    .onDisappear { resetPress() }
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
                // Loading: no spinner; the dashed ring itself spins instead.
                if !player.loading {
                    Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
                        .font(.system(size: size * 0.32))
                        .contentTransition(.symbolEffect(.replace))
                }
            }
                .frame(width: size, height: size)
                .overlay { ProgressRing(progress: player.progress, spinning: player.loading).padding(size * 0.14) }
                .contentShape(.circle)
    }
}

/// Blue arc from 12 o'clock clockwise; the rest of the circle dashed white at 40%.
struct ProgressRing: View {
    let progress: Double
    var spinning = false
    var lineWidth: CGFloat = 2.5

    var body: some View {
        GeometryReader { g in
            // Dash period fitted so the circle holds a whole number of dashes starting at
            // 12 o'clock (the path start), so the pattern never shifts as progress moves
            let d = min(g.size.width, g.size.height)
            let circ = Double.pi * Double(max(d, 1))
            let n = max(Int((circ / 5.5).rounded()), 8)
            let period = circ / Double(n)
            // Gray dashed ring is static (pattern starts at 12 o'clock, the path start) and
            // independent of progress. Over it, a SOLID blue arc grows continuously from that
            // same start point (butt cap = flush with the first dash); 4 Hz ticks are
            // interpolated linearly. Hidden at 0% and while loading (ring spins instead).
            let q = spinning ? 0 : min(max(progress, 0), 1)
            ZStack {
                Circle().stroke(.white.opacity(0.4),
                                style: StrokeStyle(lineWidth: lineWidth * 0.8, lineCap: .butt,
                                                   dash: [period * 2.5 / 5.5, period * 3 / 5.5]))
                Circle().trim(from: 0, to: q)
                    .stroke(.blue, style: StrokeStyle(lineWidth: lineWidth, lineCap: .butt))
                    .opacity(q > 0 ? 1 : 0)
            }
            .frame(width: d, height: d)
            .animation(.linear(duration: 0.25), value: q)
        }
        .rotationEffect(.degrees(-90))
        // Time-derived rotation (1 turn/s) while loading: no repeating animation that could
        // get stuck; when loading ends the TimelineView is gone and the ring is at 12 o'clock.
        .modifier(Spin(on: spinning))
    }
}

private struct Spin: ViewModifier {
    let on: Bool
    func body(content: Content) -> some View {
        if on {
            TimelineView(.animation) { tl in
                let t = tl.date.timeIntervalSinceReferenceDate
                content.rotationEffect(.degrees(t.truncatingRemainder(dividingBy: 1) * 360))
            }
            .transition(.opacity)
        } else {
            content.transition(.opacity)
        }
    }
}
