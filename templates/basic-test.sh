#!/usr/bin/env bash
# Basic agent-terminal test template.
# Replace "./your-app" with the command to test,
# and update assertions to match your app's output.
set -euo pipefail

SESSION="test-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

# 1. Launch the application
agent-terminal open "./your-app" --session "$SESSION"

# 2. Wait for the first render to stabilize
agent-terminal wait --stable 500 --session "$SESSION"

# 3. Take an initial snapshot to see what's on screen
agent-terminal snapshot --session "$SESSION"

# 4. Verify the app started correctly
agent-terminal assert --text "Expected" --session "$SESSION"

# 5. Interact with the app
agent-terminal send "key" --session "$SESSION"

# 6. Wait for the result
agent-terminal wait --text "Result" --session "$SESSION" --timeout 5000

# 7. Verify the result
agent-terminal assert --text "Result" --session "$SESSION"

# 8. Clean up
agent-terminal close --session "$SESSION"
trap - EXIT

echo "Test passed"
