# Common Testing Patterns

Reusable patterns for testing terminal applications with agent-terminal. Each pattern includes a complete, working example.

---

## 1. Basic Lifecycle Test

The simplest test: launch, verify initial render, close.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

# Launch
agent-terminal open "./my-app" --session "$SESSION"

# Wait for first render
agent-terminal wait --stable 500 --session "$SESSION"

# Verify the app started correctly
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --text "Welcome" --session "$SESSION"

# Clean up
agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: basic lifecycle"
```

---

## 2. Keyboard Navigation

Test arrow keys, vim keys, Tab, and selection in a list or menu.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Verify initial selection is on first item
agent-terminal assert --text "> Item 1" --session "$SESSION"

# Navigate down
agent-terminal send Down --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --text "> Item 2" --session "$SESSION"

# Navigate down again
agent-terminal send Down --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --text "> Item 3" --session "$SESSION"

# Navigate back up
agent-terminal send Up --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --text "> Item 2" --session "$SESSION"

# Select with Enter
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --text "Selected: Item 2" --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: keyboard navigation"
```

---

## 3. Form Input

Test text input fields, tab between fields, and form submission.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./form-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Type into the first field (name)
agent-terminal type "John Doe" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --text "John Doe" --session "$SESSION"

# Tab to the next field (email)
agent-terminal send Tab --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal type "john@example.com" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# Tab to submit button and press Enter
agent-terminal send Tab --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text "Submitted" --session "$SESSION" --timeout 5000

agent-terminal assert --text "Submitted" --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: form input"
```

---

## 4. Scrollable List

Test scrolling through a long list, verifying items appear and disappear.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./list-app" --session "$SESSION" --size 80x10
agent-terminal wait --stable 500 --session "$SESSION"

# Verify first page
agent-terminal assert --text "Item 1" --session "$SESSION"
agent-terminal assert --no-text "Item 50" --session "$SESSION"

# Scroll to bottom (vim-style)
agent-terminal send G --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"

# Verify we see the last items
agent-terminal assert --text "Item 100" --session "$SESSION"
agent-terminal assert --no-text "Item 1" --session "$SESSION"

# Scroll back to top
agent-terminal send g g --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --text "Item 1" --session "$SESSION"

# Page-down
agent-terminal send PgDn --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: scrollable list"
```

---

## 5. Resize Handling

Verify the app adapts its layout when the terminal is resized.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION" --size 80x24
agent-terminal wait --stable 500 --session "$SESSION"

# Verify normal layout
agent-terminal assert --text "Sidebar" --session "$SESSION"
agent-terminal assert --text "Main Content" --session "$SESSION"

# Shrink to narrow width -- sidebar should collapse
agent-terminal resize 40 24 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --no-text "Sidebar" --session "$SESSION"
agent-terminal assert --text "Main Content" --session "$SESSION"

# Shrink height -- footer should hide
agent-terminal resize 40 10 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal assert --no-text "Footer" --session "$SESSION"

# Restore original size -- everything should come back
agent-terminal resize 80 24 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal assert --text "Sidebar" --session "$SESSION"
agent-terminal assert --text "Footer" --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: resize handling"
```

---

## 6. Color and Theme Verification

Verify colors, styles, and theme switching.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Check that the title is bold
agent-terminal assert --style "My App" --style-check "bold" --session "$SESSION"

# Check error messages are red
agent-terminal send "t" --session "$SESSION"  # trigger an error
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --style "Error" --style-check "fg:red" --session "$SESSION"

# Test NO_COLOR mode
agent-terminal close --session "$SESSION"
agent-terminal open "./my-app" --session "$SESSION" --env NO_COLOR=1
agent-terminal wait --stable 500 --session "$SESSION"

# Snapshot with color annotations -- should have no fg/bg annotations
agent-terminal snapshot --color --session "$SESSION"

# Verify the app still works without color
agent-terminal assert --text "My App" --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: color/theme verification"
```

---

## 7. Signal Handling

Test graceful shutdown, interrupt recovery, and signal responses.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./server-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal assert --text "Listening" --session "$SESSION"

# Test Ctrl+C handling (graceful shutdown)
agent-terminal send C-c --session "$SESSION"
agent-terminal wait --text "Shutting down" --session "$SESSION" --timeout 5000
agent-terminal assert --text "Shutting down" --session "$SESSION"

# Wait for process to exit
agent-terminal wait --stable 1000 --session "$SESSION"
agent-terminal status --json --session "$SESSION"
EXIT_CODE=$(agent-terminal exit-code --session "$SESSION")

if [ "$EXIT_CODE" -ne 0 ]; then
    echo "FAIL: expected exit code 0, got $EXIT_CODE"
    agent-terminal logs --stderr --session "$SESSION"
    exit 1
fi

