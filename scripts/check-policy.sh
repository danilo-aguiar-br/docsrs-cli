#!/usr/bin/env bash
# Local policy greps (no GHA). Exit 1 on product-env teaching / product env reads.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
fail=0
# Teach env path as live feature (allow NEVER/removed/historical)
if rg -n 'still allows|ainda permite|still work|ainda funcionam|Path sandbox still|Path sandbox ainda' \
  docs/ skills/ README.md README.pt-BR.md CLAUDE.md 2>/dev/null; then
  echo "policy: docs still teach path sandbox env as live feature" >&2
  fail=1
fi
if rg -n 'DOCSRS_CLI_HOME=' docs/ skills/ CLAUDE.md 2>/dev/null | rg -v 'NEVER|removed|nunca|proibido|forbidden|historical' ; then
  echo "policy: DOCSRS_CLI_HOME= examples remain" >&2
  fail=1
fi
# Product src must not read DOCSRS_CLI_
if rg -n 'DOCSRS_CLI_' src/ --type rust | rg -v '//!|///|// ' ; then
  echo "policy: product source references DOCSRS_CLI_ outside comments" >&2
  fail=1
fi
if test -d .github; then
  echo "policy: .github must not exist" >&2
  fail=1
fi
exit "$fail"
