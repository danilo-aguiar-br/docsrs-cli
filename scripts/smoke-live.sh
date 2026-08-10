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
expected_version="$(cargo metadata --no-deps --format-version 1 --manifest-path "$ROOT/Cargo.toml" | jaq -r '.packages[0].version')"
ver="$(run version --json)"
if ! echo "$ver" | rg -qF -- '"version":"'"$expected_version"'"'; then
  echo "version mismatch: Cargo.toml says $expected_version, binary reported: $ver" >&2
  exit 1
fi

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
if ! echo "$item" | rg -qF -- '"extraction":"method"'; then
  echo "get-item: extraction is not method-scoped: $item" >&2
  exit 1
fi
if echo "$item" | rg -qF -- '# Struct Runtime'; then
  echo "get-item: method markdown contains the parent struct dump" >&2
  exit 1
fi
# Prefer method-scoped content (not full parent struct dump).
md_len="$(echo "$item" | jaq -r '.data.markdown | length' 2>/dev/null || echo 0)"
if [[ "${md_len:-0}" -gt 25000 ]]; then
  echo "WARN: method markdown still large ($md_len bytes)" >&2
fi

echo "== get-item required trait method (tymethod anchor) =="
# BUG-01 regression guard. The case above is an *inherent* method on a struct,
# which is the only anchor flavour the whole corpus used to cover. A required
# trait method lives at `#tymethod.NAME`, and probing only `method.NAME` made
# Iterator::next answer HTTP 200 → exit 66.
req="$(run get-item std method iter::Iterator::next --json)"
if ! echo "$req" | rg -qF -- '"extraction":"method"'; then
  echo "get-item: required trait method is not method-scoped: $req" >&2
  exit 1
fi
if ! echo "$req" | rg -qF -- '#tymethod.next'; then
  echo "get-item: source_url must point at the anchor that exists: $req" >&2
  exit 1
fi
# Control in the same trait: a provided method must keep the `method.` anchor.
prov="$(run get-item std method iter::Iterator::map --json)"
if ! echo "$prov" | rg -qF -- '#method.map'; then
  echo "get-item: provided trait method lost its method. anchor: $prov" >&2
  exit 1
fi

echo "== get-item associated type (associatedtype anchor) =="
# GAP-ASSOCITEM-001 regression guard. Fixing tymethod closed one member category
# and left its siblings behind: `Iterator::Item` still built
# std/iter/Iterator/type.Item.html, a path rustdoc has never emitted.
assoc_ty="$(run get-item std type iter::Iterator::Item --json)"
if ! echo "$assoc_ty" | rg -qF -- '#associatedtype.Item'; then
  echo "get-item: associated type must resolve on the parent trait page: $assoc_ty" >&2
  exit 1
fi
if ! echo "$assoc_ty" | rg -qF -- '"extraction":"method"'; then
  echo "get-item: associated type is not anchor-scoped: $assoc_ty" >&2
  exit 1
fi

echo "== get-item associated constant (associatedconstant anchor) =="
assoc_const="$(run get-item std const time::Duration::MAX --json)"
if ! echo "$assoc_const" | rg -qF -- '#associatedconstant.MAX'; then
  echo "get-item: associated constant must resolve on the parent page: $assoc_const" >&2
  exit 1
fi

echo "== get-item enum variant (variant anchor) =="
variant="$(run get-item std variant option::Option::Some --json)"
if ! echo "$variant" | rg -qF -- 'enum.Option.html#variant.Some'; then
  echo "get-item: enum variant must resolve on the enum page: $variant" >&2
  exit 1
fi

echo "== get-item struct field (structfield anchor) =="
field="$(run get-item std structfield ops::Range::start --json)"
if ! echo "$field" | rg -qF -- 'struct.Range.html#structfield.start'; then
  echo "get-item: struct field must resolve on the struct page: $field" >&2
  exit 1
fi

echo "== member-only kind without a parent fails closed =="
# Falling through to the free-item branch would build `variant.Some.html`, a URL
# rustdoc has never served — an HTTP 404 dressed up as a plan.
set +e
run get-item std variant Some --json >/tmp/docsrs-smoke-variant.json 2>&1
bare_variant_exit=$?
set -e
if [ "$bare_variant_exit" -ne 65 ]; then
  echo "get-item: unqualified variant must exit 65, got $bare_variant_exit" >&2
  exit 1
fi
rm -f /tmp/docsrs-smoke-variant.json

echo "== stderr prose is localized, wire message stays English =="
pt_line="$(run --lang pt-BR --format text get-item serde widget Foo 2>&1 >/dev/null || true)"
if ! echo "$pt_line" | rg -qF -- 'tipo de item desconhecido'; then
  echo "i18n: pt-BR stderr must be localized: $pt_line" >&2
  exit 1
fi
wire_msg="$(run --lang pt-BR get-item serde widget Foo --json 2>/dev/null || true)"
if ! echo "$wire_msg" | rg -qF -- "unknown item type 'widget'"; then
  echo "i18n: JSON message must stay English under --lang pt-BR: $wire_msg" >&2
  exit 1
fi

