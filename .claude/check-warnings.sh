#!/bin/bash
# PostToolUse hook: run cargo check after .rs file edits and surface warnings
set -euo pipefail

file=$(jq -r '.tool_input.file_path // .tool_response.filePath // ""')
if [[ ! "$file" =~ \.rs$ ]]; then
  exit 0
fi

out=$(cargo check 2>&1) || true
warnings=$(echo "$out" | grep -cE '^warning\b' || true)

if [ "$warnings" -gt 0 ]; then
  ctx=$(echo "$out" | grep -E '(^warning|^ *-->)' | head -20)
  jq -n --arg ctx "cargo check found $warnings warning(s): $ctx" \
    '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":$ctx}}'
fi
