#!/usr/bin/env bash
# cargo-release pre-release-hook: generates this crate's new changelog
# section with git-cliff and inserts it into CHANGELOG.md, right after the
# "# Changelog" title, above the previous newest entry.
#
# cargo-release runs this with cwd = the crate's own directory and provides
# CRATE_NAME, NEW_VERSION, PREV_VERSION, WORKSPACE_ROOT, CRATE_ROOT, DRY_RUN
# as env vars. There is no PREV_TAG — it's reconstructed from PREV_VERSION
# using this repo's "<crate>-v<version>" tag convention, and cargo-release
# still executes this hook during --dry-run (it relies on the hook itself
# to respect DRY_RUN), so writes are skipped when DRY_RUN=true.
set -euo pipefail

cd "$WORKSPACE_ROOT"

changelog="${CRATE_ROOT}/CHANGELOG.md"
prev_tag="${CRATE_NAME}-v${PREV_VERSION}"
if git rev-parse --verify --quiet "refs/tags/${prev_tag}" >/dev/null; then
  range="${prev_tag}..HEAD"
else
  range="HEAD"
fi

if [ ! -f "$changelog" ]; then
  printf '# Changelog\n' > "$changelog"
fi

section=$(git-cliff --config cliff.toml \
  --include-path "crates/${CRATE_NAME}/**" \
  --unreleased \
  --tag "$NEW_VERSION" \
  "$range" \
  | awk '/^## /{found=1} found')

if [ -z "$section" ]; then
  echo "warning: git-cliff produced no entries for ${CRATE_NAME} (${range}) — skipping changelog update" >&2
  exit 0
fi

if [ "${DRY_RUN:-false}" = "true" ]; then
  echo "--- dry-run: would insert into ${changelog} ---"
  echo "$section"
  exit 0
fi

awk -v section="$section" '
  NR==1 && /^# / { print; print ""; print section; print ""; next }
  { print }
' "$changelog" > "${changelog}.tmp"
mv "${changelog}.tmp" "$changelog"

git add "$changelog"
