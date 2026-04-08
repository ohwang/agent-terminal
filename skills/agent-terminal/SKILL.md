---
name: agent-terminal
description: TUI testing CLI for AI agents. Use when you need to launch, observe, interact with, or test any terminal application — full-screen TUIs (ratatui, bubbletea, textual, ink), REPLs (python, node), curses apps (htop, vim), or CLI tools with interactive prompts.
allowed-tools: Bash(agent-terminal:*), Bash(agent-terminal *)
---

# agent-terminal

**Use when**: You need to launch, observe, interact with, or test any terminal application. Works by running the app inside a tmux session with structured observation and interaction primitives.

**Requires**: tmux >= 3.0. Run `agent-terminal doctor` to verify.

## The Core Loop

Every interaction follows: **act -> wait -> observe**. Never fire-and-forget.

```bash
# BEST -- single command: act + wait + observe
agent-terminal send Enter --session s1 --wait-stable 300

# Act + wait + screenshot in one command
agent-terminal send Enter --session s1 --wait-stable 300 --screenshot

# Act + wait + save text snapshot to file
agent-terminal send Enter --session s1 --wait-stable 300 --capture /tmp/after.txt

# Type text and submit in one command
agent-terminal type "hello world" --enter --wait-stable 300 --session s1

# WRONG -- no confirmation after action
agent-terminal send Enter --session s1
agent-terminal send "j" --session s1       # you don't know if Enter worked
```

## Quick Start

```bash
agent-terminal open "./my-app" --session test
# Output: session=test size=112x30 command=./my-app

agent-terminal wait --stable 500 --session test
agent-terminal snapshot --session test

agent-terminal send "j" --session test --wait-stable 200
# Output: snapshot showing result

agent-terminal close --session test
```

## Command Reference

All commands default to `--session agent-terminal` if not specified.

### Lifecycle

| Command | Description |
|---------|-------------|
| `open "<cmd>" [--session s] [--pane p] [--env K=V]... [--size COLSxROWS\|vertical\|landscape] [--shell] [--no-stderr] [--replace]` | Launch command in tmux. Default: 112x30. Presets: `vertical` (80x55), `landscape` (112x30). `--replace` kills any existing session with the same name first. |
| `close [--session s]` | Kill the session and clean up temp files |
| `list` | List all active tmux sessions (shows PANES column for multi-pane sessions) |

### Observation

| Command | Description |
|---------|-------------|
| `snapshot [--session s] [--pane p] [--window] [--color] [--raw] [--ansi] [--json] [--diff] [--scrollback N]` | Capture current terminal content. `--window` composites all panes in their layout positions. `--json` includes `panes` array and `pane_id` for multi-pane sessions. |
| `screenshot [--session s] [--window] [--path f] [--annotate] [--html] [--theme dark\|light]` | Render as PNG or HTML. `--window` composites all panes. Default: `<session>-<timestamp>.png` |
| `scrollback [--session s] [--lines N] [--search "text"]` | Read the tmux scrollback buffer |
| `find "pattern" [--session s] [--all] [--regex] [--color "style"] [--json]` | Search screen for text. `--json` returns `{matches: [{row, col, text, style?}]}`. Style included with `--color`. |

### Interaction

| Command | Description |
|---------|-------------|
| `send <keys>... [--session s] [--pane p] [--wait-stable ms] [--capture [path]] [--screenshot [path]]` | Send key sequences (e.g., `Enter`, `C-c`, `j`). `--wait-stable` waits then prints snapshot. `--capture` saves text to file (or stdout). `--screenshot` saves PNG. |
| `type "text" [--session s] [--pane p] [--enter] [--wait-stable ms] [--capture [path]] [--screenshot [path]]` | Type literal text. `--enter` sends Enter after. `--wait-stable` waits then prints snapshot. `--capture`/`--screenshot` save after action. |
| `paste "text" [--session s] [--pane p]` | Paste via tmux buffer (safe for special chars, multi-line) |
| `resize <cols> <rows> [--session s] [--pane p]` | Resize the terminal |
| `click <row> <col> [--session s] [--right] [--double]` | Mouse click at position (1-indexed) |
| `drag <r1> <c1> <r2> <c2> [--session s]` | Mouse drag between positions |
| `scroll-wheel <up\|down> <row> <col> [--session s]` | Scroll wheel event at position |

### Waiting

| Command | Description |
|---------|-------------|
| `wait <ms>` | Hard wait (last resort) |
| `wait --text "str" [--timeout ms] [--interval ms]` | Poll until text appears on screen |
| `wait --text-any "str1" "str2" ...` | Poll until any of the texts appears (OR semantics). Conflicts with `--text`. |
| `wait --text-gone "str"` | Poll until text disappears |
| `wait --stable <ms>` | Poll until screen unchanged for N ms |
| `wait --cursor <row,col>` | Poll until cursor reaches position |
| `wait --regex "pattern"` | Poll until regex matches screen content |
| `wait --exit` | Poll until the process exits |

Default timeout: 10000ms. Default poll interval: 50ms.

