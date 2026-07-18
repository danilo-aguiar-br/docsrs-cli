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
RUSTDOCFLAGS='-D missing_docs -D rustdoc::broken_intra_doc_links' cargo doc --no-deps --locked
```


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
- Optional human live smoke: `./scripts/smoke-live.sh` (not CI)
- Tag only after maintainer approval
- Publish crates.io only with explicit maintainer authorization


## Recognition
- Contributors appear in git history and release notes when relevant
- Security reporters may be listed in SECURITY Hall of Fame after fixes ship


## Questions
- Prefer GitHub issues for product questions
- Security mail goes to daniloaguiarbr@proton.me only
