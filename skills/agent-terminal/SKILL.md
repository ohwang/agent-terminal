---
name: agent-terminal
description: TUI testing CLI for AI agents. Use when you need to launch, observe, interact with, or test any terminal application — full-screen TUIs (ratatui, bubbletea, textual, ink), REPLs (python, node), curses apps (htop, vim), or CLI tools with interactive prompts.
allowed-tools: Bash(agent-terminal:*), Bash(agent-terminal *)
---

# agent-terminal

TUI testing tool for autonomous agent-driven terminal application testing.

**Use when**: You need to launch, observe, interact with, or test any terminal application -- full-screen TUIs (ratatui, bubbletea, textual, ink), REPLs (python, node), curses apps (htop, vim), or CLI tools with interactive prompts. Works by running the app inside a tmux session and giving you structured observation and interaction primitives.

**Requires**: tmux >= 3.0. Run `agent-terminal doctor` to verify.

---

## The Core Loop

Every interaction follows this pattern. Never deviate.

```
snapshot -> reason -> act -> wait -> snapshot
```

**Never fire-and-forget.** Every action (send, type, click, resize) must be followed by a wait and then a snapshot to confirm the result. If you skip the confirmation snapshot, you will drift out of sync with reality.

```bash
# CORRECT
agent-terminal send Enter --session s1
agent-terminal wait --stable 300 --session s1
agent-terminal snapshot --session s1

# WRONG -- no confirmation
agent-terminal send Enter --session s1
agent-terminal send "j" --session s1       # you don't know if Enter worked
```

---

## Quick Start

Minimal example testing a TUI app end-to-end:

```bash
# 1. Launch the app
agent-terminal open "./my-app" --session test

# 2. Wait for first render to stabilize
agent-terminal wait --stable 500 --session test

# 3. See what's on screen
agent-terminal snapshot --session test
# Output:
# [size: 80x24  cursor: 1,0  session: test]
# -----------------------------------------
#   1| Welcome to my-app
#   2| > Option A
#   3|   Option B
#   4|   Option C
#   5|
#   6| [q]uit  [Enter] select

# 4. Interact
agent-terminal send "j" --session test

# 5. Wait for the app to respond
agent-terminal wait --stable 200 --session test

# 6. Confirm the result
agent-terminal snapshot --session test
# Output shows cursor moved to Option B

# 7. Clean up
agent-terminal close --session test
```

---

## Command Reference

All commands default to `--session agent-terminal` if not specified.

### Lifecycle

| Command | Description |
|---------|-------------|
| `open "<cmd>" [--session s] [--pane p] [--env K=V]... [--size COLSxROWS]` | Launch command in a new tmux session |
| `close [--session s]` | Kill the session and clean up temp files |
| `list` | List all active tmux sessions |

### Observation

| Command | Description |
|---------|-------------|
| `snapshot [--session s] [--pane p] [--color] [--raw] [--ansi] [--json] [--diff] [--scrollback N]` | Capture current terminal content |
| `screenshot [--session s] [--path f] [--annotate] [--html] [--theme dark\|light]` | Render terminal as PNG or HTML image |
| `scrollback [--session s] [--lines N] [--search "text"]` | Read the tmux scrollback buffer |
| `find "pattern" [--session s] [--all] [--regex] [--color "style"]` | Search screen for text, return row,col |

### Interaction

| Command | Description |
|---------|-------------|
| `send <keys>... [--session s] [--pane p]` | Send named key sequences (e.g., `Enter`, `C-c`, `j`) |
| `type "text" [--session s] [--pane p]` | Type literal text (no key-name interpretation) |
| `paste "text" [--session s] [--pane p]` | Paste via tmux buffer (safe for special chars) |
| `resize <cols> <rows> [--session s] [--pane p]` | Resize the terminal |
| `click <row> <col> [--session s] [--right] [--double]` | Mouse click at position (1-indexed) |
| `drag <r1> <c1> <r2> <c2> [--session s]` | Mouse drag between positions |
| `scroll-wheel <up\|down> <row> <col> [--session s]` | Scroll wheel event at position |

### Waiting

| Command | Description |
|---------|-------------|
| `wait <ms>` | Hard wait (last resort) |
| `wait --text "str" [--timeout ms] [--interval ms]` | Poll until text appears on screen |
| `wait --text-gone "str"` | Poll until text disappears |
| `wait --stable <ms>` | Poll until screen unchanged for N ms |
| `wait --cursor <row,col>` | Poll until cursor reaches position |
| `wait --regex "pattern"` | Poll until regex matches screen content |

