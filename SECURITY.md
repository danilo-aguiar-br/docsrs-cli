[Português (pt-BR)](SECURITY.pt-BR.md)

# Security Policy


## Supported Versions
- `0.1.x` receives security fixes
- Pre-release experiments outside tagged releases are unsupported


## Reporting a Vulnerability
- Report privately to daniloaguiarbr@proton.me
- Do not open a public issue for unfixed vulnerabilities
- Include impact description, reproduction steps, and affected version or commit
- Include OS, CLI version from `docsrs-cli version --json`, and redacted logs


## Response SLA
- Critical (CVSS 9.0-10.0): acknowledge within 2 business days
- High (CVSS 7.0-8.9): acknowledge within 3 business days
- Medium (CVSS 4.0-6.9): acknowledge within 5 business days
- Low (CVSS 0.1-3.9): acknowledge within 10 business days


## Fix SLA
- Critical: target fix or mitigation within 14 days after confirmation
- High: target fix within 30 days after confirmation
- Medium: target fix within 60 days after confirmation
- Low: target fix in the next regular maintenance window


## Disclosure Policy
- Coordinate disclosure after a fix is available or a mitigation is documented
- Credit reporters who want public recognition in the Hall of Fame
- Do not demand NDAs for good-faith reports


## Security Update Policy
- Ship security fixes on the supported minor line when possible
- Document security-relevant changes in CHANGELOG under Security
- Prefer least-privilege defaults and fail-closed configuration


## Scope Notes
- Product HTTP is GET-only against `crates.io`, `docs.rs`, `static.docs.rs`, and `doc.rust-lang.org`
- TLS uses rustls only
- No product telemetry
- No API keys are stored by the product
- Disk cache holds public HTTP bodies only
- Unix private modes prefer `0o700` dirs and `0o600` files for CLI writes


## Hall of Fame
- Security researchers who report valid issues may be listed here after fixes ship
- No entries yet for 0.1.0


## Best Practices for Users
- Install with `cargo install docsrs-cli --locked`
- Keep the binary updated on the supported minor line
- Do not pass secrets on argv when stdin or env is available
- Run `docsrs-cli doctor --json` after changing config paths
- Treat cache contents as public documentation snapshots, not secrets
