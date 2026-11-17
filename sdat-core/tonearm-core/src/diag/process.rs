use std::mem::MaybeUninit;

pub(super) struct Usage {
    pub cpu_user_ms: u64,
    pub cpu_system_ms: u64,
    pub max_rss_kb: u64,
}

fn ms(t: libc::timeval) -> u64 {
    let secs = u64::try_from(t.tv_sec).unwrap_or_default();
    let micros = u64::try_from(t.tv_usec).unwrap_or_default();
    secs * 1000 + micros / 1000
}

pub(super) fn usage() -> Usage {
    let mut ru = MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `ru` is a valid, writable `rusage`; the call only fills it in.
    let ok = unsafe { libc::getrusage(libc::RUSAGE_SELF, ru.as_mut_ptr()) } == 0;
    if !ok {
        return Usage { cpu_user_ms: 0, cpu_system_ms: 0, max_rss_kb: 0 };
    }
    // SAFETY: zero-initialised plain integers, and getrusage succeeded.
    let ru = unsafe { ru.assume_init() };
    let rss = u64::try_from(ru.ru_maxrss).unwrap_or_default();
    Usage {
        cpu_user_ms: ms(ru.ru_utime),
        cpu_system_ms: ms(ru.ru_stime),
        // Linux and Android report KiB, Apple platforms bytes.
        max_rss_kb: if cfg!(target_vendor = "apple") { rss / 1024 } else { rss },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_process_has_used_memory_and_cpu() {
        let mut x = 0u64;
        for i in 0..50_000_000u64 {
            x = x.wrapping_add(std::hint::black_box(i));
        }
        std::hint::black_box(x);
        let u = usage();
        assert!(u.max_rss_kb > 0);
        assert!(u.cpu_user_ms + u.cpu_system_ms > 0);
    }
}
