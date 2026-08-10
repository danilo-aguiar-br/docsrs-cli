[Português (pt-BR)](0010-explicit-target-designation.pt-BR.md)

# ADR 0010 — Explicit Target Designation for destructive verbs

## Status
- Accepted (2026-08-10)

## Context
- Two commands destroy something: `cache clear` empties every cached body under the resolved root, and `config init --force` replaces `config.toml` in place.
- Both resolve their target through the same layered path resolution as every other command: a CLI flag if one is present, otherwise the XDG directory.
- That resolution is correct for a read. For a destroy it produces the confused-deputy shape: the caller names the verb, the environment names the victim, and nothing compares the two.
- A caller who never passed `--cache-dir` never saw the path about to be emptied. The command still had everything it needed to empty it.
- `cache clear` learned the rule first, as one arm of one `match`. `config init --force` was supposed to learn it in the same plan and did not.
- The gap was not the missing guard. It was that nothing could tell the guard was missing, because a rule living inside a `match` arm cannot be asked a question.
- A second destructive verb therefore shipped without a waiver while every gate in the tree agreed with the tree.

## Decision

### 1. A destructive verb must have its target designated in argv
- Naming the directory with the verb's target flag designates it: the caller saw the path.
- Passing the waiver flag accepts the ambient directory on purpose: the caller chose not to name it.
- Passing neither is refused: exit 64, `kind=usage`, and nothing is destroyed.
- Only `PathSource::CliFlag` counts as designation. `Xdg` is the ambient layer the caller never named.
- `Unresolved` is worse than ambient and is refused for a different reason: there is no target at all, so acting would be acting somewhere unknown.

### 2. The refusal never depends on the disk
- `config init --force` refuses an ambient target even where no `config.toml` exists yet.
- A rule that consulted the disk would answer differently on two machines with the same argv, which makes the contract unteachable.
- The caller cannot know in advance whether a directory they never named holds a file, so the answer must not depend on it.

### 3. The refusal names the victim and both ways out
- The message carries the verb, the resolved target, the target flag and the waiver flag.
- A refusal that hides which path was about to be destroyed teaches the caller nothing about what to pass next.

### 4. The rule lives in a register, not in a guard
- `src/cli/destructive.rs` holds one `DestructiveVerb` per verb, with `wire`, `target_flag`, `waiver_flag`, `effect` and `schema_stem`.
- The runtime reads that list to decide whether to refuse.
- `tests/policy/etd.rs` reads the same list to demand a waiver flag, a `target_source` in the named schema, a line in both configuration references, and an announcement in both migration guides.
- A third verb that destroys is either in the register or the class gate fails. It cannot be quietly correct the way `config init --force` was.

### 5. The envelope reports which layer resolved the target
- Both verbs emit `target_source` with values `cli`, `xdg` or `unresolved`.
- The field carries the same name and the same values on both verbs, so one audit reads both.
- `config init` previously emitted `source`; the rename is the 1.3.0 break recorded in the migration guide.

## Consequences
- A script written before this rule, invoking either verb with no target and no waiver, stops working and destroys nothing, which is the intended failure.
- Every recipe, cookbook entry and skill runbook that teaches a destructive invocation must pass a waiver or a target, and a gate scans prose for the ones that do not.
- Adding a destructive verb costs one register entry plus the four documentation anchors the class gate demands.
- Out of scope: interactive confirmation prompts. This is a one-shot CLI with no TTY guarantee, so a prompt would either hang an agent or be skipped silently.

## Related
- ADR 0002 error model (the `usage` kind and exit 64) · ADR 0006 type-system posture (`PathSource` as a domain type)
- `src/cli/destructive.rs` · `tests/policy/etd.rs` · `docs/schemas/cache-clear.schema.json` · `docs/schemas/config-init.schema.json`
- [Configuration](../CONFIGURATION.md) · [Migration](../MIGRATION.md)
