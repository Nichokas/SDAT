//! Drift decisions: how a guest corrects its player towards the host.
//!
//! Policy: the playback rate is **never** changed. Speeding up or slowing
//! down music is audible, and a few seconds of divergence between listeners
//! is acceptable, so errors below [`TOLERANCE_MS`] are ignored. Larger errors
//! are fixed with a single seek, at most once every [`MIN_INTERVAL_NS`], and
//! never while the player is still settling after a start or a seek
//! ([`SETTLE_NS`]): measuring a player that is still filling its buffer
//! reports a large, fake error and causes seek loops (audible gaps).

/// Errors up to this many milliseconds are left alone.
pub const TOLERANCE_MS: i64 = 3_000;
/// Precision for positioning a **paused** player.
///
/// Clients use it to decide whether a paused guest must be moved to the host
/// position. A paused seek is inaudible, so it stays precise (300 ms, as
/// before); it is not used while playing.
pub const SEEK_MS: i64 = 300;
/// A host anchor on the same track that moves the position by more than this
/// (beyond normal progress) is a host seek, not drift.
pub const HOST_JUMP_MS: i64 = 1_000;
/// Minimum time between two corrective seeks.
pub const MIN_INTERVAL_NS: i64 = 10_000_000_000;
/// After a start, a reset or a corrective seek no correction is made for this
/// long, counted from the first measurement (the player is still buffering).
pub const SETTLE_NS: i64 = 5_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Correction {
    /// Change nothing.
    None,
    /// Play at this rate. **Never produced any more**: kept only so existing
    /// clients still compile. Clients should treat it like `None`.
    Rate(f64),
    /// Seek to this position (ms), playing at normal speed.
    Seek(i64),
}

#[derive(Debug, Clone, Default)]
pub struct DriftController {
    last_seek_ns: Option<i64>,
    /// Start of the current settle window; `None` until the first measurement
    /// after [`Self::new`] or [`Self::reset`].
    settle_from_ns: Option<i64>,
    /// The host seeked (or moved to another entry of the same song): the
    /// next decision follows it right away, ignoring settle and spacing.
    host_jump: bool,
    /// Kept for API compatibility; has no effect (every correction is a seek).
    pub seek_only: bool,
}

impl DriftController {
    pub fn new() -> Self {
        Self::default()
    }

    /// `expected_ms`: where the host is (already including output latency);
    /// `actual_ms`: where the local player is; `now_ns`: local monotonic time.
    /// Call it only while the local player is actually playing.
    pub fn decide(&mut self, expected_ms: i64, actual_ms: i64, now_ns: i64) -> Correction {
        let abs = (expected_ms - actual_ms).abs();
        if std::mem::take(&mut self.host_jump) && abs > TOLERANCE_MS {
            return self.seek(expected_ms, now_ns);
        }
        let from = *self.settle_from_ns.get_or_insert(now_ns);
        if now_ns - from < SETTLE_NS {
            return Correction::None;
        }
        if abs <= TOLERANCE_MS {
            return Correction::None;
        }
        if self.last_seek_ns.is_some_and(|t| now_ns - t < MIN_INTERVAL_NS) {
            return Correction::None;
        }
        self.seek(expected_ms, now_ns)
    }

    fn seek(&mut self, expected_ms: i64, now_ns: i64) -> Correction {
        self.last_seek_ns = Some(now_ns);
        self.settle_from_ns = Some(now_ns);
        Correction::Seek(expected_ms.max(0))
    }

    /// The host jumped on the same track (a seek, or the next queue entry is
    /// the same song so the client keeps its player): honor it at once,
    /// bypassing the settle window and seek spacing (it is not drift).
    pub fn host_jumped(&mut self) {
        self.host_jump = true;
    }

