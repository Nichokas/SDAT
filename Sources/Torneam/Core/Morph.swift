import Foundation

/// Pure geometry + fade schedule of the bottom-chrome morph (tab bar <-> miniplayer).
/// Everything derives from ONE clock `t` (0...1, linear in time over `Morph.duration`)
/// and the direction, so widths come from a single budget and can never overlap.
enum Morph {
    static let duration = 0.6
    /// Shrinking shape finishes in the first half (0.3 s of 0.6 s).
    static let shrinkShare = 0.5
    static let round = 54.0      // collapsed tab button
    static let circle = 62.0     // miniplayer circle
    static let gap = 12.0

    /// Snappy ease-out with a slight overshoot (bounce); callers clamp it to the budget.
    static func fast(_ x: Double) -> Double {
        let u = min(max(x, 0), 1) - 1, s = 0.6
        return 1 + (s + 1) * u * u * u + s * u * u
    }
    /// Soft ease-in-out (smootherstep).
    static func soft(_ x: Double) -> Double {
        let u = min(max(x, 0), 1)
        return u * u * u * (u * (u * 6 - 15) + 10)
    }
    static func clamp01(_ x: Double) -> Double { min(max(x, 0), 1) }

    struct Frame: Equatable {
        var tabW: Double, miniW: Double
        /// Shape fractions: tab 0 = full bar, 1 = round; mini 0 = circle, 1 = capsule.
        var tabF: Double, miniF: Double
    }

    /// `w` = width inside the horizontal margins. `t` = collapse clock (0 expanded, 1 collapsed).
    /// `collapsing` = direction of the running/last transition.
    static func frame(width w: Double, t: Double, collapsing: Bool) -> Frame {
        let a = max(w - gap, round + circle)          // shared budget
        let tabMax = a - circle, miniMax = a - round
        let c = clamp01(t)
        var tabW: Double, miniW: Double
        if collapsing {
            // Tab shrinks fast; miniplayer grows soft, never past what the tab has freed.
            tabW = tabMax - (tabMax - round) * fast(c / shrinkShare)
            tabW = min(max(tabW, round), tabMax)
            miniW = circle + (miniMax - circle) * soft(c)
            miniW = min(max(miniW, circle), a - tabW)
        } else {
            let e = 1 - c                              // elapsed of the expand
            miniW = miniMax - (miniMax - circle) * fast(e / shrinkShare)
            miniW = min(max(miniW, circle), miniMax)
            tabW = round + (tabMax - round) * soft(e)
            tabW = min(max(tabW, round), a - miniW)
        }
        let span = max(tabMax - round, 1)
        return Frame(tabW: tabW, miniW: miniW,
                     tabF: clamp01((tabMax - tabW) / span),
                     miniF: clamp01((miniW - circle) / span))
    }

    /// Outgoing content: gone (and shrunk to 0.6) in the first 30% of its shape's change.
    static func outAlpha(_ f: Double) -> Double { clamp01(1 - f / 0.3) }
    /// Incoming content: only after the outgoing one is gone (from 45% to 100%).
    static func inAlpha(_ f: Double) -> Double { clamp01((f - 0.45) / 0.55) }
    static func outScale(_ f: Double) -> Double { 1 - 0.4 * clamp01(f / 0.3) }
    static func inScale(_ f: Double) -> Double { 0.6 + 0.4 * clamp01((f - 0.45) / 0.55) }
}
