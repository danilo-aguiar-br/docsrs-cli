//! Named HTTP/TCP posture constants (Rules Rust — rede; no magic numbers).

/// Max idle connections retained per host in the reqwest pool (one-shot CLI).
///
/// Product issues few sequential GETs against 2–4 allowlisted hosts; a large
/// pool wastes FDs without latency wins after process exit.
pub const POOL_MAX_IDLE_PER_HOST: usize = 2;

/// Idle connection lifetime in the pool before eviction (seconds).
pub const POOL_IDLE_TIMEOUT_SECS: u64 = 30;

/// TCP keepalive idle probe delay (seconds) — Rules Rust baseline for dead peers.
pub const TCP_KEEPALIVE_IDLE_SECS: u64 = 60;

/// TCP keepalive probe interval (seconds).
pub const TCP_KEEPALIVE_INTERVAL_SECS: u64 = 10;

/// TCP keepalive probe retry count before the connection is considered dead.
pub const TCP_KEEPALIVE_RETRIES: u32 = 3;
