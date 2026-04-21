#!/bin/sh
# Run rustfmt across the workspace before any git staging operation.
# Blocks staging if rustfmt exits non-zero (e.g. unparseable source).
input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // ""')
case "$cmd" in
  *"git add"*)
    cargo fmt
    echo "pre-stage-fmt: cargo fmt ran before git add"
    ;;
esac
