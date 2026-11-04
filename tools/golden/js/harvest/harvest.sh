#!/usr/bin/env bash
# Harvest call inputs for the given FrankMD lib modules from FrankMD's vitest suite.
# usage: harvest.sh <out.jsonl> mod1 mod2 ...
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
export FRANKMD="${FRANKMD:-$HOME/Projects/FrankMD}"
OUT="$1"; shift
TMP="${GOLDEN_TMP:-$(mktemp -d)}/harvest-wrappers"
rm -rf "$TMP"; mkdir -p "$TMP"
# HARVEST_WRAPPER_NODE_OPTS: extra node flags for export enumeration only
# (e.g. "--import runtime/setup.mjs" for modules with importmap-only bare imports)
node ${HARVEST_WRAPPER_NODE_OPTS:-} "$HERE/make_wrappers.mjs" "$TMP" "$@"
export HARVEST_WRAPPERS="$TMP" HARVEST_MODS="$(IFS=,; echo "$*")" HARVEST_OUT="$OUT"
: > "$OUT"
cd "$FRANKMD"
node "$FRANKMD/node_modules/vitest/vitest.mjs" run --config "$HERE/vitest.harvest.config.mjs" --reporter=dot "${VITEST_FILTER:-}" >&2 || echo "WARN: some vitest tests failed under harvest" >&2
wc -l "$OUT" >&2
