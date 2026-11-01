import SwiftUI

/// Retro boot loader: a solid arc orbiting a thin ring, with three sharp squares
/// lighting up in sequence inside (1, 1-2, 1-2-3, off). White on black.
/// Reduce Motion: the arc stays still and the squares fade gently together.
struct BootLoaderView: View {
    var size: CGFloat = 96
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            TimelineView(.animation) { ctx in
                let t = ctx.date.timeIntervalSinceReferenceDate
                ZStack {
                    Circle().stroke(Color.white.opacity(0.18), lineWidth: 2)
                    Circle()
                        .trim(from: 0, to: 0.22)
                        .stroke(Color.white, style: StrokeStyle(lineWidth: 4, lineCap: .round))
                        .rotationEffect(.degrees(reduceMotion ? -90 : (t.truncatingRemainder(dividingBy: 1.2) / 1.2) * 360 - 90))
                    HStack(spacing: size * 0.08) {
                        ForEach(0..<3) { i in
                            Rectangle()
                                .fill(Color.white)
                                .frame(width: size * 0.1, height: size * 0.1)
                                .opacity(squareOpacity(i, t))
                        }
                    }
                }
                .frame(width: size, height: size)
            }
        }
        .accessibilityElement()
        .accessibilityLabel("Loading")
    }

    /// 4 steps of 0.3 s: 1, 1-2, 1-2-3, none.
    private func squareOpacity(_ i: Int, _ t: Double) -> Double {
        if reduceMotion { return 0.55 + 0.45 * (0.5 + 0.5 * sin(t * 1.5)) }
        let step = Int(t / 0.3) % 4
        return i < step ? 1 : 0
    }
}
