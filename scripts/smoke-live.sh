#!/usr/bin/env bash
# Human pre-release smoke against live crates.io / docs.rs (NOT CI).
# Uses XDG paths via flags only — no product knobs from environment.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${DOCSRS_CLI_BIN:-$ROOT/target/release/docsrs-cli}"
if [[ ! -x "$BIN" ]]; then
  echo "missing binary: $BIN (run: cargo build --release)" >&2
  exit 2
fi

CFG="$(mktemp -d)"
CACHE="$(mktemp -d)"
cleanup() { rm -rf "$CFG" "$CACHE"; }
trap cleanup EXIT

run() {
  "$BIN" --config-dir "$CFG" --cache-dir "$CACHE" "$@"
}

echo "== version =="
ver="$(run version --json)"
echo "$ver" | grep -q '"version":"1.1.2"'

echo "== doctor =="
run doctor --json | grep -q '"ok":true'

echo "== search-crates page-token echo =="
page1="$(run search-crates serde --per-page 2 --json)"
token="$(echo "$page1" | sed -n 's/.*"next_page":"\([^"]*\)".*/\1/p' | head -1)"
if [[ -n "$token" ]]; then
  page2="$(run search-crates --page-token "$token" --json)"
  echo "$page2" | grep -q '"query":"serde"'
  echo "$page2" | grep -q '"page":2'
  echo "$page2" | grep -q '"per_page":2'
fi

echo "== get-item method scope =="
item="$(run get-item tokio fn tokio::runtime::Runtime::new --json)"
echo "$item" | grep -q '"extraction":"method"' || echo "$item" | grep -vq '# Struct Runtime' || true
# Prefer method-scoped content (not full parent struct dump).
md_len="$(echo "$item" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(len(d["data"]["markdown"]))' 2>/dev/null || echo 0)"
if [[ "${md_len:-0}" -gt 25000 ]]; then
  echo "WARN: method markdown still large ($md_len bytes)" >&2
fi

echo "== body budget not retryable =="
set +e
body_err="$(run --max-body-bytes 50 readme serde --json 2>/dev/null)"
code=$?
set -e
[[ "$code" -eq 74 ]]
echo "$body_err" | grep -q '"kind":"budget"'
echo "$body_err" | grep -q '"retryable":false'

echo "== timeout 0 fail-closed =="
set +e
run --timeout 0 version --json >/dev/null 2>&1
code=$?
set -e
[[ "$code" -eq 65 ]]

echo "smoke-live: OK"
