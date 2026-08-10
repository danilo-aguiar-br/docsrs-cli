#!/usr/bin/env bash
# Local cross-target gate (no GHA). Exit 1 if any installed target fails cargo check.
#
# Every non-Linux target is checked explicitly rather than by proxy. An earlier
# comment here claimed windows-gnu stood in for msvc and that macOS needed no
# target at all; both claims were unfalsifiable while zero cross targets ran, and
# the target list contradicted the second one by listing aarch64-apple-darwin.
#
# Fail-closed on zero coverage (NC-1): the previous version counted a *skipped*
# target as a pass, so on a host without the windows target installed the gate
# printed green while verifying no non-Linux code at all — the exact blind spot it
# was written to close. Skipping is now an explicit decision, never a default.
#
# Measured on 2026-08-09, with the C provider temporarily removed: all four
# non-Linux targets checked clean with no compiler installed. That run is the
# evidence that this gate's only barrier is `ring`, not any property of this
# crate — and it is why GAP-TOOLCHAIN-001 now carries a measured cost instead of
# an argued one. The provider was restored because both pure-Rust candidates
# failed on other grounds (CPU floor; certificate-validation advisories), so the
# probe below is back with it.
#
# Measured again on 2026-08-10, with the C provider in place: msvc checks clean
# through cargo-xwin, so #[cfg(windows)] is type-checked on this host after all.
# gnu and darwin stay skipped for toolchains that genuinely are not here.
#
#   ./scripts/check-targets.sh                    # fails when no non-Linux target ran
#   ./scripts/check-targets.sh --allow-no-cross   # accept host without cross toolchain
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
      echo "check-targets: unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

# This list and the support matrix in docs/CROSS_PLATFORM.md must hold the same
# targets, and a policy gate enforces that. They disagreed until 2026-08-10: the
# matrix claimed `aarch64-unknown-linux-gnu` as supported while this gate never
# looked at it, and this gate checked `x86_64-pc-windows-gnu` while the matrix
# never mentioned it. Two lists of "the targets we support" that nobody compared.
targets=(
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  x86_64-pc-windows-gnu
  x86_64-pc-windows-msvc
  aarch64-apple-darwin
  x86_64-apple-darwin
)

# Targets whose coverage is the whole point of this gate. The host target alone
# proves nothing about #[cfg(windows)].
non_linux_targets=(
  x86_64-pc-windows-gnu
  x86_64-pc-windows-msvc
  aarch64-apple-darwin
  x86_64-apple-darwin
)

# The Rust target being installed is necessary, not sufficient: `ring` compiles
# crypto/*.c through cc-rs, so every cross target also needs a C toolchain able
# to target it. A missing toolchain is a host limitation, not a defect in this
# crate, so it is a skip with its reason named — never a FAILED.
#
# The msvc probe used to ask `command -v lib.exe`. `lib.exe` is Microsoft's
# archiver and cannot exist on Linux, so that condition was unsatisfiable: msvc
# was skipped on every run that would ever happen, while the hint three functions
# below already named the remedy that works. Two audits read `cross_checked=0`,
# read the hint, and still recorded "no non-Linux target is verifiable on this
# host" — a claim generalised from a probe that had been rigged to fail.
#
# Measured 2026-08-10, with cargo-xwin already present:
# `cargo xwin check --locked --all-targets --target x86_64-pc-windows-msvc`
# exits 0 over the whole graph, ring included. The barrier was the question.
#
# Real barriers that remain on this host, both outside this crate:
#
#   x86_64-pc-windows-gnu   needs mingw64-gcc (dnf, requires root)
#   *-apple-darwin          cc: unrecognized command-line option '-arch';
#                           needs an Apple SDK, which zig does not ship
# Measured again on 2026-08-10, with `zig` and `cargo-zigbuild` installed (no
# root, both under $HOME): `x86_64-pc-windows-gnu` and `aarch64-unknown-linux-gnu`
# build clean, ring included. Cross coverage went from 1 target to 3.
#
# The Apple measurement corrected a belief this file used to state. `ring` is NOT
# what blocks the Apple targets: zig compiles ring's C for both of them and the
# build reaches the LINK step. What fails there is:
#
#   error: unable to find framework 'CoreFoundation'
#   error: unable to find framework 'Security'
#   error: unable to find framework 'SystemConfiguration'
#
# Those come from `rustls-platform-verifier` (via security-framework /
# core-foundation) and reqwest's `system-configuration` — and the first of those
# is the crate reqwest forces into the graph unconditionally through
# `rustls-no-provider`, which no code path in this product consults. The blocker
# is Apple SDK frameworks at link time, not the C provider, and zig does not
# redistribute those frameworks.
cross_tools_for() {
  case "$1" in
    x86_64-pc-windows-gnu) echo "cargo-zigbuild zig" ;;
    x86_64-pc-windows-msvc) echo "cargo-xwin clang-cl llvm-lib" ;;
    aarch64-apple-darwin|x86_64-apple-darwin) echo oa64-clang ;;
    aarch64-unknown-linux-gnu) echo "cargo-zigbuild zig" ;;
    *) echo cc ;;
  esac
}

