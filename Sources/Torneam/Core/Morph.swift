import Foundation

/// Pure geometry + fade schedule of the bottom-chrome morph (tab bar <-> miniplayer).
/// Everything derives from ONE clock `t` (0...1, linear in time over `Morph.duration`)
/// and the direction, so widths come from a single budget and can never overlap.
enum Morph {
    static let duration = 0.75
    /// Shrinking shape finishes at 42% of the clock (~0.32 s of 0.75 s).
    static let shrinkShare = 0.42
    static let round = 54.0      // collapsed tab button
    static let circle = 62.0     // miniplayer circle
    static let gap = 12.0

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
    /// Per-role speeds on ONE budget: the shrinking shape snaps away (ease-out, done at
    /// ~0.32 s of 0.75 s, like a released rubber band), the growing one spreads slowly
    /// (ease-in, constant speed, ease-out; done at ~0.6 s, like water) and is capped at the space actually freed, so
    /// tabW + gap + miniW <= w always. Any leftover is a transparent, temporary extra gap:
    /// the tab bar is pinned to the leading margin and the miniplayer to the trailing one.
    static func frame(width w: Double, t: Double, collapsing: Bool) -> Frame {
        let a = max(w - gap, round + circle)
        let tabMax = a - circle, span = tabMax - round
        let c = clamp01(t)
        let shrink = snap(collapsing ? c : 1 - c)
        let grow = spread(collapsing ? c : 1 - c)
        var tabF: Double, miniF: Double
        if collapsing { tabF = shrink; miniF = min(grow, tabF) }
        else { miniF = 1 - shrink; tabF = max(1 - grow, miniF) }
        return Frame(tabW: tabMax - span * tabF, miniW: circle + span * miniF, tabF: tabF, miniF: miniF)
    }

    /// Growing shape: finishes at `growShare` of the clock (~0.6 s of 0.75 s) with a
    /// trapezoidal velocity: short ease-in, long constant-speed middle, short ease-out.
    static let growShare = 0.8
    static let growEase = 0.22   // fraction of the grow spent accelerating (and braking)
    static func spread(_ x: Double) -> Double {
        let u = clamp01(x / growShare), a = growEase, v = 1 / (1 - a)
        if u < a { return 0.5 * v * u * u / a }
        if u > 1 - a { let r = 1 - u; return 1 - 0.5 * v * r * r / a }
        return v * (u - a / 2)
    }

    /// Rubber-band release: fast start, ease-out, finished at `shrinkShare` of the clock.
    /// Smoothstep on a warped clock (u^0.7): a short, subtle ease-in, peak speed early,
    /// then a soft brake. Done at `shrinkShare` of the clock (~0.32 s of 0.75 s).
    static func snap(_ x: Double) -> Double {
        let u = pow(clamp01(x / shrinkShare), 0.7)
        return u * u * (3 - 2 * u)
    }

    /// Short, crisp crossfade around the midpoint: content is fully shown outside 0.38...0.62,
    /// outgoing is gone exactly when incoming starts (never both visible).
    static func outAlpha(_ f: Double) -> Double { clamp01((0.5 - f) / 0.12) }
    static func inAlpha(_ f: Double) -> Double { clamp01((f - 0.5) / 0.12) }
    static func outScale(_ f: Double) -> Double { 1 - 0.4 * clamp01((f - 0.38) / 0.12) }
    static func inScale(_ f: Double) -> Double { 0.6 + 0.4 * clamp01((f - 0.5) / 0.12) }
}
