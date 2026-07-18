[Português (pt-BR)](TESTING.pt-BR.md)

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
DOCSRS_CLI_NETWORK_TESTS=1 cargo test --locked --test network_live
DOCSRS_CLI_STDLIB_NETWORK_TESTS=1 cargo test --locked --test network_live
```
- Leave these unset in default local loops
- Use them only when you intend to hit public hosts

## Offline Mocks With config.toml
- Product origins are not set via `DOCSRS_CLI_CRATES_IO_ORIGIN` or `DOCSRS_CLI_DOCS_RS_ORIGIN`
- Point tests at wiremock by writing TOML under a sandbox home:
```bash
export DOCSRS_CLI_HOME=/tmp/docsrs-test-home
export DOCSRS_CLI_ALLOW_LOCALHOST=1
mkdir -p "$DOCSRS_CLI_HOME/config"
cat > "$DOCSRS_CLI_HOME/config/config.toml" <<'TOML'
crates_io_origin = "http://127.0.0.1:PORT"
docs_rs_origin = "http://127.0.0.1:PORT"
rate_limit_delay_ms = 0
max_retries = 1
TOML
```
- `DOCSRS_CLI_HOME` isolates config and cache for the process
- `DOCSRS_CLI_ALLOW_LOCALHOST` allows local mock origins in controlled tests
- Keep product knobs (timeouts, retries, UA, origins) in flags or TOML only

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
- Asserts page-token echo, `budget` non-retryable, timeout 0 fail-closed, version 0.1.2
- Requires network; fail open if hosts are down

## Environment Variables
- `DOCSRS_CLI_NETWORK_TESTS` enables crates.io and docs.rs live tests
- `DOCSRS_CLI_STDLIB_NETWORK_TESTS` enables doc.rust-lang.org live tests
- `DOCSRS_CLI_ALLOW_LOCALHOST` allows local mock origins in controlled tests
- `DOCSRS_CLI_HOME` isolates config and cache during tests
- Path overrides `DOCSRS_CLI_CONFIG_DIR` and `DOCSRS_CLI_CACHE_DIR` remain available
- There are no product env knobs for origins, retries, UA, or timeouts in 0.1.x

## Troubleshooting
- If live tests fail, confirm network and host availability first
- If offline tests fail, do not enable live flags to hide the failure
- If golden tests fail, inspect intentional render contract changes
- If mock origins are refused, set `DOCSRS_CLI_ALLOW_LOCALHOST=1` and put origins in `config.toml`
- If signal tests flake, ensure no external process steals the controlling terminal