Default timeout: 10000ms. Default poll interval: 50ms.

### Assertion

| Command | Description |
|---------|-------------|
| `assert --text "str" [--session s]` | Exit 0 if text present, exit 1 with snapshot if not |
| `assert --no-text "str"` | Exit 0 if text absent |
| `assert --row N --row-text "str"` | Check specific row contains text |
| `assert --cursor-row N` | Check cursor is on expected row |
| `assert --color N --color-style "fg:red,bold"` | Check row N has the given style |
| `assert --style "text" --style-check "fg:red"` | Check that specific text has a style |

### Process Health

| Command | Description |
|---------|-------------|
| `status [--session s] [--pane p] [--json]` | Is the process alive, dead, or crashed? |
| `exit-code [--session s]` | Get the exit code of the terminated process |
| `logs [--session s] [--stderr]` | Read captured stderr/stdout |
| `signal <SIG> [--session s]` | Send a Unix signal (SIGINT, SIGTERM, SIGWINCH, etc.) |

### Clipboard

| Command | Description |
|---------|-------------|
| `clipboard read [--session s]` | Read the tmux paste buffer |
| `clipboard write "text" [--session s]` | Write to the tmux paste buffer |
| `clipboard paste [--session s]` | Paste from buffer into the pane |

### Performance

| Command | Description |
|---------|-------------|
| `perf start [--session s]` | Begin background frame recording |
| `perf stop [--session s] [--json]` | Stop recording and return FPS metrics |
| `perf fps --during "cmds" [--session s]` | Measure FPS during a command string |
| `perf fps --during-batch [--session s]` | Measure FPS during JSON batch from stdin |
| `perf fps --duration <ms> [--session s]` | Passively observe FPS for N ms |
| `perf latency [--session s] [--key k] [--samples N] [--json]` | Measure keystroke-to-render latency |

### Environment & Testing

| Command | Description |
|---------|-------------|
| `doctor` | Validate tmux version and capabilities |
| `init` | Detect framework and generate starter test |
| `test-matrix --command "cmd" --test "cmds" [--sizes] [--terms] [--colors]` | Run tests across terminal configurations |
| `a11y-check "command"` | Accessibility audit (NO_COLOR, TERM=dumb, resize) |

---

## Failure Recovery

When things go wrong, follow this flowchart:

### "Snapshot shows nothing / stale content"

```bash
# 1. Check if the process is alive
agent-terminal status --session s1 --json

# 2a. If alive=false: read the crash output
agent-terminal logs --stderr --session s1
agent-terminal exit-code --session s1
# -> Fix the bug, then re-open

# 2b. If alive=true: the app may be waiting for input or still loading
agent-terminal wait --stable 1000 --session s1 --timeout 15000
agent-terminal snapshot --session s1
```

### "wait timed out"

The error message includes the last snapshot and session diagnostics. Read them.

```bash
# The wait error shows what was on screen. Common causes:
# 1. Text never appeared -> wrong expected text, or app is in wrong state
# 2. Screen kept changing -> animation/spinner, use a longer --stable window
# 3. App crashed mid-wait -> check status

agent-terminal status --session s1 --json
# If dead: agent-terminal logs --stderr --session s1
# If alive: agent-terminal snapshot --session s1  # see what's actually there
```

### "Session already exists"

```bash
# Close the old one first
agent-terminal close --session s1
agent-terminal open "./my-app" --session s1
```

### "App seems to ignore my keystrokes"

```bash
# 1. Make sure the app is focused (not a shell prompt)
agent-terminal snapshot --session s1

# 2. Use 'type' for literal text, 'send' for key names
agent-terminal type "hello" --session s1    # types h-e-l-l-o literally
agent-terminal send "Enter" --session s1    # sends the Enter key

# 3. For special characters in text, use paste
agent-terminal paste "line1\nline2" --session s1

# 4. Check if the app needs mouse mode
agent-terminal click 3 10 --session s1
```

### "Colors/styles not what I expected"

```bash
# Use --color for annotated view
agent-terminal snapshot --color --session s1
# Shows: [fg:red bold] annotations per line

# Use --json for precise spans
agent-terminal snapshot --json --session s1

# Assert specific styles
agent-terminal assert --style "Error" --style-check "fg:red" --session s1
```