    /// New track, host seek/pause/resume or reconnect: start a fresh settle
    /// window. The seek spacing is kept so a flapping host can't cause a seek
    /// loop; a genuinely new position is still followed within
    /// [`MIN_INTERVAL_NS`].
    pub fn reset(&mut self) {
        self.settle_from_ns = None;
        self.host_jump = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const S: i64 = 1_000_000_000;

    /// A controller past its first settle window, at `t` = `10 * S`.
    fn settled() -> DriftController {
        let mut d = DriftController::new();
        assert_eq!(d.decide(10_000, 0, 0), Correction::None);
        d
    }

    #[test]
    fn never_changes_rate() {
        let mut d = DriftController::new();
        for i in 0..200 {
            let err = (i * 37 % 7_000) - 3_500;
            let c = d.decide(100_000 + err, 100_000, i * S / 4);
            assert!(!matches!(c, Correction::Rate(_)), "{c:?}");
        }
    }

    #[test]
    fn errors_within_tolerance_are_ignored() {
        let mut d = settled();
        assert_eq!(d.decide(13_000, 10_000, 10 * S), Correction::None);
        assert_eq!(d.decide(7_000, 10_000, 11 * S), Correction::None);
        assert_eq!(d.decide(10_100, 10_000, 12 * S), Correction::None);
    }

    #[test]
    fn large_error_seeks() {
        let mut d = settled();
        assert_eq!(d.decide(13_001, 10_000, 10 * S), Correction::Seek(13_001));
        let mut d = settled();
        assert_eq!(d.decide(6_999, 10_000, 10 * S), Correction::Seek(6_999));
    }

    #[test]
    fn nothing_happens_while_settling_after_start() {
        let mut d = DriftController::new();
        assert_eq!(d.decide(20_000, 0, 100 * S), Correction::None);
        assert_eq!(d.decide(20_000, 0, 100 * S + SETTLE_NS - 1), Correction::None);
        assert_eq!(d.decide(20_000, 0, 100 * S + SETTLE_NS), Correction::Seek(20_000));
    }

    #[test]
    fn seeks_are_spaced_and_followed_by_a_settle_window() {
        let mut d = settled();
        assert_eq!(d.decide(20_000, 0, 10 * S), Correction::Seek(20_000));
        // Still wrong right after the seek (buffering): no second seek.
        assert_eq!(d.decide(25_000, 0, 16 * S), Correction::None);
        assert_eq!(d.decide(30_000, 0, 20 * S - 1), Correction::None);
        assert_eq!(d.decide(30_000, 0, 20 * S), Correction::Seek(30_000));
    }

    #[test]
    fn reset_restarts_settling_but_keeps_seek_spacing() {
        let mut d = settled();
        assert_eq!(d.decide(20_000, 0, 10 * S), Correction::Seek(20_000));
        d.reset();
        assert_eq!(d.decide(90_000, 0, 30 * S), Correction::None); // settle starts
        assert_eq!(d.decide(90_000, 0, 34 * S), Correction::None);
        assert_eq!(d.decide(90_000, 0, 35 * S), Correction::Seek(90_000));
        d.reset();
        assert_eq!(d.decide(1_000, 90_000, 36 * S), Correction::None);
        // Settled at 41 s but the last seek was at 35 s: wait for the spacing.
        assert_eq!(d.decide(1_000, 90_000, 41 * S), Correction::None);
        assert_eq!(d.decide(1_000, 90_000, 45 * S), Correction::Seek(1_000));
    }

    #[test]
    fn host_jump_is_followed_at_once() {
        let mut d = settled();
        assert_eq!(d.decide(20_000, 0, 10 * S), Correction::Seek(20_000));
        // Host seeks 1 s after our correction: no cooldown, no settle.
        d.host_jumped();
        assert_eq!(d.decide(90_000, 21_000, 11 * S), Correction::Seek(90_000));
        // Our own corrections are still spaced afterwards.
        assert_eq!(d.decide(99_000, 90_000, 17 * S), Correction::None);
    }

    #[test]
    fn host_jump_within_tolerance_needs_nothing() {
        let mut d = DriftController::new();
        d.host_jumped();
        assert_eq!(d.decide(12_000, 10_000, 0), Correction::None);
        // The pending jump is consumed: settling applies as usual.
        assert_eq!(d.decide(20_000, 10_000, S), Correction::None);
    }

    #[test]
    fn seek_only_flag_changes_nothing() {
        let mut d = settled();
        d.seek_only = true;
        assert_eq!(d.decide(10_500, 10_000, 10 * S), Correction::None);
    }

    #[test]
    fn never_seeks_negative() {
        let mut d = settled();
        assert_eq!(d.decide(-5_000, 1_000, 10 * S), Correction::Seek(0));
    }
}
