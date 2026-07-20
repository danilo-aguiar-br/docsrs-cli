[Português (pt-BR)](TESTING.pt-BR.md)

> **1.2.0 dogfood:** always invoke `./target/release/docsrs-cli` (or `cargo run --release --`) for product checks. A PATH install may lag the tree (GAP-W-005). Prefer `cargo audit --no-fetch` when the advisory DB index hangs (GAP-W-010). Install from this tree with `cargo install --path . --force` (not crates.io until a publish is authorized; crates.io may still list an older version — GAP-X-009).

# Testing
## Why Categorized Tests
- Offline unit and integration tests must stay green without network
- Live network tests are intentional and gated
- Signal and CLI smoke tests protect the agent contract

## Test Categories
- Unit tests inside `src/**` for pure logic
- Integration tests under `tests/` with wiremock and offline fixtures
- Golden render and golden diff tests for stable output shapes
- CLI smoke tests for argv and exit behavior
- Signal tests for cancel and terminate semantics
- Optional live network tests for crates.io, docs.rs, and stdlib

## How to Run
```bash
cargo test --locked --all-targets
cargo test --locked --test cli_smoke
cargo test --locked --test e2e_offline
cargo test --locked --test http_integration
cargo test --locked --test golden_render
cargo test --locked --test golden_diff
cargo test --locked --test signal_term
cargo test --locked --test lib_dispatch
```

## Live Network Profiles
```bash
DOCSRS_CLI_NETWORK_TESTS=1 cargo test --locked --test network_live -- --ignored
DOCSRS_CLI_STDLIB_NETWORK_TESTS=1 cargo test --locked --test network_live -- --ignored
```
- Leave these unset in default local loops
- Use them only when you intend to hit public hosts

## Offline Mocks With config.toml
- Product origins are not set via env vars
- Point tests at wiremock by writing TOML under a sandbox home (or use `--config-dir` + `--allow-loopback`):
```bash
CFG="$(mktemp -d)"
CACHE="$(mktemp -d)"
cat > "$CFG/config.toml" <<'TOML'
allow_loopback = true
crates_io_origin = "http://127.0.0.1:PORT"
docs_rs_origin = "http://127.0.0.1:PORT"
rate_limit_delay_ms = 0
max_retries = 1
TOML
docsrs-cli --allow-loopback --config-dir "$CFG" --cache-dir "$CACHE" doctor --json
```
- Isolate config/cache with `--config-dir` / `--cache-dir` (product never reads path env)
- Loopback mocks require `allow_loopback = true` in TOML and/or CLI `--allow-loopback` (never env; ADR 0009)
- Keep product knobs (timeouts, retries, UA, origins, loopback) in flags or TOML only

## CI Profiles
- This repository does not ship GitHub Actions workflows by policy
- Local validation is the gate before publish
- Recommended local gate:
```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --all-targets
cargo build --release
```

## Human live smoke (pre-release, not CI)
```bash
cargo build --release
./scripts/smoke-live.sh
```
- Uses `--config-dir` / `--cache-dir` temp dirs (XDG via flags; no product env knobs)
- Asserts page-token echo, `budget` non-retryable, timeout 0 fail-closed, version from binary (dogfood `./target/release/docsrs-cli` for 1.2.0)
- Requires network; fail open if hosts are down

## Environment Variables
- `DOCSRS_CLI_NETWORK_TESTS` enables crates.io and docs.rs live tests (harness only)
- `DOCSRS_CLI_STDLIB_NETWORK_TESTS` enables doc.rust-lang.org live tests (harness only)
- Integration tests isolate storage with `--config-dir` / `--cache-dir` (tempdir)
- Live network tests are `#[ignore]`; enable with env harness + `cargo test -- --ignored`
- There are **no** product env knobs for origins, retries, UA, timeouts, or loopback allowlist (use CLI/XDG)

## Troubleshooting
- If live tests fail, confirm network and host availability first
- If offline tests fail, do not enable live flags to hide the failure
- If golden tests fail, inspect intentional render contract changes
- If mock origins are refused, set `allow_loopback = true` in `config.toml` and/or pass `--allow-loopback`
- If signal tests flake, ensure no external process steals the controlling terminal
