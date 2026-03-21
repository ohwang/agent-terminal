# Common TUI Testing Patterns

Reusable patterns for testing terminal applications with agent-terminal.

---

## Basic Lifecycle Test

The minimal test pattern: open, verify render, clean up.

```bash
#!/usr/bin/env bash
set -euo pipefail

SESSION="test-lifecycle-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

# Open and wait for stable render
agent-terminal open "./my-app" --session "$SESSION" --size 80x24
agent-terminal wait --stable 500 --session "$SESSION"

# Verify the app rendered
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --text "expected content" --session "$SESSION"

# Verify process is alive
agent-terminal status --session "$SESSION" --json

# Clean up
agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: lifecycle test"
```

---

## Testing Keyboard Navigation

Verify arrow keys, vim bindings, or tab navigation move focus correctly.

```bash
SESSION="test-nav-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Verify initial selection
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --row 3 --row-text "> Item 1" --session "$SESSION"

# Move down
agent-terminal send "j" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --row 4 --row-text "> Item 2" --session "$SESSION"

# Move down again
agent-terminal send "j" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --row 5 --row-text "> Item 3" --session "$SESSION"

# Move back up
agent-terminal send "k" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --row 4 --row-text "> Item 2" --session "$SESSION"

# Test wrap-around (if applicable)
agent-terminal send "k" --session "$SESSION"
agent-terminal send "k" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: navigation test"
```

---

## Testing Form Input

Test typing into input fields, validating, and submitting.

```bash
SESSION="test-form-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --text "Name:" --session "$SESSION"

# Type into the name field
agent-terminal type "John Doe" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --text "John Doe" --session "$SESSION"

# Tab to next field
agent-terminal send "Tab" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# Type into email field
agent-terminal type "john@example.com" --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# Submit the form
agent-terminal send "Enter" --session "$SESSION"
agent-terminal wait --text "Success" --session "$SESSION" --timeout 5000

# Verify success message
agent-terminal assert --text "Saved" --session "$SESSION"
agent-terminal assert --no-text "Error" --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: form input test"
```

---

## Testing Scrollable Lists

Verify scrolling behavior with long lists that extend beyond the viewport.

```bash
SESSION="test-scroll-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION" --size 80x10
agent-terminal wait --stable 500 --session "$SESSION"

# Verify initial viewport shows first items
agent-terminal assert --text "Item 1" --session "$SESSION"
agent-terminal assert --no-text "Item 50" --session "$SESSION"

# Scroll to bottom
agent-terminal send "G" --session "$SESSION"    # vim-style go to end
agent-terminal wait --stable 300 --session "$SESSION"

# Now the last items should be visible
agent-terminal snapshot --session "$SESSION"
agent-terminal assert --text "Item 50" --session "$SESSION"
agent-terminal assert --no-text "Item 1" --session "$SESSION"

# Scroll back to top
agent-terminal send "g" "g" --session "$SESSION"  # vim-style go to top
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal assert --text "Item 1" --session "$SESSION"

# Test page-based scrolling
agent-terminal send "PgDn" --session "$SESSION"
agent-terminal wait --stable 300 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: scroll test"
```

---

## Testing Resize Handling

Verify the app adapts to terminal size changes without crashing.

```bash
SESSION="test-resize-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION" --size 80x24
agent-terminal wait --stable 500 --session "$SESSION"

# Capture baseline
agent-terminal snapshot --session "$SESSION"
BASELINE_STATUS=$(agent-terminal status --session "$SESSION" --json)

# Resize small
agent-terminal resize 40 10 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Verify still alive
agent-terminal status --session "$SESSION" --json
agent-terminal snapshot --session "$SESSION"

# Resize very small (edge case)
agent-terminal resize 20 5 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal status --session "$SESSION" --json

# Resize back to normal
agent-terminal resize 80 24 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Verify app recovered
agent-terminal snapshot --session "$SESSION"
agent-terminal status --session "$SESSION" --json

# Resize wide
agent-terminal resize 200 50 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: resize test"
```

---

## Testing Color and Theme

Verify correct use of colors and styles.

```bash
SESSION="test-color-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Check that error messages are red
agent-terminal assert --style "Error" --style-check "fg:red" --session "$SESSION"

# Check that selected items are highlighted
agent-terminal assert --color 3 --color-style "reverse" --session "$SESSION"

# Check that the header is bold
agent-terminal assert --color 1 --color-style "bold" --session "$SESSION"

# Take a color-annotated snapshot for review
agent-terminal snapshot --color --session "$SESSION"

# Test NO_COLOR compliance
agent-terminal close --session "$SESSION"
agent-terminal open "./my-app" --session "$SESSION" --env NO_COLOR=1
agent-terminal wait --stable 500 --session "$SESSION"

# Verify no color codes with NO_COLOR=1
agent-terminal snapshot --raw --session "$SESSION"
# (inspect output -- should have no ANSI color sequences)

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: color test"
```

---

## Testing Signal Handling

Verify graceful shutdown and signal responses.

```bash
SESSION="test-signal-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Test Ctrl+C (graceful exit)
agent-terminal send "C-c" --session "$SESSION"
agent-terminal wait --stable 1000 --session "$SESSION"

# Check if the app shut down cleanly
STATUS=$(agent-terminal status --session "$SESSION" --json)
EXIT_CODE=$(agent-terminal exit-code --session "$SESSION" 2>/dev/null || echo "still running")

# Verify clean exit (exit code 0 or 130 for SIGINT)
echo "Status: $STATUS"
echo "Exit code: $EXIT_CODE"

# Start again for SIGTERM test
agent-terminal close --session "$SESSION" 2>/dev/null || true
agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Send SIGTERM (real signal, not keystroke)
agent-terminal signal SIGTERM --session "$SESSION"
agent-terminal wait --stable 1000 --session "$SESSION"

# Check for graceful shutdown
agent-terminal logs --stderr --session "$SESSION"

# Start again for suspend/resume test
agent-terminal close --session "$SESSION" 2>/dev/null || true
agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Suspend
agent-terminal signal SIGTSTP --session "$SESSION"
sleep 1

# Resume
agent-terminal signal SIGCONT --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Verify app recovered from suspend
agent-terminal snapshot --session "$SESSION"
agent-terminal status --session "$SESSION" --json

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: signal handling test"
```

