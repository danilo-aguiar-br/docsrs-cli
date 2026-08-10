#!/usr/bin/env bash
# Local supply-chain gate (no GHA). Exit 1 on a banned crate or a known advisory.
#
# `deny.toml` has existed since Camada M and its own header says it is a
# "maintainer tool — not wired to CI (product forbids CI/CD in-tree)". That was
# accurate and it was also the problem: a policy that runs only when somebody
# remembers is a policy that reports nothing on the day it matters. The rules in
# that file are the product's TLS posture (ADR 0007) written as enforceable
# bans, so they deserve the same treatment as the other gates in this directory.
#
# This is still local and still invokable by hand. It is not CI.
#
#   ./scripts/check-supply.sh              # fail on missing tooling
#   ./scripts/check-supply.sh --allow-missing-tools
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

allow_missing=0
for arg in "$@"; do
  case "$arg" in
    --allow-missing-tools) allow_missing=1 ;;
    -h|--help)
      echo "usage: $0 [--allow-missing-tools]" >&2
      exit 0
      ;;
    *)
      echo "check-supply: unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

fail=0
ran=0

# Fail closed when the tool is absent, mirroring check-targets.sh.
#
# A skipped check that returns 0 is the failure mode NC-1 named: the run looks
# green while nothing was verified. `--allow-missing-tools` makes accepting that
# an explicit decision on a host without the tooling, never the default.
require_tool() {
  local tool="$1" install="$2"
  if command -v "$tool" >/dev/null 2>&1; then
    return 0
  fi
  echo "check-supply: $tool not installed" >&2
  echo "  $install" >&2
  if [ "$allow_missing" -eq 1 ]; then
    echo "check-supply: WARNING skipping $tool (--allow-missing-tools)" >&2
    return 1
  fi
  fail=1
  return 1
}

# Bans and licence policy from deny.toml. The bans are the enforceable half of
# ADR 0007: native-tls, openssl and the C-backed crypto providers must never
# reappear through a transitive feature pull.
if require_tool cargo-deny "cargo install cargo-deny --locked"; then
  echo "check-supply: cargo deny check" >&2
  if cargo deny check; then
    ran=$((ran + 1))
  else
    echo "check-supply: FAILED cargo deny check" >&2
    fail=1
  fi
fi

# Known advisories against the resolved lockfile. Distinct from the bans above:
# deny.toml answers "is this crate allowed here", audit answers "is this version
# known to be vulnerable today".
if require_tool cargo-audit "cargo install cargo-audit --locked"; then
  echo "check-supply: cargo audit" >&2
  if cargo audit; then
    ran=$((ran + 1))
  else
    echo "check-supply: FAILED cargo audit" >&2
    fail=1
  fi
fi

echo "check-supply: ran=$ran failed=$fail"

if [ "$fail" -ne 0 ]; then
  exit 1
fi

if [ "$ran" -eq 0 ]; then
  echo "check-supply: WARNING nothing verified (--allow-missing-tools)" >&2
fi

exit 0
