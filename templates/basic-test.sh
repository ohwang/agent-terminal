#!/usr/bin/env bash
set -euo pipefail

# Basic TUI test template
# Usage: bash basic-test.sh
#
# Replace "./your-app" with your application's run command.
# Replace "Expected text" / "New state" with actual expected content.

SESSION="test-basic-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== Basic TUI Test ==="

# 1. Launch the app
agent-terminal open "./your-app" --session "$SESSION" --size 80x24

# 2. Wait for the initial render to stabilize
agent-terminal wait --stable 500 --session "$SESSION"

# 3. Verify initial state
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --text "Expected text" --session "$SESSION"

# 4. Verify process is alive
agent-terminal status --session "$SESSION" --json

# 5. Interact
agent-terminal send "j" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# 6. Verify the interaction had the expected effect
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --text "New state" --session "$SESSION"

# 7. Test resize handling
agent-terminal resize 40 10 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal status --session "$SESSION" --json
agent-terminal snapshot --session "$SESSION"

# 8. Resize back
agent-terminal resize 80 24 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# 9. Clean exit
agent-terminal send "q" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# 10. Verify clean shutdown
EXIT_CODE=$(agent-terminal exit-code --session "$SESSION" 2>/dev/null || echo "unknown")
echo "Exit code: $EXIT_CODE"

# 11. Clean up
agent-terminal close --session "$SESSION"
trap - EXIT

echo "=== Test passed ==="