All `wait` conditions also accept `--capture [path]` and `--screenshot [path]` to save text/PNG after the condition is met.

All `wait` conditions accept `--json` for structured timeout errors: `{error, condition, elapsed_ms, timeout_ms, last_snapshot, session}`. For `--stable`, also includes `last_stable_duration_ms` and `change_count`.

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
| `status [--session s] [--pane p] [--json]` | Process status. Shows pane layout when multiple panes exist (pane IDs, sizes, positions). JSON includes `panes` array and `last_stderr` when process is dead. |
| `exit-code [--session s]` | Get the exit code of the terminated process |
| `logs [--session s] [--stderr]` | Read captured stderr/stdout |
| `signal <SIG> [--session s]` | Send a Unix signal (SIGINT, SIGTERM, SIGWINCH, etc.) |

### Clipboard

| Command | Description |
|---------|-------------|
| `clipboard read [--session s]` | Read the tmux paste buffer |
| `clipboard write "text" [--session s]` | Write to the tmux paste buffer |
| `clipboard paste [--session s]` | Paste from buffer into the pane |

### Recording & Replay

| Command | Description |
|---------|-------------|
| `record start [--session s] [--group g] [--label l] [--fps N] [--dir path]` | Start recording (background poller) |
| `record stop [--session s]` | Stop recording. Prints recording dir path. |
| `record list [--dir path] [--json]` | List all recordings |
| `record view --dir path [--all-frames] [--json]` | View recording as chronological text stream |
| `web [--dir path] [--port 8080]` | Web viewer for visual replay |

### Performance & Testing

| Command | Description |
|---------|-------------|
| `perf start/stop [--session s] [--json]` | Background frame recording with FPS metrics |
| `perf fps --duration <ms> [--session s]` | Passive FPS observation |
| `perf latency [--session s] [--key k] [--samples N] [--json]` | Keystroke-to-render latency |
| `test-matrix --command "cmd" --test "cmds" [--sizes] [--terms] [--colors]` | Test across terminal configurations |
| `doctor` | Validate tmux version and capabilities |
| `watch [--interval ms] [--filter prefix]` | Live dashboard (interactive) |

## Failure Recovery

**Snapshot shows nothing / stale content:**
```bash
agent-terminal status --session s1 --json
# If alive=false: last_stderr is included in JSON -- no separate logs call needed
# If alive=true: app may still be loading
agent-terminal wait --stable 1000 --session s1 --timeout 15000
```

**wait timed out:** The error includes the last snapshot. Use `--json` for structured diagnostics (elapsed_ms, change_count). Common causes: wrong expected text, animation/spinner (use longer `--stable`), or app crashed (check `status`).

**Branching outcomes (success vs error):**
```bash
# Wait for either success or error message in one call
agent-terminal wait --text-any "Success" "Error:" "Failed" --session s1
```

**Session already exists:** Use `--replace` to kill and re-open in one call: `agent-terminal open "./app" --session s1 --replace`.

## Key Distinctions

- **`send` vs `type`**: `send "Enter"` sends the Enter key. `type "Enter"` types the letters E-n-t-e-r. Use `type` for text input, `send` for key presses.
- **`send "C-c"` vs `signal SIGINT`**: `send` goes through the app's input handler. `signal` bypasses it. Usually prefer `send "C-c"`.
- **Full-screen TUI vs scrolling CLI**: Full-screen apps (ratatui, vim) -- `snapshot` captures everything, scrollback is useless. Scrolling apps (REPL, build output) -- use `scrollback --lines N` or `snapshot --scrollback N` for history.
- **`paste` vs `type`**: Use `paste` for multi-line text or special characters. Use `type` for simple single-line input.

## Multi-Pane Workflows

Run two programs side by side and observe both at once:

```bash
# Open first program
agent-terminal open "./app-v1" --session compare --size 180x40

# Open second program in a split pane
agent-terminal open "./app-v2" --session compare --pane right

# Check status to discover pane IDs and layout
agent-terminal status --session compare
# Output includes: Panes: 2
#   %0  [90x40 at 0,0]  "left"  (active)
#   %1  [89x40 at 91,0]  "right"
#   Hint: use --window to capture all panes, or --pane <id> for a specific one

# Snapshot all panes composited in layout
agent-terminal snapshot --window --session compare

# JSON snapshot with per-pane layout, cursor, and lines
agent-terminal snapshot --window --json --session compare

# Screenshot both panes as one image
agent-terminal screenshot --window --session compare --path compare.png

# Interact with a specific pane using its ID
agent-terminal send --pane %0 --session compare j
agent-terminal snapshot --pane %1 --session compare
```

Pane IDs (`%0`, `%1`, ...) are returned by `status` and work with `--pane` on all commands. Use `--window` on `snapshot`/`screenshot` to capture all panes composited. `--window` conflicts with `--pane`.

## Reference Docs

For detailed guides on specific topics, read:
- `references/recording.md` -- session recording, before/after pattern, reviewing recordings
- `references/performance.md` -- FPS measurement, latency testing, interpreting results
- `references/matrix-testing.md` -- cross-configuration testing
