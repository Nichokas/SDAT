import SwiftUI

/// First-sign-in loader: the miniplayer's dashed ring filling (white) with real warm-up
/// progress, and three sharp squares lighting up in sequence inside (none, 1, 1-2, 1-2-3).
/// Reduce Motion: the squares fade gently together instead of stepping.
struct BootLoaderView: View {
    let progress: Double
    var size: CGFloat = 96
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            ZStack {
                ProgressRing(progress: progress, lineWidth: 3, color: .white)
                TimelineView(.animation(minimumInterval: reduceMotion ? 1 / 30 : 0.1)) { ctx in
                    let t = ctx.date.timeIntervalSinceReferenceDate
                    HStack(spacing: size * 0.08) {
                        ForEach(0..<3) { i in
                            Rectangle().fill(Color.white)
                                .frame(width: size * 0.1, height: size * 0.1)
                                .opacity(squareOpacity(i, t))
                        }
                    }
                }
            }
            .frame(width: size, height: size)
        }
        .accessibilityElement()
        .accessibilityLabel("Loading")
        .accessibilityValue("\(Int(progress * 100)) percent")
    }

    /// Steps of 0.3 s: none, 1, 1-2, 1-2-3.
    private func squareOpacity(_ i: Int, _ t: Double) -> Double {
        if reduceMotion { return 0.55 + 0.45 * (0.5 + 0.5 * sin(t * 1.5)) }
        return i < Int(t / 0.3) % 4 ? 1 : 0
    }
}
