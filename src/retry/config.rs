//! Retry policy struct resolved from [`Config`] (single source of truth).

use std::time::Duration;

use crate::config::Config;

use super::backoff::backoff_full_jitter;
use super::{
    DEFAULT_MAX_RETRIES, DEFAULT_RETRY_BASE_MS, DEFAULT_RETRY_MAX_DELAY_MS,
    DEFAULT_RETRY_MAX_ELAPSED_MS, HARD_MAX_DELAY_MS, HARD_MAX_ELAPSED_MS, HARD_MAX_RETRIES,
    MILLIS_PER_SECOND, MIN_DERIVED_ELAPSED_MS, MIN_RETRY_BASE_MS,
};

/// Named retry policy for the product HTTP client (one dependency class).
///
/// Built from [`Config`] so CLI / TOML / env share one struct. Clone is cheap
/// (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryConfig {
    /// Retries after the first attempt (`0` = single try).
    pub max_retries: u32,
    /// Exponential base delay in milliseconds.
    pub base_ms: u64,
    /// Maximum single sleep in milliseconds.
    pub max_delay_ms: u64,
    /// Total wall budget for the retry loop (attempts + sleeps), milliseconds.
    pub max_elapsed_ms: u64,
    /// Kill switch: when false, never retries.
    pub enabled: bool,
}

impl RetryConfig {
    /// Resolve policy from runtime config (clamped).
    ///
    /// Coherence: `base_ms ≥ MIN_RETRY_BASE_MS`, `max_delay ≥ base`,
    /// `max_elapsed ≥ max_delay` (or wall timeout when config field is `0`).
    pub fn from_config(cfg: &Config) -> Self {
        let max_retries = cfg.max_retries.min(HARD_MAX_RETRIES);
        let base_ms = if cfg.retry_base_ms == 0 {
            DEFAULT_RETRY_BASE_MS
        } else {
            cfg.retry_base_ms.max(MIN_RETRY_BASE_MS)
        };
        let max_delay_ms = cfg.retry_max_delay_ms.min(HARD_MAX_DELAY_MS).max(base_ms);
        // `0` means derive from wall-clock timeout (one-shot: retries must not
        // outlive the operation deadline).
        let derived = cfg
            .timeout_secs
            .saturating_mul(MILLIS_PER_SECOND)
            .max(MIN_DERIVED_ELAPSED_MS);
        let max_elapsed_ms = if cfg.retry_max_elapsed_ms == 0 {
            derived.min(HARD_MAX_ELAPSED_MS)
        } else {
            cfg.retry_max_elapsed_ms
                .min(HARD_MAX_ELAPSED_MS)
                .max(max_delay_ms)
        };
        let enabled = !cfg.disable_retry && max_retries > 0;
        Self {
            max_retries,
            base_ms,
            max_delay_ms,
            max_elapsed_ms,
            enabled,
        }
    }

    /// Total attempts including the first try.
    pub fn max_attempts(self) -> u32 {
        if !self.enabled {
            1
        } else {
            self.max_retries.saturating_add(1)
        }
    }

    /// Whether another attempt is allowed after `attempt` (1-based completed count).
    pub fn may_retry(self, attempt: u32) -> bool {
        self.enabled && attempt < self.max_attempts()
    }

    /// Whether another attempt is allowed given attempt count **and** elapsed budget.
    ///
    /// Rules: combine `max_attempts` and `max_elapsed_time`. A planned wait that
    /// would exceed the remaining budget aborts retry (no sleep past budget).
    pub fn may_retry_within_budget(
        self,
        attempt: u32,
        elapsed: Duration,
        planned_wait: Duration,
    ) -> bool {
        if !self.may_retry(attempt) {
            return false;
        }
        let budget = Duration::from_millis(self.max_elapsed_ms);
        if elapsed >= budget {
            return false;
        }
        let remaining = budget.saturating_sub(elapsed);
        planned_wait <= remaining
    }

    /// Remaining time in the elapsed budget (zero when exhausted).
    pub fn remaining_budget(self, elapsed: Duration) -> Duration {
        Duration::from_millis(self.max_elapsed_ms).saturating_sub(elapsed)
    }

    /// Full-jitter delay for this attempt number (1-based).
    pub fn delay_for_attempt(self, attempt: u32) -> Duration {
        backoff_full_jitter(self.base_ms, attempt, self.max_delay_ms)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            base_ms: DEFAULT_RETRY_BASE_MS,
            max_delay_ms: DEFAULT_RETRY_MAX_DELAY_MS,
            max_elapsed_ms: DEFAULT_RETRY_MAX_ELAPSED_MS,
            enabled: true,
        }
    }
}