---

## Framework-Specific Tips

### Ratatui (Rust)

```bash
# Ratatui apps typically use crossterm and render immediately
agent-terminal open "cargo run --release" --session rui
agent-terminal wait --stable 500 --session rui

# Ratatui uses alternate screen -- snapshot captures it correctly
# Key bindings are usually single characters
agent-terminal send "q" --session rui     # quit
agent-terminal send "j" --session rui     # down (vim-style)

# Test resize -- ratatui apps should handle SIGWINCH
agent-terminal resize 40 10 --session rui
agent-terminal wait --stable 300 --session rui
agent-terminal snapshot --session rui     # verify layout adapted
```

### Bubbletea (Go)

```bash
# Bubbletea apps need time for the initial tea.Program render
agent-terminal open "go run ." --session bt
agent-terminal wait --stable 1000 --session bt   # generous first wait

# Bubbletea uses the Elm architecture -- state updates are async
# Always wait after sending keys
agent-terminal send "j" --session bt
agent-terminal wait --stable 200 --session bt

# Mouse support: most bubbletea apps using lipgloss support mouse
agent-terminal click 5 10 --session bt
agent-terminal wait --stable 200 --session bt

# Test NO_COLOR compliance (bubbletea/lipgloss respects this)
agent-terminal close --session bt
agent-terminal open "go run ." --session bt --env NO_COLOR=1
```

### Textual (Python)

```bash
# Textual has a CSS-based styling warmup period
agent-terminal open "python -m my_app" --session tx
agent-terminal wait --stable 1500 --session tx   # CSS rendering needs time

# Textual apps heavily use mouse -- test click targets
agent-terminal click 3 20 --session tx
agent-terminal wait --stable 300 --session tx

# Textual supports TEXTUAL_PRESS for testing specific widgets
# Use type for search/input widgets
agent-terminal type "search term" --session tx
agent-terminal send "Enter" --session tx

# Test with NO_COLOR (textual should degrade gracefully)
agent-terminal close --session tx
agent-terminal open "python -m my_app" --session tx --env NO_COLOR=1
agent-terminal wait --stable 1500 --session tx
agent-terminal snapshot --color --session tx   # verify no color codes
```

### Ink (React for CLI)

```bash
# Ink apps are Node.js -- may need build step
agent-terminal open "npx tsx src/index.tsx" --session ink
agent-terminal wait --stable 1000 --session ink

# Ink re-renders on state change -- wait for stability
agent-terminal send "j" --session ink
agent-terminal wait --stable 300 --session ink

# Ink apps often use Tab for navigation
agent-terminal send "Tab" --session ink
agent-terminal wait --stable 200 --session ink

# Test with CI=true (common env var that affects ink rendering)
agent-terminal close --session ink
agent-terminal open "npx tsx src/index.tsx" --session ink --env CI=true
```

---

## Scrolling CLI vs Full-Screen TUI

These require different strategies.

### Full-Screen TUI (ratatui, bubbletea, textual, htop, vim)

Full-screen apps use the alternate screen buffer. The visible viewport IS the entire state.

```bash
# snapshot captures everything -- no scrollback needed
agent-terminal snapshot --session s1

# Navigation is via app keys (j/k, arrows, PgUp/PgDn)
agent-terminal send "G" --session s1          # vim-style go to bottom
agent-terminal wait --stable 200 --session s1
agent-terminal snapshot --session s1          # now shows bottom of list

# Scrollback buffer is NOT useful for full-screen apps
# (it contains pre-TUI shell output, not app content)
```

### Scrolling CLI (build output, REPL, logs)

Scrolling apps write to stdout and content scrolls off the top.

```bash
# snapshot only shows the visible viewport (last N rows)
agent-terminal snapshot --session s1

# Use scrollback to see history
agent-terminal scrollback --lines 500 --session s1

# Search scrollback for specific output
agent-terminal scrollback --search "error" --session s1

# Include scrollback in a snapshot
agent-terminal snapshot --scrollback 100 --session s1
```

---

## Performance Testing Pattern

Use start/stop mode across multiple tool calls (recommended for Claude Code):