echo "== primitive constant keeps the legacy free-item page =="
# REGRESSION GUARD: `u32::MAX` is an associated constant on a primitive, but std
# still serves it from the module page. The uppercase-parent rule is what keeps
# this lookup — which works today — off the parent-anchor path.
prim_const="$(run get-item std const u32::MAX --json)"
# Assert on the resolved field, never on the whole envelope: this page's own
# markdown links to #associatedconstant.MAX ("use the associated constant
# instead"), so a substring match over the body would pass for the wrong reason.
prim_url="$(echo "$prim_const" | jaq -r '.data.source_url')"
if [[ "$prim_url" != "https://doc.rust-lang.org/stable/std/u32/constant.MAX.html" ]]; then
  echo "get-item: primitive constant left its legacy page: $prim_url" >&2
  exit 1
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

echo "== the file and the flag agree on the rule, differ on the remedy =="
# GAP-TOML-001. The flag rejected a zero timeout while config.toml accepted it,
# so one knob had two answers. Both must fail now; only the exit code differs,
# because the operator edits a file and the caller edits a command.
for key in timeout_secs connect_timeout_secs log_directive; do
  toml_cfg="$(mktemp -d)"
  case "$key" in
    log_directive) printf '%s = "docsrs_cli=not_a_level"\n' "$key" > "$toml_cfg/config.toml" ;;
    *)             printf '%s = 0\n' "$key" > "$toml_cfg/config.toml" ;;
  esac
  set +e
  "$BIN" --config-dir "$toml_cfg" --cache-dir "$CACHE" version --json >/dev/null 2>&1
  toml_code=$?
  set -e
  rm -rf "$toml_cfg"
  if [[ "$toml_code" -ne 78 ]]; then
    echo "config.toml: $key must fail closed with exit 78, got $toml_code" >&2
    exit 1
  fi
done

echo "== host locale steers stderr prose but never the wire =="
# GAP-I18N-003. The rustdoc claimed no environment read reached locale
# resolution; sys_locale reads LC_ALL/LC_MESSAGES/LANG, and these two lines are
# the proof. The behaviour is correct and stays — what must never drift is the
# wire staying English under either value.
pt_prose="$(LANG=pt_BR.UTF-8 LC_ALL=pt_BR.UTF-8 run --format text get-item serde widget Foo 2>&1 >/dev/null || true)"
en_prose="$(LANG=C LC_ALL=C run --format text get-item serde widget Foo 2>&1 >/dev/null || true)"
if [[ "$pt_prose" == "$en_prose" ]]; then
  echo "i18n: host locale no longer reaches stderr prose: $pt_prose" >&2
  exit 1
fi
for lang_tag in pt_BR.UTF-8 C; do
  wire="$(LANG="$lang_tag" LC_ALL="$lang_tag" run get-item serde widget Foo --json 2>/dev/null || true)"
  if ! echo "$wire" | rg -qF -- "unknown item type 'widget'"; then
    echo "i18n: wire message must stay English under LANG=$lang_tag: $wire" >&2
    exit 1
  fi
done

echo "== top-N of a filtered set, with no jaq stage =="
# GAP-AGENT-001. Filtering without a limit returns every match; limiting without a
# sort returns an arbitrary slice. Both flags together are what removes the pipe.
topn="$(run --filter kind=struct --sort-by name --max-items 5 --select name \
  search-in-crate serde "" --limit 200 --json)"
n_hits="$(echo "$topn" | jaq -r '.data.hits | length')"
if [[ "$n_hits" -ne 5 ]]; then
  echo "agent-ops: --max-items 5 must emit 5 hits, got $n_hits" >&2
  exit 1
fi
if ! echo "$topn" | rg -qF -- '"limited":true'; then
  echo "agent-ops: a cut set must report limited=true: $topn" >&2
  exit 1
fi
# Ascending by name, verified against the emitted order itself.
if ! echo "$topn" | jaq -e '.data.hits | map(.name) | . == sort' >/dev/null; then
  echo "agent-ops: --sort-by name must emit ascending names: $topn" >&2
  exit 1
fi

echo "== emitted follows the array, total keeps describing upstream =="
# GAP-COUNT-001. `emitted` is published as hits actually emitted, so reduction has
# to rewrite it; `total` counts the upstream index and must survive untouched.
reduced="$(run --filter kind=struct search-in-crate serde "" --limit 200 --json)"
if ! echo "$reduced" | jaq -e '.data | (.emitted == (.hits | length)) and (.total >= .emitted)' >/dev/null; then
  echo "agent-ops: emitted must match hits after reduction: $reduced" >&2
  exit 1
fi

echo "== an unordered key is a no-op, and a failure is never reduced =="
if ! run --sort-by no_such_key --max-items 3 search-crates serde --per-page 3 --json >/dev/null; then
  echo "agent-ops: sorting on an absent key must not fail" >&2
  exit 1
fi
set +e
err="$(run --sort-by name --max-items 1 get-item serde widget Foo --json 2>/dev/null)"
err_code=$?
set -e
if [[ "$err_code" -ne 65 ]]; then
  echo "agent-ops: reduction must not change a failure's exit code, got $err_code" >&2
  exit 1
fi
if echo "$err" | rg -qF -- 'agent_surface'; then
  echo "agent-ops: a failure envelope must reach the caller untouched: $err" >&2
  exit 1
fi

echo "smoke-live: OK"
