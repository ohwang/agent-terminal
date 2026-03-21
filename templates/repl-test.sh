#!/usr/bin/env bash
# Template for testing REPL applications (Python, Node, etc.).
# Covers: launch, send commands, verify output, exit cleanly.
# Adjust the REPL command and expected prompts/output for your use case.
set -euo pipefail

SESSION="test-repl-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

# --- Launch the REPL ---

# Python example (change to "node", "irb", "ghci", etc.)
agent-terminal open "python3" --session "$SESSION"

# Wait for the REPL prompt to appear
agent-terminal wait --text ">>>" --session "$SESSION" --timeout 10000
agent-terminal snapshot --session "$SESSION"

# --- Test basic expressions ---

# Type a simple expression
agent-terminal type "2 + 2" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"

# Verify the result
agent-terminal assert --text "4" --session "$SESSION"

# --- Test importing a module ---

agent-terminal type "import os" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text ">>>" --session "$SESSION"

agent-terminal type "os.path.exists('/')" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"

agent-terminal assert --text "True" --session "$SESSION"

# --- Test multi-line input ---

agent-terminal type "def greet(name):" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text "..." --session "$SESSION"

agent-terminal type "    return f'Hello, {name}!'" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text "..." --session "$SESSION"

# Empty line to end the function definition
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text ">>>" --session "$SESSION"

# Call the function
agent-terminal type "greet('World')" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"

agent-terminal assert --text "Hello, World!" --session "$SESSION"

# --- Test error handling ---

agent-terminal type "1 / 0" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"

agent-terminal assert --text "ZeroDivisionError" --session "$SESSION"

# Verify the REPL recovered and shows a new prompt
agent-terminal wait --text ">>>" --session "$SESSION"

# --- Exit the REPL cleanly ---

agent-terminal type "exit()" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 1000 --session "$SESSION"

# --- Cleanup ---

agent-terminal close --session "$SESSION"
trap - EXIT

echo "Test passed"