```bash
# 1. Open and stabilize
agent-terminal open "./my-app" --session perf
agent-terminal wait --stable 500 --session perf

# 2. Start recording
agent-terminal perf start --session perf

# 3. Perform interactions (each is a separate tool call)
agent-terminal send "j" --session perf
agent-terminal send "j" --session perf
agent-terminal send "j" --session perf
agent-terminal send "G" --session perf

# 4. Stop and get metrics
agent-terminal perf stop --json --session perf
# Returns: { "fps": 24.5, "frame_count": 12, "p95_frame_ms": 88, ... }

# 5. Measure input latency separately
agent-terminal perf latency --key "j" --samples 5 --json --session perf
# Returns: { "mean_ms": 18, "p95_ms": 38, ... }

# 6. Interpret results
#    FPS: 0 = frozen, 1-5 = sluggish, 10-30 = normal
#    Latency: <16ms = excellent, 16-50ms = good, 50-100ms = noticeable, >100ms = sluggish

agent-terminal close --session perf
```

For quick one-shot measurement:

```bash
agent-terminal perf fps --duration 3000 --session perf    # passive observation
agent-terminal perf fps --during 'send "j" && send "k"' --session perf  # during actions
```

---

## Matrix Testing Pattern

Test your app across terminal sizes, TERM values, and color modes in one command:

```bash
agent-terminal test-matrix \
  --command "./my-app" \
  --sizes "80x24,120x40,40x10" \
  --terms "xterm-256color,dumb" \
  --colors "default,NO_COLOR=1" \
  --test "agent-terminal assert --text 'Welcome' --session {session}; agent-terminal status --session {session} --json"
```

This runs 12 combinations (3 sizes x 2 terms x 2 colors) and reports:

```
COMBINATION                              RESULT
------------------------------------------------------------
80x24+xterm-256color+default             pass
80x24+xterm-256color+NO_COLOR=1          pass
80x24+dumb+default                       FAIL: Process crashed during startup
40x10+xterm-256color+default             FAIL: text "Welcome" not found
...

10/12 passed, 2 failed
Failure snapshots saved to: ./agent-terminal-matrix/
```

Use `{session}` in test commands -- it gets replaced with the per-combination session name.

**When to use test-matrix**: after you have a working app and want to verify it handles edge cases. Not for initial development.

---

## Common Mistakes

**1. Forgetting to wait after actions**
```bash
# WRONG
agent-terminal send "j" --session s1
agent-terminal snapshot --session s1    # may capture pre-action state

# RIGHT
agent-terminal send "j" --session s1
agent-terminal wait --stable 200 --session s1
agent-terminal snapshot --session s1
```

**2. Using `send` when you mean `type`**
```bash
# send interprets key names: "Enter" sends the Enter key
agent-terminal send "Enter" --session s1

# type sends literal characters: types E-n-t-e-r
agent-terminal type "Enter" --session s1

# For typing text into input fields, use type
agent-terminal type "hello world" --session s1
agent-terminal send "Enter" --session s1
```

**3. Not checking process health when things seem stuck**
```bash
# If snapshot shows stale content, ALWAYS check status first
agent-terminal status --session s1 --json
# { "alive": false, "exit_code": 1 }
agent-terminal logs --stderr --session s1
# panic at src/main.rs:42: index out of bounds
```

**4. Hardcoding waits instead of using conditions**
```bash
# WRONG -- fragile, wastes time
agent-terminal wait 3000 --session s1

# RIGHT -- fast and reliable
agent-terminal wait --text "Ready" --session s1
agent-terminal wait --stable 500 --session s1
```

**5. Not using unique session names for parallel tests**
```bash
# WRONG -- tests collide
agent-terminal open "./app" --session test
# ...in another test...
agent-terminal open "./app" --session test   # ERROR: already exists

# RIGHT -- unique names
agent-terminal open "./app" --session "test-$$"    # PID-scoped in bash
agent-terminal open "./app" --session "test-resize"
agent-terminal open "./app" --session "test-input"
```

**6. Not cleaning up sessions**
```bash
# Always use trap in bash scripts
cleanup() { agent-terminal close --session "$SESSION" 2>/dev/null || true; }
trap cleanup EXIT
```

**7. Sending Ctrl+C with `send` vs `signal`**
```bash
# send "C-c" sends the keystroke -- app's input handler processes it
agent-terminal send "C-c" --session s1

# signal SIGINT sends the actual signal -- bypasses app input
agent-terminal signal SIGINT --session s1

# Usually you want send "C-c" for graceful app shutdown
# Use signal SIGINT for testing signal handlers specifically
```
