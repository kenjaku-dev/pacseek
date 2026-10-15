//! Backend timing (P1 perf: measure first).
//!
//! `observability`: every backend call in the search/updates/stats fan-outs
//! reports `backend finished in Nms (M results)`. Slow calls (> [`SLOW_MS`])
//! log at warn so they surface with the default filter; fast calls stay at
//! info for `-v`. No behavior change — timing only.
//!
//! No `expect()`/`unwrap()`; std-only.

/// Calls slower than this are warn-level (visible by default).
pub const SLOW_MS: u128 = 2000;

/// True when `ms` exceeds the slow-call budget. Pure, tested.
pub fn is_slow(ms: u128) -> bool {
    ms > SLOW_MS
}

/// One-line report body. Pure, tested — keeps log format stable for scripts
/// scraping `-v` output.
pub fn message(backend: &str, ms: u128, results: usize) -> String {
    format!("backend {backend} finished in {ms}ms ({results} results)")
}

/// Start a timer; call [`Timer::finish`] with the result count when done.
pub fn start(backend: &'static str) -> Timer {
    Timer {
        backend,
        start: std::time::Instant::now(),
    }
}

pub struct Timer {
    backend: &'static str,
    start: std::time::Instant,
}

impl Timer {
    pub fn finish(self, results: usize) {
        let ms = self.start.elapsed().as_millis();
        let msg = message(self.backend, ms, results);
        if is_slow(ms) {
            tracing::warn!("{msg} — over {SLOW_MS}ms budget");
        } else {
            tracing::info!("{msg}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_threshold_boundary() {
        assert!(!is_slow(0));
        assert!(!is_slow(SLOW_MS));
        assert!(is_slow(SLOW_MS + 1));
    }

    #[test]
    fn message_format_stable() {
        assert_eq!(
            message("aur-search", 123, 50),
            "backend aur-search finished in 123ms (50 results)"
        );
    }

    #[test]
    fn timer_finish_never_panics() {
        // No subscriber installed here: tracing no-ops must not panic.
        start("test-backend").finish(0);
        start("test-backend").finish(1000);
    }
}
