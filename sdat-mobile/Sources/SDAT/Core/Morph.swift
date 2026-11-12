import Foundation
import SwiftUI

/// Pure geometry + fade schedule of the bottom-chrome morph (tab bar <-> miniplayer).
/// Both widths derive from ONE progress `t` (0 expanded ... 1 collapsed), so what the tab bar
/// gives up the miniplayer takes in the same frame: the gap between them never changes.
enum Morph {
    /// A spring, so a reversal mid-morph turns around smoothly. No bounce: an overshoot
    /// would push the shapes past their margins.
    static let animation = Animation.smooth(duration: 0.45)
    static let round = 54.0      // collapsed tab button
    static let circle = 62.0     // miniplayer circle
    static let gap = 12.0

    static func clamp01(_ x: Double) -> Double { min(max(x, 0), 1) }

    struct Frame: Equatable {
        var tabW: Double, miniW: Double
        /// 0 = full tab bar + round miniplayer, 1 = round tab button + capsule miniplayer.
        var f: Double
    }

    /// `w` = width inside the horizontal margins.
    static func frame(width w: Double, t: Double) -> Frame {
        let a = max(w - gap, round + circle)
        let span = a - circle - round
        let f = clamp01(t)
        return Frame(tabW: round + span * (1 - f), miniW: circle + span * f, f: f)
    }

    /// Crossfade of each shape's two contents. The windows overlap slightly so a shape is
    /// never an empty blob mid-morph.
    static func outAlpha(_ f: Double) -> Double { 1 - clamp01((f - 0.1) / 0.4) }
    static func inAlpha(_ f: Double) -> Double { clamp01((f - 0.45) / 0.4) }
    static func outScale(_ f: Double) -> Double { 1 - 0.2 * clamp01(f / 0.5) }
    static func inScale(_ f: Double) -> Double { 0.8 + 0.2 * clamp01((f - 0.45) / 0.4) }
}
