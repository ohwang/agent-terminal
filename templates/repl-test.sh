#!/usr/bin/env bash
set -euo pipefail

# REPL/readline app test template
# Usage: bash repl-test.sh
#
# For testing Python REPL, Node REPL, irb, psql, sqlite3, or any
# line-based interactive program that uses a prompt.
#
# Key differences from full-screen TUI testing:
# - Output scrolls (use scrollback to find past output)
# - Wait for the prompt before sending input
# - Use --text to match prompt patterns (>>>, $, >, etc.)

SESSION="test-repl-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== REPL Test ==="

# 1. Launch the REPL
agent-terminal open "python3" --session "$SESSION"

# 2. Wait for the prompt to appear
agent-terminal wait --text ">>>" --session "$SESSION" --timeout 10000
echo "Python REPL ready"

# 3. Execute a simple expression
agent-terminal type "2 + 2" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text "4" --session "$SESSION" --timeout 3000
agent-terminal assert --text "4" --session "$SESSION"
echo "✓ Basic arithmetic works"

# 4. Wait for prompt to return
agent-terminal wait --text ">>>" --session "$SESSION" --timeout 3000

# 5. Test a multi-line command
agent-terminal type "for i in range(3):" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text "..." --session "$SESSION" --timeout 3000

agent-terminal type "    print(f'item {i}')" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text "..." --session "$SESSION" --timeout 3000

# Empty line to end the block
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text "item 2" --session "$SESSION" --timeout 5000
echo "✓ Multi-line code works"

# 6. Check scrollback for earlier output
agent-terminal scrollback --search "item 0" --session "$SESSION"
echo "✓ Scrollback search works"

# 7. Test import
agent-terminal wait --text ">>>" --session "$SESSION" --timeout 3000
agent-terminal type "import sys; print(sys.version)" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --regex "\\d+\\.\\d+\\.\\d+" --session "$SESSION" --timeout 3000
echo "✓ Import works"

# 8. Take a snapshot for reference
agent-terminal snapshot --session "$SESSION"

# 9. Clean exit
agent-terminal wait --text ">>>" --session "$SESSION" --timeout 3000
agent-terminal type "exit()" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
sleep 1

# 10. Verify clean exit
EXIT_CODE=$(agent-terminal exit-code --session "$SESSION" 2>/dev/null || echo "unknown")
echo "Exit code: $EXIT_CODE"

agent-terminal close --session "$SESSION"
trap - EXIT

echo "=== REPL test passed ==="
