#!/usr/bin/env bash
set -euo pipefail

# Template for testing ncurses/curses-style full-screen TUI apps
# Usage: bash curses-app.sh
#
# Curses apps use the alternate screen buffer, handle SIGWINCH for
# resize, and often have complex keyboard navigation. This template
# covers the common testing patterns.
#
# Replace "./your-curses-app" with your app's run command.
# Adjust key bindings, expected text, and sizes for your app.

SESSION="test-curses-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== Curses App Test ==="

# --- Setup ---

# Curses apps need a proper TERM value
agent-terminal open "./your-curses-app" --session "$SESSION" \
    --size 80x24 \
    --env TERM=xterm-256color

# Curses apps typically render immediately but may need a moment
# for initialization (especially if loading config files)
agent-terminal wait --stable 1000 --session "$SESSION"

# --- Verify Initial Render ---

agent-terminal snapshot --session "$SESSION"

# Check that the app drew its UI (curses apps should fill the screen)
# Replace with text specific to your app
agent-terminal assert --text "File" --session "$SESSION"

# Verify process is alive
agent-terminal status --session "$SESSION" --json

# --- Keyboard Navigation ---

# Curses apps often use function keys, arrow keys, and ctrl sequences
# Test basic navigation
agent-terminal send "Down" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

agent-terminal send "Up" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# Test menu access (common in curses apps: F1, Alt+F, etc.)
agent-terminal send "F1" --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Close the menu/dialog
agent-terminal send "Escape" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# --- Resize Handling ---

# Curses apps receive SIGWINCH and must redraw. This is a common
# source of crashes (especially in older ncurses apps).

# Small terminal
agent-terminal resize 40 10 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal status --session "$SESSION" --json
agent-terminal snapshot --session "$SESSION"

# Very small (edge case -- may trigger "terminal too small" message)
agent-terminal resize 20 5 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal status --session "$SESSION" --json

# Back to standard
agent-terminal resize 80 24 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Verify app recovered
agent-terminal snapshot --session "$SESSION"
agent-terminal status --session "$SESSION" --json

# Wide terminal
agent-terminal resize 200 50 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Restore
agent-terminal resize 80 24 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# --- Color Verification ---

# Curses apps use color pairs. Verify key colors are correct.
agent-terminal snapshot --color --session "$SESSION"

# --- TERM=dumb Fallback ---

# Some curses apps should degrade gracefully with TERM=dumb.
# Others may legitimately refuse to start. Test both cases.
agent-terminal close --session "$SESSION"

echo "Testing TERM=dumb fallback..."
agent-terminal open "./your-curses-app" --session "$SESSION" \
    --size 80x24 \
    --env TERM=dumb
sleep 2

STATUS=$(agent-terminal status --session "$SESSION" --json)
echo "TERM=dumb status: $STATUS"

# If the app crashed, that may be acceptable for a curses app
# (unlike a TUI framework app, where graceful degradation is expected)
agent-terminal snapshot --session "$SESSION" 2>/dev/null || true

# --- Signal Handling ---

# Restart with normal TERM for signal tests
agent-terminal close --session "$SESSION" 2>/dev/null || true
agent-terminal open "./your-curses-app" --session "$SESSION" \
    --size 80x24 \
    --env TERM=xterm-256color
agent-terminal wait --stable 500 --session "$SESSION"

# Test Ctrl+C
agent-terminal send "C-c" --session "$SESSION"
agent-terminal wait --stable 1000 --session "$SESSION"
EXIT_CODE=$(agent-terminal exit-code --session "$SESSION" 2>/dev/null || echo "still running")
echo "Exit code after Ctrl+C: $EXIT_CODE"

# --- Cleanup ---

agent-terminal close --session "$SESSION" 2>/dev/null || true
trap - EXIT

echo "=== Curses app test complete ==="
