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
```


## Live Network Profiles
```bash
DOCSRS_CLI_NETWORK_TESTS=1 cargo test --locked --test network_live
DOCSRS_CLI_STDLIB_NETWORK_TESTS=1 cargo test --locked --test network_live
```
- Leave these unset in default local loops
- Use them only when you intend to hit public hosts


## CI Profiles
- This repository does not ship GitHub Actions workflows by policy
- Local validation is the gate before publish
- Recommended local gate:
```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --all-targets
```


## Environment Variables
- `DOCSRS_CLI_NETWORK_TESTS` enables crates.io and docs.rs live tests
- `DOCSRS_CLI_STDLIB_NETWORK_TESTS` enables doc.rust-lang.org live tests
- `DOCSRS_CLI_ALLOW_LOCALHOST` allows local mock origins in controlled tests
- `DOCSRS_CLI_CRATES_IO_ORIGIN` and `DOCSRS_CLI_DOCS_RS_ORIGIN` override bases for mocks
- `DOCSRS_CLI_HOME` isolates config and cache during tests


## Troubleshooting
- If live tests fail, confirm network and host availability first
- If offline tests fail, do not enable live flags to hide the failure
- If golden tests fail, inspect intentional render contract changes
- If signal tests flake, ensure no external process steals the controlling terminal