# Test SIGTERM handling
agent-terminal close --session "$SESSION"
agent-terminal open "./server-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

agent-terminal signal SIGTERM --session "$SESSION"
agent-terminal wait --stable 2000 --session "$SESSION"

# Check logs for clean shutdown message
agent-terminal logs --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: signal handling"
```

---

## 8. Performance Regression

Measure FPS and latency, compare against thresholds.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

MIN_FPS=10
MAX_LATENCY_MS=100

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Measure FPS during typical interaction
agent-terminal perf start --session "$SESSION"
for i in $(seq 1 20); do
    agent-terminal send "j" --session "$SESSION"
done
agent-terminal wait --stable 300 --session "$SESSION"
PERF_JSON=$(agent-terminal perf stop --json --session "$SESSION")

FPS=$(echo "$PERF_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['fps'])")
echo "FPS: $FPS"

if (( $(echo "$FPS < $MIN_FPS" | bc -l) )); then
    echo "FAIL: FPS $FPS below threshold $MIN_FPS"
    exit 1
fi

# Measure input latency
LATENCY_JSON=$(agent-terminal perf latency --key "j" --samples 10 --json --session "$SESSION")
P95=$(echo "$LATENCY_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['p95_ms'])")
echo "P95 latency: ${P95}ms"

if (( $(echo "$P95 > $MAX_LATENCY_MS" | bc -l) )); then
    echo "FAIL: P95 latency ${P95}ms exceeds threshold ${MAX_LATENCY_MS}ms"
    exit 1
fi

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: performance regression"
```

---

## 9. Cross-Terminal Compatibility

Use test-matrix to validate across terminal configurations.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Run the app across multiple sizes, TERM values, and color modes
agent-terminal test-matrix \
    --command "./my-app" \
    --sizes "80x24,120x40,40x10" \
    --terms "xterm-256color,screen-256color,dumb" \
    --colors "default,NO_COLOR=1" \
    --test "
        agent-terminal wait --stable 500 --session {session};
        agent-terminal assert --text 'Welcome' --session {session};
        agent-terminal send 'q' --session {session};
        agent-terminal wait --stable 500 --session {session};
        agent-terminal status --json --session {session}
    "

echo "PASS: cross-terminal compatibility"
```

---

## 10. CI Integration

A complete test script suitable for CI environments.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Verify environment
agent-terminal doctor

FAILURES=0

run_test() {
    local name="$1"
    local session="ci-test-$name-$$"
    shift

    echo "--- $name ---"
    if "$@" "$session"; then
        echo "PASS: $name"
    else
        echo "FAIL: $name"
        FAILURES=$((FAILURES + 1))
        # Capture debug info
        agent-terminal snapshot --session "$session" 2>/dev/null || true
        agent-terminal logs --stderr --session "$session" 2>/dev/null || true
    fi
    agent-terminal close --session "$session" 2>/dev/null || true
}

test_startup() {
    local session="$1"
    agent-terminal open "./my-app" --session "$session"
    agent-terminal wait --stable 500 --session "$session"
    agent-terminal assert --text "Welcome" --session "$session"
}

test_navigation() {
    local session="$1"
    agent-terminal open "./my-app" --session "$session"
    agent-terminal wait --stable 500 --session "$session"
    agent-terminal send Down --session "$session"
    agent-terminal wait --stable 200 --session "$session"
    agent-terminal assert --text "> Item 2" --session "$session"
}

test_quit() {
    local session="$1"
    agent-terminal open "./my-app" --session "$session"
    agent-terminal wait --stable 500 --session "$session"
    agent-terminal send "q" --session "$session"
    agent-terminal wait --stable 1000 --session "$session"
    local code
    code=$(agent-terminal exit-code --session "$session")
    [ "$code" -eq 0 ]
}

run_test "startup" test_startup
run_test "navigation" test_navigation
run_test "quit" test_quit

echo ""
echo "=== Results: $((3 - FAILURES))/3 passed ==="

if [ "$FAILURES" -gt 0 ]; then
    exit 1
fi
```

---

## 11. REPL Testing

Test interactive REPLs (Python, Node, etc.) by typing commands and checking output.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

# Start Python REPL
agent-terminal open "python3" --session "$SESSION"
agent-terminal wait --text ">>>" --session "$SESSION"

# Run a computation
agent-terminal type "2 + 2" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --text "4" --session "$SESSION"

# Import and use a module
agent-terminal type "import math" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --text ">>>" --session "$SESSION"

agent-terminal type "math.pi" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --text "3.14159" --session "$SESSION"

