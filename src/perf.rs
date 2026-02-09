#[cfg(feature = "perf")]
use std::time::Instant;

#[cfg(feature = "perf")]
pub struct PerfGuard {
    label: &'static str,
    start: Instant,
}

#[cfg(feature = "perf")]
impl Drop for PerfGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let micros = elapsed.as_secs() * 1_000_000 + u64::from(elapsed.subsec_micros());
        eprintln!("[perf] {}: {}us", self.label, micros);
    }
}

#[cfg(feature = "perf")]
#[inline]
pub fn scope(label: &'static str) -> PerfGuard {
    PerfGuard {
        label,
        start: Instant::now(),
    }
}

#[cfg(not(feature = "perf"))]
pub struct PerfGuard;

#[cfg(not(feature = "perf"))]
impl Drop for PerfGuard {
    fn drop(&mut self) {}
}

#[cfg(not(feature = "perf"))]
#[inline]
pub fn scope(_label: &'static str) -> PerfGuard {
    PerfGuard
}
