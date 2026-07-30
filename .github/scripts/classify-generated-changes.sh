#!/usr/bin/env bash
set -euo pipefail

changed="$(cat)"
matches() { grep -Eq "$1" <<< "$changed"; }

# Only these documentation/community paths are proven isolated from generated
# output and runtime contracts. Everything else (including new or mixed paths)
# is deliberately fail-closed.
isolated='^(docs/book/.*|CODE_OF_CONDUCT\.md|CONTRIBUTING\.md|FAQ\.md|GOVERNANCE\.md|SECURITY\.md|VERSIONING\.md)$'
non_isolated="$(grep -Ev "$isolated" <<< "$changed" || true)"
if [[ -n "$changed" && -z "$non_isolated" ]]; then
  ts_runtime=false
  go_runtime=false
  generated_targets=false
else
  ts_runtime=true
  go_runtime=true
  generated_targets=true
fi

matches '\.vexil$' && schema=true || schema=false

printf 'ts_runtime=%s\n' "$ts_runtime"
printf 'go_runtime=%s\n' "$go_runtime"
printf 'schema=%s\n' "$schema"
printf 'generated_targets=%s\n' "$generated_targets"
