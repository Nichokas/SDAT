import SwiftUI

/// Press-and-hold menu of the circle miniplayer. The circle sits in the bottom-right corner, so it
/// turns into a close button under the finger and the controls fan out over the quarter above and
/// to its left. Sliding onto a control and lifting runs it; lifting on the close button leaves the
/// fan open for taps. Mirrored by the Android `MiniFan`.
@MainActor @Observable
final class MiniFan {
    static let shared = MiniFan()

    /// A shorter hold toggled playback by accident while tapping to open the player.
    nonisolated static let hold: TimeInterval = 0.4
    nonisolated static let radius: CGFloat = 92
    /// How far the finger must slide from where it pressed before it leaves the close button.
    nonisolated static let deadZone: CGFloat = 32

    enum Item: CaseIterable {
        case close, toggle, previous, next

        /// Degrees from straight up, counterclockwise toward the left.
        var angle: CGFloat {
            switch self {
            case .close, .toggle: 0
            case .previous: 45
            case .next: 90
            }
        }
        var size: CGFloat {
            switch self {
            case .close: CGFloat(Morph.circle)
            case .toggle: 56
            case .previous, .next: 48
            }
        }
        /// Centre relative to the circle's centre.
        var offset: CGSize {
            let r = self == .close ? 0 : MiniFan.radius
            let a = angle * .pi / 180
            return CGSize(width: -sin(a) * r, height: -cos(a) * r)
        }
    }

    enum State: Equatable {
        case closed
        /// The finger is still down; `hover` is what lifting it would pick, nil for nothing.
        case held(hover: Item?)
        case open
    }

    private(set) var state = State.closed
    /// Centre of the circle in global coordinates; read when the fan opens.
    @ObservationIgnored var anchor = CGPoint.zero

    var hover: Item? {
        if case .held(let h) = state { return h }
        return nil
    }

    /// What a finger `d` away from where it pressed points at.
    static func hit(_ d: CGSize) -> Item? {
        if hypot(d.width, d.height) < deadZone { return .close }
        let angle = atan2(-d.width, -d.height) * 180 / .pi
        // Past the fan's quarter by more than half a gap: the finger points away from every control.
        guard angle >= -30, angle <= 120 else { return nil }
        return [Item.toggle, .previous, .next].min { abs($0.angle - angle) < abs($1.angle - angle) }
    }

    func hold() { state = .held(hover: .close) }

    /// Returns true when the finger has just moved onto a different item.
    func move(_ d: CGSize) -> Bool {
        guard case .held(let before) = state else { return false }
        let h = Self.hit(d)
        state = .held(hover: h)
        return h != nil && h != before
    }

    /// Ends a hold and returns the control to run, if the finger lifted on one.
    func release() -> Item? {
        let h = hover
        state = h == .close ? .open : .closed
        return h == .close ? nil : h
    }

    func close() { state = .closed }
}