---

## Performance Regression Testing

Establish a baseline and detect regressions.

```bash
SESSION="test-perf-$$"
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Measure baseline input latency
LATENCY=$(agent-terminal perf latency --key "j" --samples 10 --json --session "$SESSION")
echo "Latency: $LATENCY"

# Measure FPS during interaction
agent-terminal perf start --session "$SESSION"

# Perform representative interactions
for i in $(seq 1 20); do
    agent-terminal send "j" --session "$SESSION"
done

FPS=$(agent-terminal perf stop --json --session "$SESSION")
echo "FPS: $FPS"

# Parse and check thresholds (using jq or python)
MEAN_LATENCY=$(echo "$LATENCY" | python3 -c "import sys,json; print(json.load(sys.stdin)['mean_ms'])")
FPS_VAL=$(echo "$FPS" | python3 -c "import sys,json; print(json.load(sys.stdin)['fps'])")

echo "Mean latency: ${MEAN_LATENCY}ms"
echo "FPS: $FPS_VAL"

# Fail if latency is too high or FPS too low
python3 -c "
import sys
latency = $MEAN_LATENCY
fps = $FPS_VAL
if latency > 100:
    print(f'FAIL: latency {latency}ms > 100ms threshold')
    sys.exit(1)
if fps < 5:
    print(f'FAIL: FPS {fps} < 5 threshold')
    sys.exit(1)
print(f'PASS: latency={latency}ms fps={fps}')
"

agent-terminal close --session "$SESSION"
trap - EXIT
echo "PASS: performance test"
```

---

## Cross-Terminal Compatibility Testing

Use `test-matrix` for automated multi-configuration testing.

```bash
# Quick compatibility check
agent-terminal test-matrix \
    --command "./my-app" \
    --sizes "80x24,40x10,120x40" \
    --terms "xterm-256color,screen-256color,dumb" \
    --colors "default,NO_COLOR=1,COLORTERM=truecolor" \
    --test "agent-terminal wait --stable 500 --session {session}; agent-terminal assert --text 'Welcome' --session {session}"

# Or run the a11y-check for focused accessibility testing
agent-terminal a11y-check "./my-app"
```

### Manual Matrix Testing

For more control, run each configuration explicitly:

```bash
# Test with dumb terminal
SESSION="test-dumb-$$"
agent-terminal open "./my-app" --session "$SESSION" --env TERM=dumb
agent-terminal wait --stable 1000 --session "$SESSION"
agent-terminal status --session "$SESSION" --json
agent-terminal snapshot --session "$SESSION"
agent-terminal close --session "$SESSION"

# Test with NO_COLOR
SESSION="test-nocolor-$$"
agent-terminal open "./my-app" --session "$SESSION" --env NO_COLOR=1
agent-terminal wait --stable 1000 --session "$SESSION"
agent-terminal snapshot --color --session "$SESSION"
# Verify no color annotations appear
agent-terminal close --session "$SESSION"

# Test with small terminal
SESSION="test-small-$$"
agent-terminal open "./my-app" --session "$SESSION" --size 30x8
agent-terminal wait --stable 1000 --session "$SESSION"
agent-terminal status --session "$SESSION" --json
agent-terminal snapshot --session "$SESSION"
agent-terminal close --session "$SESSION"
```

---

## CI Integration Patterns

### GitHub Actions

```yaml
name: TUI Tests
on: [push, pull_request]

jobs:
  tui-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install tmux
        run: sudo apt-get install -y tmux

      - name: Install agent-terminal
        run: cargo install agent-terminal

      - name: Validate environment
        run: agent-terminal doctor

      - name: Build app
        run: cargo build --release

      - name: Run TUI tests
        run: bash tests/tui/basic_test.sh

      - name: Run matrix tests
        run: |
          agent-terminal test-matrix \
            --command "./target/release/my-app" \
            --test "agent-terminal assert --text 'Ready' --session {session}"

      - name: Run accessibility check
        run: agent-terminal a11y-check "./target/release/my-app"

      - name: Upload failure artifacts
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: tui-test-failures
          path: |
            agent-terminal-matrix/
            a11y-report/
```

### Makefile Integration

```makefile
.PHONY: test-tui test-tui-matrix test-tui-a11y

test-tui:
	bash tests/tui/basic_test.sh

test-tui-matrix:
	agent-terminal test-matrix \
		--command "./target/release/my-app" \
		--sizes "80x24,40x10" \
		--terms "xterm-256color,dumb" \
		--test "agent-terminal assert --text 'Ready' --session {session}"

test-tui-a11y:
	agent-terminal a11y-check "./target/release/my-app"

test: test-unit test-tui test-tui-matrix test-tui-a11y
```

### Script Runner Pattern

For test suites with multiple test scripts:

```bash
#!/usr/bin/env bash
# run_tui_tests.sh -- run all TUI test scripts
set -euo pipefail

PASS=0
FAIL=0

for test_script in tests/tui/test_*.sh; do
    echo "--- Running: $test_script ---"
    if bash "$test_script"; then
        echo "PASS: $test_script"
        ((PASS++))
    else
        echo "FAIL: $test_script"
        ((FAIL++))
    fi
    echo
done

echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
```
