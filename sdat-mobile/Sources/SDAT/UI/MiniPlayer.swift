import SwiftUI

/// Floating miniplayer. `.circle`: ring play button beside the expanded tab bar.
/// `.capsule`: cover, title and play beside the collapsed tab circle. `.wide`: full width on detail pages.
struct MiniPlayer: View {
    enum Style { case circle, capsule, wide }
    let style: Style
    /// Collapse progress shared with the tab bar (0 = circle, 1 = capsule).
    var p: CGFloat = 0
    @Environment(Player.self) private var player
    @Environment(Chrome.self) private var chrome

    private func resetPress() {
        if chrome.miniPressed { chrome.miniPressed = false }
    }

    var body: some View {
        let isCircle = style == .circle
        // ONE hierarchy; the parent sizes it from the shared progress, content opacities
        // are computed from that same value (capsule in late, circle out early).
        // Trailing-anchored: the circle's ring sits where the capsule's own ring ends up,
        // never centred in the growing capsule. Circle content is gone within the first 6%
        // of the morph (6%); capsule content only appears once the shape is 60% grown.
        // Fixed-size base + overlays: contents can never resize the shape. Before, the
        // ~130 pt capsule content made the flexible frame report 130 pt for a 62 pt circle,
        // pushing it off the right edge with the ring off-centre. The circle content is
        // exactly 62 pt wide and trailing-pinned, so in circle state it is centred.
        Color.clear
            .overlay(alignment: .trailing) {
                capsuleContent
                    .scaleEffect(style == .wide ? 1 : Morph.inScale(p), anchor: .trailing)
                    .opacity(style == .wide ? 1 : Morph.inAlpha(p))
                    .allowsHitTesting(!isCircle)
            }
            .overlay(alignment: .trailing) {
                circleContent
                    .scaleEffect(Morph.outScale(p))
                    .opacity(style == .wide ? 0 : Morph.outAlpha(p))
                    .allowsHitTesting(isCircle)
            }
        .clipShape(.capsule)
    }

    private var circleContent: some View {
                // Tap: open the full player. Long-press (PressRecognizer.miniHold): play/pause.
                PlayRing(size: 62)
                    .frame(width: 62)
                    .frame(maxHeight: .infinity)
                    // Generous hit area: 12 pt beyond the visible circle.
                    .contentShape(Circle().inset(by: -12))
                    // UIKit press recognizer instead of a SwiftUI DragGesture: it receives the
                    // touch directly (no SwiftUI gesture arbitration with the interactive glass /
                    // parent gestures) and times the hold with a run-loop timer, not a Task.
                    // One press is either a tap (released before miniHold) or a hold, never both.
                    .gesture(PressRecognizer(
                        hold: PressRecognizer.miniHold, slop: 40,
                        onDown: { chrome.miniPressed = true; gestureLog.notice("mini press began") },
                        onHold: {
                            UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                            Live.shared.userToggle()
                        },
                        onEnd: { kind in
                            gestureLog.notice("mini press ended \(String(describing: kind))")
                            chrome.miniPressed = false
                            if kind == .tap { chrome.showPlayer = true }
                        }))
                    .sensoryFeedback(.selection, trigger: player.isPlaying)
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
                        Button { Live.shared.userNext() } label: { Image(systemName: "forward.fill").frame(width: 36, height: 40) }
                            .buttonStyle(.plain)
                            .opacity(Live.shared.playbackLocked ? 0.4 : 1)
                    }
                }
                .padding(.leading, 9).padding(.trailing, 10)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .contentShape(.capsule)
                .onTapGesture { chrome.showPlayer = true }
    }

    private func playButton(size: CGFloat) -> some View {
        Button { Live.shared.userToggle() } label: { PlayRing(size: size) }
            .buttonStyle(.plain)
            .opacity(Live.shared.playbackLocked ? 0.4 : 1)
    }
}

/// Play/pause glyph inside the progress ring. Its own view so the 4 Hz position ticks
/// re-render only the ring, not the whole miniplayer.
private struct PlayRing: View {
    let size: CGFloat
    @Environment(Player.self) private var player

    var body: some View {
        // Loading: no spinner; the dashed ring itself spins instead.
        // Always mounted; hidden (not removed) while loading so nothing is re-inserted.
        Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
            .font(.system(size: size * 0.32))
            .contentTransition(.symbolEffect(.replace))
            .opacity(player.loading ? 0 : 1)
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
    var color: Color = .blue

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
                    // Dashes are removed under the played part (pattern itself never moves),
                    // so nothing can show through or over the blue arc.
                    .mask { Circle().trim(from: q, to: 1).stroke(lineWidth: lineWidth * 3) }
                    .zIndex(0)
                Circle().trim(from: 0, to: q)
                    .stroke(color, style: StrokeStyle(lineWidth: lineWidth, lineCap: .butt))
                    .opacity(q > 0 ? 1 : 0)
                    .zIndex(1)
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
    /// ONE stable view in both states (no if/else branch): switching branches swapped the
    /// ring's identity with an opacity transition on every track change, so the ring (and,
    /// with the icon removed while loading, the whole circle) blinked out and back.
    func body(content: Content) -> some View {
        TimelineView(.animation(paused: !on)) { tl in
            let t = tl.date.timeIntervalSinceReferenceDate
            content.rotationEffect(.degrees(on ? t.truncatingRemainder(dividingBy: 1) * 360 : 0))
        }
    }
}

/// Touch-down press recognizer: `onDown` immediately, `onHold` once after `hold` seconds
/// unless the finger moved more than `slop` or lifted; `onEnd` always (tap / hold / cancel).
struct PressRecognizer: UIGestureRecognizerRepresentable {
    enum End: Equatable { case tap, hold, cancel }
    /// Miniplayer circle: a press held this long is a hold (play/pause); shorter is a tap.
    static let miniHold: TimeInterval = 0.12
    let hold: TimeInterval
    let slop: CGFloat
    let onDown: () -> Void
    let onHold: () -> Void
    let onEnd: (End) -> Void

    final class Coordinator { var timer: Timer?; var fired = false; var cancelled = false; var start: CGPoint = .zero }
    func makeCoordinator(converter: CoordinateSpaceConverter) -> Coordinator { Coordinator() }

    func makeUIGestureRecognizer(context: Context) -> UILongPressGestureRecognizer {
        let g = UILongPressGestureRecognizer()
        g.minimumPressDuration = 0          // .began on touch-down; we time the hold ourselves
        g.allowableMovement = .greatestFiniteMagnitude
        g.cancelsTouchesInView = false
        g.delaysTouchesBegan = false
        return g
    }

    func handleUIGestureRecognizerAction(_ g: UILongPressGestureRecognizer, context: Context) {
        let c = context.coordinator
        switch g.state {
        case .began:
            c.timer?.invalidate(); c.fired = false; c.cancelled = false
            c.start = g.location(in: g.view)
            onDown()
            let t = Timer(timeInterval: hold, repeats: false) { _ in
                guard !c.cancelled, !c.fired else { return }
                c.fired = true
                MainActor.assumeIsolated { onHold() }
            }
            RunLoop.main.add(t, forMode: .common)
            c.timer = t
        case .changed:
            let p = g.location(in: g.view)
            if !c.cancelled, hypot(p.x - c.start.x, p.y - c.start.y) > slop {
                c.cancelled = true; c.timer?.invalidate()
            }
        case .ended, .cancelled, .failed:
            c.timer?.invalidate(); c.timer = nil
            let kind: End = c.fired ? .hold : (c.cancelled || g.state != .ended ? .cancel : .tap)
            onEnd(kind)
        default: break
        }
    }
}