# Exit cleanly
agent-terminal type "exit()" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 1000 --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: REPL testing"
```

---

## 12. Multi-Pane (Client/Server)

Test a client/server setup where both run in the same session on separate panes.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

# Start server in its own pane
agent-terminal open "./server --port 9090" --session "$SESSION" --pane server
agent-terminal wait --text "Listening on :9090" --session "$SESSION" --pane server --timeout 10000

# Verify server is running
agent-terminal assert --text "Listening" --session "$SESSION" --pane server
agent-terminal status --json --session "$SESSION" --pane server

# Start client in another pane
agent-terminal open "./client --connect localhost:9090" --session "$SESSION" --pane client
agent-terminal wait --stable 500 --session "$SESSION" --pane client

# Type a message in the client
agent-terminal type "Hello, server!" --session "$SESSION" --pane client
agent-terminal send Enter --session "$SESSION" --pane client
agent-terminal wait --stable 500 --session "$SESSION" --pane client

# Verify client received response
agent-terminal assert --text "Response:" --session "$SESSION" --pane client

# Verify server logged the request
agent-terminal snapshot --session "$SESSION" --pane server
agent-terminal assert --text "Hello, server!" --session "$SESSION" --pane server

# Graceful shutdown: stop client first, then server
agent-terminal send C-c --session "$SESSION" --pane client
agent-terminal wait --stable 500 --session "$SESSION" --pane client

agent-terminal send C-c --session "$SESSION" --pane server
agent-terminal wait --stable 500 --session "$SESSION" --pane server

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: multi-pane client/server"
```

---

## 13. Testing Pager Apps (less, man)

Pagers exit and kill the tmux session when they quit. Use a shell wrapper to keep the session alive.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

# Open bash first, then run the pager inside it
agent-terminal open "bash" --session "$SESSION" --no-stderr --env PS1='$ '
agent-terminal wait --text "$ " --session "$SESSION"

# Launch the pager from inside bash
agent-terminal type "less /etc/hosts" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# Quit the pager -- session stays alive because bash is still running
agent-terminal send "q" --session "$SESSION"
agent-terminal wait --text "$ " --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: pager testing"
```

---

## 14. Testing Bash / Readline Apps

Use `--no-stderr` so that PS1 prompts and tab completion output (which go through stderr) remain visible in snapshots.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

# --no-stderr keeps prompts visible; --env PS1 makes them predictable
agent-terminal open "bash" --session "$SESSION" --no-stderr --env PS1='$ '
agent-terminal wait --text "$ " --session "$SESSION"

# Run a command and verify output
agent-terminal type "echo hello" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --text "hello" --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: bash/readline testing"
```

---

## 15. Testing Fast-Exiting Commands

Use `--shell` to keep the session alive after the command exits, so you can inspect its output.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

# --shell keeps the session alive after grep exits
agent-terminal open "grep -r 'TODO' src/" --session "$SESSION" --shell
agent-terminal wait --exit --session "$SESSION" --timeout 10000
agent-terminal snapshot --session "$SESSION"

# Output is still visible even though grep has finished
agent-terminal assert --text "TODO" --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: fast-exiting command"
```

---

## 16. Testing nvim

Use `nvim --clean` to avoid user config interference.

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "nvim --clean" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Enter insert mode and type
agent-terminal send "i" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal type "Hello from nvim" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --text "Hello from nvim" --session "$SESSION"

# Exit without saving
agent-terminal send Escape --session "$SESSION"
agent-terminal type ":q!" --session "$SESSION"
agent-terminal send Enter --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: nvim testing"
```

---

## General Principles

These principles apply across all patterns:

1. **Always wait after acting.** Every `send`, `type`, `click`, or `resize` must be followed by a `wait` and then a `snapshot` or `assert`.

2. **Use traps for cleanup.** Ensure sessions are destroyed even if the test fails partway through.

3. **Use unique session names.** Avoid collisions between parallel tests with `SESSION="test-$$"`.

4. **Prefer condition waits over hard waits.** `wait --text` and `wait --stable` are faster and more reliable than `wait 3000`.

5. **Check process health when things go wrong.** If a snapshot shows unexpected content, run `status --json` and `logs --stderr` before debugging further.

6. **Start with `--stable` after `open`.** The first render may take time, especially for apps that compile or load data.

7. **Use `--env PS1='$ '` for interactive shells.** A simple, predictable prompt makes `wait --text "$ "` reliable. Default PS1 values vary across systems and include escape sequences that complicate matching.

8. **Use `--shell` for commands that exit immediately.** Fast-exiting commands (grep, curl, ls) finish before you can inspect output. `--shell` wraps the command so a shell takes over afterward.

9. **Use `--no-stderr` for bash/readline apps.** Bash sends PS1 prompts and tab completion through stderr. Without `--no-stderr`, prompts are invisible in snapshots.

10. **Use `--exit` instead of `sleep` to wait for process completion.** `wait --exit` polls tmux `#{pane_dead}` and is more reliable than guessing how long a command takes.
