#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
classifier="$script_dir/classify-generated-changes.sh"

assert_case() {
  local paths="$1"
  shift
  local output
  output="$(printf '%s\n' "$paths" | bash "$classifier")"
  for expected in "$@"; do
    if ! grep -Fxq "$expected" <<< "$output"; then
      printf 'classification mismatch for %s\nexpected: %s\nactual:\n%s\n' \
        "$paths" "$expected" "$output" >&2
      return 1
    fi
  done
}

assert_case 'tools/generated-code-checks/check-generated-wire.mjs' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case 'compliance/vectors/annotations.json' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case 'corpus/projects/trait_alias/app/root.vexil' \
  'ts_runtime=true' 'go_runtime=true' 'schema=true' 'generated_targets=true'
assert_case '.github/scripts/classify-generated-changes.sh' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case 'docs/book/src/introduction.md' \
  'ts_runtime=false' 'go_runtime=false' 'schema=false' 'generated_targets=false'
assert_case 'Cargo.toml' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case 'Cargo.lock' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case 'rust-toolchain.toml' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case 'crates/vexil-runtime/src/lib.rs' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case 'new-unclassified-path.txt' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'
assert_case $'docs/book/src/introduction.md\nCargo.toml' \
  'ts_runtime=true' 'go_runtime=true' 'schema=false' 'generated_targets=true'

printf 'Generated change classifier tests passed.\n'
