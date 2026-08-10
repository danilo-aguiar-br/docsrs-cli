//! Full-jitter backoff, politeness delay, and wait selection.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use super::HARD_MAX_DELAY_MS;
use super::config::RetryConfig;

/// Full jitter: `uniform(0..=min(base*2^attempt, max_delay))`.
///
/// Entropy from monotonic `Instant` + stack address (no `rand` dependency).
/// Not cryptographic; sufficient to desynchronize multi-agent thundering herds.
/// For deterministic tests use [`backoff_full_jitter_seeded`].
pub fn backoff_full_jitter(base_ms: u64, attempt: u32, max_delay_ms: u64) -> Duration {
    let seed = mix_u64(attempt);
    backoff_full_jitter_seeded(base_ms, attempt, max_delay_ms, seed)
}

/// Largest exponent the attempt counter may drive `1 << n` with.
///
/// Not a product knob: a shift of 64 or more is undefined behaviour on `u64`,
/// so this is the arithmetic guard, and `max_delay_ms` is what actually bounds
/// the wait. `2^16 * base` already dwarfs any configurable ceiling.
const MAX_BACKOFF_SHIFT: u32 = 16;

/// Full jitter with explicit seed (tests / property checks).
pub fn backoff_full_jitter_seeded(
    base_ms: u64,
    attempt: u32,
    max_delay_ms: u64,
    seed: u64,
) -> Duration {
    let base = base_ms.max(1);
    let max_delay = max_delay_ms.max(base);
    // Saturating shift: attempt capped so 2^n fits in u64 comfortably.
    let exp = base.saturating_mul(1u64 << attempt.min(MAX_BACKOFF_SHIFT));
    let cap = exp.min(max_delay);
    let pick = seed % (cap.saturating_add(1));
    Duration::from_millis(pick)
}

/// Per-host politeness delay with additive jitter (never below the configured floor).
///
/// ```text
/// effective = base + uniform(0..=base/5)   // up to +20% jitter
/// ```
///
/// Keeps the operator-configured minimum interval (crawl-delay style floor) while
/// avoiding synchronized multi-process hits that a fixed sleep would create.
/// Zero base stays zero (tests / `--rate-limit-delay-ms 0`).
pub fn politeness_delay(base: Duration) -> Duration {
    let base_ms = base.as_millis() as u64;
    if base_ms == 0 {
        return Duration::ZERO;
    }
    let span = (base_ms / 5).max(1);
    let extra = mix_u64(base_ms as u32) % (span.saturating_add(1));
    Duration::from_millis(base_ms.saturating_add(extra))
}

/// Prefer server `Retry-After` when present; otherwise full-jitter formula.
///
/// Server hint is honored without additional jitter and **without** shrinking
/// below the server value via `max_delay_ms` (Rules: never impose a shorter
/// backoff than `Retry-After`). Only the hard delay ceiling applies; the
/// elapsed budget gate in the HTTP loop refuses waits that would overrun
/// `max_elapsed_ms`.
pub fn wait_for_retry(
    policy: RetryConfig,
    attempt: u32,
    retry_after: Option<Duration>,
) -> Duration {
    if let Some(d) = retry_after {
        return d.min(Duration::from_millis(HARD_MAX_DELAY_MS));
    }
    policy.delay_for_attempt(attempt)
}

fn mix_u64(attempt: u32) -> u64 {
    let mut h = DefaultHasher::new();
    Instant::now().hash(&mut h);
    attempt.hash(&mut h);
    std::thread::current().id().hash(&mut h);
    // Stack address mixes per-call entropy without a global RNG.
    let marker = &h as *const DefaultHasher as usize;
    marker.hash(&mut h);
    h.finish()
}
