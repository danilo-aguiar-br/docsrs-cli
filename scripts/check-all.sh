#!/usr/bin/env bash
# Run every local gate in one command (no GHA, by product policy).
#
# Why this exists: `scripts/` held four gates while the two documents that tell a
# maintainer what to run — CONTRIBUTING.md "Release Process" and
# docs/TESTING.md "Recommended local gate" — named exactly one of them,
# `smoke-live.sh`. The three that actually enforce policy were reachable only by
# remembering they existed.
#
# That is the defect GAP-SUPPLY-001 was opened for, reproduced one level up:
# `deny.toml` was flagged because it "runs only when somebody remembers", and the
# gate written to fix it then landed outside the checklist and inherited the same
# failure. A gate nobody invokes and a policy nobody runs are the same thing.
#
# The list is discovered from the directory rather than kept here, so a new
# check-*.sh becomes part of this run by existing. `smoke-live.sh` is deliberately
# not a check-*.sh: it needs the network and a release build, so it stays an
# explicit pre-release step instead of a gate that fails when a host is down.
#
#   ./scripts/check-all.sh                    # every gate, fail-closed
#   ./scripts/check-all.sh --allow-no-cross   # forwarded to check-targets.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

allow_no_cross=0
for arg in "$@"; do
  case "$arg" in
    --allow-no-cross) allow_no_cross=1 ;;
    -h|--help)
      echo "usage: $0 [--allow-no-cross]" >&2
      exit 0
      ;;
    *)
      echo "check-all: unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

# Probe the discovery tool before blaming the directory.
#
# Without this, a host missing `fd` produced "discovered no gates under scripts/"
# — a message that indicts the repository for a tool that is merely absent, and
# sends the reader looking for deleted files. That is the same defect this run
# was auditing: GAP-GATE-003 was a gate whose failure text pointed away from its
# cause. Writing a new gate with the same disease is how the class survives.
if ! command -v fd >/dev/null 2>&1; then
  echo "check-all: FAIL fd is not installed; gate discovery cannot run" >&2
  echo "  cargo install fd-find    # the gates themselves are present" >&2
  exit 1
fi

ran=0
failed=0
declare -a broken=()

# The policy gates are Rust, not a script, so discovery by filename cannot find
# them. They used to be `scripts/check-policy.sh`: 540 lines of bash wrapping 260
# lines of inline Python, which violated the full-stack Rust rule outright and —
# less obviously — meant the policy of a crate promising Linux, macOS and Windows
# was only verifiable where bash, python3, fd and rg all existed.
#
# Named explicitly here rather than discovered, because "which cargo test targets
# are gates" is not derivable from the tree. A policy gate asserts the suite is
# named in the maintainer checklists, so it cannot quietly stop being invoked.
echo "check-all: === policy_gates (cargo test) ===" >&2
ran=$((ran + 1))
status=0
cargo test --test policy_gates --quiet || status=$?
if [ "$status" -ne 0 ]; then
  echo "check-all: FAILED policy_gates (exit $status)" >&2
  failed=$((failed + 1))
  broken+=("policy_gates")
fi

# Sorted so the report order is stable between runs and between machines.
while IFS= read -r gate; do
  name="$(basename "$gate")"
  [ "$name" = "check-all.sh" ] && continue
  echo "check-all: === $name ===" >&2
  ran=$((ran + 1))
  status=0
  if [ "$name" = "check-targets.sh" ] && [ "$allow_no_cross" -eq 1 ]; then
    bash "$gate" --allow-no-cross || status=$?
  else
    bash "$gate" || status=$?
  fi
  if [ "$status" -ne 0 ]; then
    echo "check-all: FAILED $name (exit $status)" >&2
    failed=$((failed + 1))
    broken+=("$name")
  fi
done < <(fd -t f -g 'check-*.sh' scripts/ | sort)

echo "check-all: ran=$ran failed=$failed"

# Zero gates discovered is a failure, not a pass. A rename or a moved directory
# would otherwise report green while checking nothing — the same false-green
# shape check-targets.sh was rewritten to reject at zero cross coverage.
if [ "$ran" -eq 0 ]; then
  echo "check-all: FAIL discovered no gates under scripts/" >&2
  exit 1
fi

if [ "$failed" -ne 0 ]; then
  printf 'check-all: failing gates: %s\n' "${broken[*]}" >&2
  exit 1
fi

exit 0
