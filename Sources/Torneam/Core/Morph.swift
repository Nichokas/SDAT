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
    /// The two widths are complementary on ONE budget: tabW + gap + miniW == w at every t,
    /// so the HStack never has slack to re-centre (shapes drifting / big gaps) nor overflow.
    /// Feel: the shared fraction leaves fast (the shrinking shape snaps away) and lands
    /// softly (the growing shape spreads into the freed space).
    static func frame(width w: Double, t: Double, collapsing: Bool) -> Frame {
        let a = max(w - gap, round + circle)
        let tabMax = a - circle
        let c = clamp01(t)
        let g = collapsing ? ease(c) : 1 - ease(1 - c)
        let tabW = tabMax - (tabMax - round) * g
        return Frame(tabW: tabW, miniW: a - tabW, tabF: g, miniF: g)
    }

    /// Fast start, soft landing (blend of the snappy and soft curves, monotonic, no overshoot).
    static func ease(_ x: Double) -> Double {
        let u = clamp01(x)
        return 0.5 * (1 - pow(1 - u, 3)) + 0.5 * soft(u)
    }

    /// Short, crisp crossfade around the midpoint: content is fully shown outside 0.38...0.62,
    /// outgoing is gone exactly when incoming starts (never both visible).
    static func outAlpha(_ f: Double) -> Double { clamp01((0.5 - f) / 0.12) }
    static func inAlpha(_ f: Double) -> Double { clamp01((f - 0.5) / 0.12) }
    static func outScale(_ f: Double) -> Double { 1 - 0.4 * clamp01((f - 0.38) / 0.12) }
    static func inScale(_ f: Double) -> Double { 0.6 + 0.4 * clamp01((f - 0.5) / 0.12) }
}