# msvc is driven through cargo-xwin, which supplies the fetched CRT and points
# cc-rs at clang-cl and llvm-lib in place of cl.exe and lib.exe. Every other
# target invokes cargo directly, because a plain host cc already suits it.
check_driver_for() {
  case "$1" in
    x86_64-pc-windows-msvc) echo xwin ;;
    x86_64-pc-windows-gnu|aarch64-unknown-linux-gnu) echo zigbuild ;;
    *) echo cargo ;;
  esac
}

# The hint must name a remedy that exists.
#
# The previous msvc hint read "not reachable from Linux", which is false and cost
# three audits: each one read it, believed the target was unreachable and stopped
# investigating. `cargo-xwin` fetches the MSVC headers and libraries into
# ~/.cache with no root, and `llvm-lib` / `clang-cl` stand in for `lib.exe` and
# `cl.exe`. A gate that misstates its own remedy ends the investigation that
# would have fixed it.
cross_cc_hint() {
  case "$1" in
    x86_64-pc-windows-gnu) echo "  cargo install cargo-zigbuild --locked && install zig in ~/.local/bin (no root)" ;;
    x86_64-pc-windows-msvc) echo "  cargo install cargo-xwin && dnf install clang-tools-extra llvm (clang-cl, llvm-lib; SDK lands in ~/.cache)" ;;
    aarch64-apple-darwin|x86_64-apple-darwin) echo "  needs Apple SDK FRAMEWORKS at link time (CoreFoundation, Security, SystemConfiguration); zig compiles ring fine but does not redistribute them — osxcross with a real SDK is the only route" ;;
    aarch64-unknown-linux-gnu) echo "  cargo install cargo-zigbuild --locked && install zig in ~/.local/bin (no root)" ;;
    *) echo "  install a C compiler for this target" ;;
  esac
}

installed="$(rustup target list --installed)"
checked=0
skipped=0
failed=0
cross_checked=0

is_non_linux() {
  printf '%s\n' "${non_linux_targets[@]}" | rg -qxF -- "$1"
}

for t in "${targets[@]}"; do
  if ! printf '%s\n' "$installed" | rg -qxF -- "$t"; then
    echo "check-targets: skip $t (target not installed)" >&2
    echo "  rustup target add $t" >&2
    cross_cc_hint "$t" >&2
    skipped=$((skipped + 1))
    continue
  fi
  missing=""
  for tool in $(cross_tools_for "$t"); do
    command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
  done
  if [ -n "$missing" ]; then
    echo "check-targets: skip $t (missing C cross toolchain:$missing)" >&2
    # Printed only where it is true. This line used to be unconditional, so an
    # Apple skip emitted "ring compiles crypto/*.c" immediately above a hint
    # saying the blocker is Apple SDK frameworks and that zig compiles ring
    # fine. Two contradictory causes in one skip, and the false one came first
    # — which is how the ring folklore survived the correction that removed it
    # everywhere else (GAP-APPLE-CAUSE-001).
    case "$t" in
      aarch64-apple-darwin|x86_64-apple-darwin) : ;;
      *) echo "  ring compiles crypto/*.c, so the Rust target alone is not enough" >&2 ;;
    esac
    cross_cc_hint "$t" >&2
    skipped=$((skipped + 1))
    continue
  fi
  driver="$(check_driver_for "$t")"
  echo "check-targets: check --target $t (driver: $driver)" >&2
  checked=$((checked + 1))
  check_ok=0
  case "$driver" in
    xwin)
      if cargo xwin check --locked --all-targets --target "$t"; then check_ok=1; fi
      ;;
    # cargo-zigbuild exposes no `check` subcommand, so this builds instead. That
    # is a superset of the type check this gate exists for: it also links, which
    # is precisely where the Apple targets fail and windows-gnu does not.
    zigbuild)
      if cargo zigbuild --locked --target "$t"; then check_ok=1; fi
      ;;
    *)
      if cargo check --locked --all-targets --target "$t"; then check_ok=1; fi
      ;;
  esac
  if [ "$check_ok" -eq 1 ]; then
    if is_non_linux "$t"; then
      cross_checked=$((cross_checked + 1))
    fi
  else
    echo "check-targets: FAILED $t" >&2
    failed=$((failed + 1))
  fi
done

echo "check-targets: checked=$checked skipped=$skipped failed=$failed cross_checked=$cross_checked"

if [ "$failed" -ne 0 ]; then
  exit 1
fi

if [ "$cross_checked" -eq 0 ]; then
  if [ "$allow_no_cross" -eq 1 ]; then
    echo "check-targets: WARNING zero non-Linux targets verified (--allow-no-cross)" >&2
    echo "check-targets: #[cfg(windows)] in src/shutdown/signals.rs stays unchecked on this host" >&2
    exit 0
  fi
  echo "check-targets: FAIL zero non-Linux targets verified" >&2
  echo "  this gate exists to type-check #[cfg(windows)]; a green run with zero" >&2
  echo "  cross coverage is a false pass, not a success" >&2
  echo "  install the toolchain above, or re-run with --allow-no-cross" >&2
  exit 1
fi

exit 0
