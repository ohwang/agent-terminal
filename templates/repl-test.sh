#!/usr/bin/env bash
set -euo pipefail

# Template for testing REPL/readline apps
# Usage: bash repl-test.sh
#
# REPL apps (Python REPL, Node REPL, irb, ghci, etc.) are scrolling
# CLIs, not full-screen TUIs. They use readline for line editing and
# print output sequentially. Testing strategy differs from TUI apps:
#
# - Use scrollback to see past output
# - Use wait --text to detect prompt readiness
# - Use type for input (not send, since readline interprets keys)
# - Output scrolls off screen, so snapshot alone may miss earlier output
#
# Replace "python3" with your REPL command.
# Adjust the prompt pattern (e.g., ">>>", ">", "irb>", "ghci>").

SESSION="test-repl-$$"
PROMPT=">>>"   # Python REPL prompt -- change for your REPL

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== REPL Test ==="

# --- Setup ---

# REPLs are scrolling apps, so standard terminal size works fine
agent-terminal open "python3" --session "$SESSION" --size 80x24

# Wait for the REPL prompt to appear
agent-terminal wait --text "$PROMPT" --session "$SESSION" --timeout 15000
agent-terminal snapshot --session "$SESSION"

# --- Basic Input/Output ---

# Type a simple expression and execute it
agent-terminal type "2 + 2" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"

# Wait for the result to appear
agent-terminal wait --text "4" --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Verify the result is on screen
agent-terminal assert --text "4" --session "$SESSION"

# Wait for the next prompt (REPL is ready for more input)
agent-terminal wait --text "$PROMPT" --session "$SESSION"

# --- Multi-line Input ---

# Some REPLs support multi-line input
agent-terminal type "def greet(name):" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# The REPL should show a continuation prompt (e.g., "...")
agent-terminal type "    return f'Hello, {name}'" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# Empty line to finish the function definition
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --text "$PROMPT" --session "$SESSION"

# Call the function
agent-terminal type "greet('World')" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --text "Hello, World" --session "$SESSION"

agent-terminal assert --text "Hello, World" --session "$SESSION"

# --- Readline Features ---

# Test command history (Up arrow)
agent-terminal wait --text "$PROMPT" --session "$SESSION"
agent-terminal send "Up" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Clear the line
agent-terminal send "C-c" --session "$SESSION"
agent-terminal wait --text "$PROMPT" --session "$SESSION"

# Test tab completion
agent-terminal type "pri" --session "$SESSION"
agent-terminal send "Tab" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Clear for next test
agent-terminal send "C-c" --session "$SESSION"
agent-terminal wait --text "$PROMPT" --session "$SESSION"

# --- Error Handling ---

# Trigger an error and verify the REPL handles it gracefully
agent-terminal type "1 / 0" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --text "ZeroDivision" --session "$SESSION"

# REPL should still be alive and show a prompt
agent-terminal wait --text "$PROMPT" --session "$SESSION"
agent-terminal status --session "$SESSION" --json

# --- Scrollback ---

# After many inputs, earlier output scrolls off screen.
# Use scrollback to find it.
agent-terminal scrollback --search "Hello, World" --session "$SESSION"

# Or get the last N lines
agent-terminal scrollback --lines 50 --session "$SESSION"

# --- Import and Module Usage ---

agent-terminal type "import sys" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --text "$PROMPT" --session "$SESSION"

agent-terminal type "sys.version" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# --- Clean Exit ---

# Exit the REPL
agent-terminal wait --text "$PROMPT" --session "$SESSION"
agent-terminal type "exit()" --session "$SESSION"
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --stable 1000 --session "$SESSION"

EXIT_CODE=$(agent-terminal exit-code --session "$SESSION" 2>/dev/null || echo "unknown")
echo "REPL exit code: $EXIT_CODE"

# --- Cleanup ---

agent-terminal close --session "$SESSION"
trap - EXIT

echo "=== REPL test complete ==="
