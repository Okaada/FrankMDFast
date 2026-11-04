#!/usr/bin/env bash
# Regenerate every golden fixture from the original FrankMD checkout.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
export FRANKMD="${FRANKMD:-$HOME/Projects/FrankMD}"
export GOLDEN_TMP="${GOLDEN_TMP:-$(mktemp -d)}"
echo "FrankMD @ $(git -C "$FRANKMD" rev-parse HEAD)"
run_rb() { (cd "$FRANKMD" && env "$@" mise exec ruby@3.4.8 -- bundle exec rails runner "$HERE/ruby/$RB.rb"); }
for RB in config hugo media_usage media omarchy_theme translations; do RB=$RB run_rb GOLDEN_TMP="$GOLDEN_TMP"; done
for RB in path_safety notes_tree notes_crud notes_rename search recent_files backlinks helpers; do
  RB=$RB run_rb RAILS_ENV=test TZ=UTC FRANKMD_LOCALE=en GOLDEN_TMP="$GOLDEN_TMP"
done
cd "$ROOT"
for JS in utils render ui_data draft_storage image_sources; do node "$HERE/js/$JS.mjs"; done
test -z "$(git -C "$FRANKMD" status --short)" || { echo "ERROR: FrankMD tree is dirty" >&2; exit 1; }
