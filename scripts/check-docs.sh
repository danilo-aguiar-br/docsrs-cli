#!/usr/bin/env bash
# Compile the documentation this crate promises to publish (no GHA, by policy).
#
# Why this exists: on 2026-08-10 a pre-publish run found 83 broken intra-doc
# links across 29 of the 36 source files, and not one test in the tree could see
# them. `check-all.sh` discovered four gates and none of them invoked `cargo
# doc`; the policy suite reads source as *text* and never asks rustdoc whether
# the links resolve. That is GAP-DOC-LINKS-001, and its `### Estado` recorded the
# case as fixed and the class as open. This file closes the class.
#
# The lint set matters more than it looks. CONTRIBUTING used to teach
# `-D missing_docs -D rustdoc::broken_intra_doc_links`, which omits
# `rustdoc::private_intra_doc_links` — exactly the class that produced the last
# five errors of that run. A maintainer following the documented command to the
# letter would have seen green. `-D warnings` is the superset, so a lint rustdoc
# gains tomorrow is enforced the day it ships rather than the day someone
# remembers to add it here.
#
# `--cfg docsrs` is deliberately absent: `src/lib.rs` gates `feature(doc_cfg)`
# behind it, that feature is nightly-only, and this repository pins stable 1.88.0
# in rust-toolchain.toml. Passing it would fail the build for a reason that has
# nothing to do with the documentation.
#
#   ./scripts/check-docs.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

for arg in "$@"; do
  case "$arg" in
    -h|--help)
      echo "usage: $0" >&2
      exit 0
      ;;
    *)
      echo "check-docs: unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

# Probe the toolchain before blaming the tree.
#
# Without this, a host without `cargo` reports "documentation failed to build" —
# a message that sends the reader hunting for a broken link that does not exist.
# `check-all.sh` learned the same lesson about `fd`; writing a new gate with the
# disease it was written to cure is how the class survives.
if ! command -v cargo >/dev/null 2>&1; then
  echo "check-docs: FAIL cargo is not installed; the documentation cannot be built" >&2
  exit 1
fi

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

echo "check-docs: RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked" >&2
status=0
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked >"$log" 2>&1 || status=$?

# Counted from the log rather than inferred from the exit code, so the footer
# says how much is broken instead of only that something is.
#
# Bash pattern matching, not grep or rg: this gate guards a crate that ships to
# Linux, macOS and Windows, and the policy suite was moved out of shell precisely
# because it was only verifiable where a pile of external tools happened to
# exist. A counter that needs one more binary installed is a counter that reports
# nothing on the host that lacks it.
errors=0
warnings=0
while IFS= read -r line; do
  case "$line" in
    error:* | 'error['*) errors=$((errors + 1)) ;;
    warning:* | 'warning['*) warnings=$((warnings + 1)) ;;
  esac
done <"$log"

echo "check-docs: errors=$errors warnings=$warnings"

if [ "$status" -ne 0 ]; then
  echo "check-docs: FAILED cargo doc (exit $status)" >&2
  while IFS= read -r line; do printf '%s\n' "$line" >&2; done <"$log"
  exit 1
fi

# A clean exit with warnings would mean `-D warnings` stopped being honoured. The
# flag is the whole gate, so its silence is itself a finding.
if [ "$warnings" -ne 0 ]; then
  echo "check-docs: FAIL rustdoc emitted $warnings warning(s) but still exited 0" >&2
  echo "  -D warnings is no longer being applied" >&2
  while IFS= read -r line; do printf '%s\n' "$line" >&2; done <"$log"
  exit 1
fi

exit 0
