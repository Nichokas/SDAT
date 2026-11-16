use std::fmt::Write;

use super::{Event, Stats};

fn duration(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}h {:02}m {:02}s", s / 3600, s % 3600 / 60, s % 60)
}

pub(super) fn build(extra: &str, stats: &Stats, events: &[Event]) -> String {
    let mut out = String::new();
    let extra = extra.trim();
    if !extra.is_empty() {
        let _ = writeln!(out, "{extra}\n");
    }
    let _ = writeln!(out, "Uptime: {}", duration(stats.uptime_ms));
    let _ = writeln!(out, "CPU time: {} ms user, {} ms system", stats.cpu_user_ms, stats.cpu_system_ms);
    let _ = writeln!(out, "Peak memory: {} MiB\n", stats.max_rss_kb / 1024);
    let _ = writeln!(out, "Network");
    let _ = writeln!(
        out,
        "{:<14}{:>9}{:>9}{:>12}{:>9}{:>8}{:>6}",
        "", "requests", "failures", "bytes in", "avg ms", "max ms", "last"
    );
    for n in &stats.net {
        let avg = n.total_ms.checked_div(n.requests).unwrap_or(0);
        let last = n.last_status.map_or_else(|| "-".to_owned(), |s| s.to_string());
        let _ = writeln!(
            out,
            "{:<14}{:>9}{:>9}{:>12}{:>9}{:>8}{:>6}",
            n.category.name(),
            n.requests,
            n.failures,
            n.bytes_in,
            avg,
            n.max_ms,
            last
        );
    }
    let _ = writeln!(out, "\nEvents ({})", events.len());
    for e in events {
        let _ = writeln!(out, "{e}");
    }
    out
}
