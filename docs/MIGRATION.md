[Português (pt-BR)](MIGRATION.pt-BR.md)

# Migration


## What Changes
- Public launch line is `0.1.x`
- Dual license is MIT OR Apache-2.0
- Documentation framework is bilingual with skills under `skills/`
- Command surface for agents remains one-shot JSON on stdout


## Step-by-Step Migration
- Install or upgrade with `cargo install docsrs-cli --locked --force`
- Run `docsrs-cli version --json` and confirm `0.1.0` or newer on the 0.1 line
- Run `docsrs-cli doctor --json` against your config and cache dirs
- Re-read `commands --json` if your agent cached an older command tree
- Point skills and docs links at the new repository layout


## JSON Schema Changes
- Success envelopes keep `schema_version: 1`
- Command payload schemas live under `docs/schemas/*.schema.json`
- Before: agents often scraped HTML without a schema index
- After: agents load `docs/schemas/README.md` and per-command schema files
- `source_url` remains the provenance field on fetched document payloads


## Compatibility Notes
- No daemon migration is required because no daemon exists
- Environment allowlist keys keep the `DOCSRS_CLI_` prefix
- Hard ceilings for body and output bytes remain product policy
- Host allowlist remains crates.io, docs.rs, static.docs.rs, and doc.rust-lang.org


## Rollback
- Install a previous binary only if you still have that artifact
- Clear incompatible local experiments with `docsrs-cli cache clear --json`
- Keep `DOCSRS_CLI_HOME` sandboxes so rollback does not touch production XDG data


## See Also
- [CHANGELOG.md](../CHANGELOG.md)
- [AGENTS.md](AGENTS.md)
- [schemas/README.md](schemas/README.md)
