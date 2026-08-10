[Português (pt-BR)](CONTRIBUTING.pt-BR.md)

# Contributing
## Welcome
- Thank you for improving docsrs-cli
- Keep diffs surgical and product-focused
- Do not add product telemetry


## Quick Start
```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --all-targets
```


## Development Setup
- Install Rust 1.88 or newer via rustup
- Clone the repository and work from the checkout root
- Prefer `cargo run -q -- <args>` during local development
- Validate rustdoc with:
```bash
./scripts/check-docs.sh
# equivalently, by hand:
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
```
- `-D warnings` is the whole lint set on purpose. The narrower pair this file used
  to teach — `-D missing_docs -D rustdoc::broken_intra_doc_links` — omits
  `rustdoc::private_intra_doc_links`, the class that produced the last five
  failures found by a pre-publish run (GAP-DOC-GATE-001). Following the documented
  command to the letter reported green while the documentation did not build.


## Branching Strategy
- Branch from `main` for every change
- Use short topic names such as `fix/rate-limit` or `docs/agents`
- Keep one concern per branch


## Commit Convention
- Write imperative subject lines
- Explain why in the body when the diff is non-obvious
- Never add `Co-authored-by` trailers


## PR Process
- Open a focused pull request with a clear summary
- Link related issues when they exist
- Expect review on contract stability, docs pairs, and tests
- Do not publish crates.io or push release tags without maintainer authorization


## Testing
- Run the full suite with `cargo test --locked --all-targets`
- Network live tests stay gated behind `DOCSRS_CLI_NETWORK_TESTS`
- Stdlib live tests stay gated behind `DOCSRS_CLI_STDLIB_NETWORK_TESTS`
- See [docs/TESTING.md](docs/TESTING.md)


## Documentation
- Update English and Portuguese public docs in the same delivery
- Keep technical tokens untranslated
- Index every new JSON schema in `docs/schemas/README.md`
- Dogfood product checks with `./target/release/docsrs-cli` (PATH install may lag the tree)
- Prefer `cargo audit --no-fetch` when the advisory DB index hangs
- Keep docs bilingual and version-current with `Cargo.toml` on every release
- Mirror operational knowledge into `skills/docsrs-cli-en` and `skills/docsrs-cli-pt`


## Report Bugs
- Open an issue with command, flags, exit code, and redacted stderr
- Include OS, Rust version, and `docsrs-cli version --json` output
- Never paste secrets into issues


## Request Features
- Describe the agent workflow the feature unblocks
- Prefer stable JSON contracts over human prose
- Call out whether the change is one-shot compatible


## Release Process
- Bump version in `Cargo.toml` with SemVer
- Update `CHANGELOG.md` and `CHANGELOG.pt-BR.md`
- Sync bilingual public docs, `llms*.txt`, and skills with the new command surface
- Update `SECURITY.md` / `SECURITY.pt-BR.md` supported versions line
- Confirm `docs/MIGRATION.md` covers the release breakings
- Run offline gates: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --locked --all-targets`
- Run every local gate in one command: `./scripts/check-all.sh` — it runs the Rust policy suite (`cargo test --test policy_gates`), then discovers `check-supply.sh` and `check-targets.sh` from the directory, and fails closed on any of them
- The policy gates are Rust (`tests/policy_gates.rs`), not a shell script: they run on all three supported platforms, with no interpreter and no external CLI
- On a host without a mingw or Apple cross toolchain: `./scripts/check-all.sh --allow-no-cross`, which records the missing coverage as a decision instead of a silent pass
- Optional human live smoke: `./scripts/smoke-live.sh` (not CI)
- Tag only after maintainer approval
- Publish crates.io only with explicit maintainer authorization


## Recognition
- Contributors appear in git history and release notes when relevant
- Security reporters may be listed in SECURITY Hall of Fame after fixes ship


## Questions
- Prefer GitHub issues for product questions
- Security mail goes to daniloaguiarbr@proton.me only
