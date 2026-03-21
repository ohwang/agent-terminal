#!/usr/bin/env bash
# Template for testing ncurses / full-screen TUI applications.
# Covers: launch, navigation, selection, resize responsiveness, cleanup.
# Replace "./your-curses-app" and assertions to match your app.
set -euo pipefail

SESSION="test-curses-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

# --- Launch and stabilize ---

agent-terminal open "./your-curses-app" --session "$SESSION" --size 80x24
agent-terminal wait --stable 1000 --session "$SESSION"

# Take initial snapshot to see the full-screen layout
agent-terminal snapshot --session "$SESSION"

# Verify the app rendered its main UI elements
agent-terminal assert --text "Menu" --session "$SESSION"

# --- Keyboard navigation ---

# Move down through the menu
agent-terminal send Down --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

agent-terminal send Down --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Select the highlighted item
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"

# Verify the selection took effect
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --text "Selected" --session "$SESSION"

# Navigate back with Escape
agent-terminal send Escape --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --text "Menu" --session "$SESSION"

# --- Resize handling ---

# Shrink the terminal -- app should adapt its layout
agent-terminal resize 40 12 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# The app should still be functional at the smaller size
agent-terminal assert --text "Menu" --session "$SESSION"

# Grow the terminal back
agent-terminal resize 120 40 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Verify layout restored correctly
agent-terminal assert --text "Menu" --session "$SESSION"

# --- Quit the app ---

agent-terminal send "q" --session "$SESSION"
agent-terminal wait --stable 1000 --session "$SESSION"

# Verify the process exited cleanly
EXIT_CODE=$(agent-terminal exit-code --session "$SESSION")
if [ "$EXIT_CODE" -ne 0 ]; then
    echo "FAIL: expected exit code 0, got $EXIT_CODE"
    agent-terminal logs --stderr --session "$SESSION"
    exit 1
fi

# --- Cleanup ---

agent-terminal close --session "$SESSION"
trap - EXIT

echo "Test passed"
