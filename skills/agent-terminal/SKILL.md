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
| `open "<cmd>" [--session s] [--pane p] [--env K=V]... [--size COLSxROWS\|vertical\|landscape] [--shell] [--no-stderr]` | Launch command in tmux. Default: 112x30. Presets: `vertical` (80x55), `landscape` (112x30). |
| `close [--session s]` | Kill the session and clean up temp files |
| `list` | List all active tmux sessions |

### Observation

| Command | Description |
|---------|-------------|
| `snapshot [--session s] [--pane p] [--color] [--raw] [--ansi] [--json] [--diff] [--scrollback N]` | Capture current terminal content |
| `screenshot [--session s] [--path f] [--annotate] [--html] [--theme dark\|light]` | Render as PNG or HTML. Default: `<session>-<timestamp>.png` |
| `scrollback [--session s] [--lines N] [--search "text"]` | Read the tmux scrollback buffer |
| `find "pattern" [--session s] [--all] [--regex] [--color "style"]` | Search screen for text, return row,col |

### Interaction

| Command | Description |
|---------|-------------|
| `send <keys>... [--session s] [--pane p] [--wait-stable ms]` | Send key sequences (e.g., `Enter`, `C-c`, `j`). `--wait-stable` waits then prints snapshot. |
| `type "text" [--session s] [--pane p] [--enter] [--wait-stable ms]` | Type literal text. `--enter` sends Enter after. `--wait-stable` waits then prints snapshot. |
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
| `wait --text-gone "str"` | Poll until text disappears |
| `wait --stable <ms>` | Poll until screen unchanged for N ms |
| `wait --cursor <row,col>` | Poll until cursor reaches position |
| `wait --regex "pattern"` | Poll until regex matches screen content |
| `wait --exit` | Poll until the process exits |

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
# If alive=false: check logs and exit code
agent-terminal logs --stderr --session s1
# If alive=true: app may still be loading
agent-terminal wait --stable 1000 --session s1 --timeout 15000
```

**wait timed out:** The error includes the last snapshot. Common causes: wrong expected text, animation/spinner (use longer `--stable`), or app crashed (check `status`).

**Session already exists:** `agent-terminal close --session s1` then re-open.

## Key Distinctions

- **`send` vs `type`**: `send "Enter"` sends the Enter key. `type "Enter"` types the letters E-n-t-e-r. Use `type` for text input, `send` for key presses.
- **`send "C-c"` vs `signal SIGINT`**: `send` goes through the app's input handler. `signal` bypasses it. Usually prefer `send "C-c"`.
- **Full-screen TUI vs scrolling CLI**: Full-screen apps (ratatui, vim) -- `snapshot` captures everything, scrollback is useless. Scrolling apps (REPL, build output) -- use `scrollback --lines N` or `snapshot --scrollback N` for history.
- **`paste` vs `type`**: Use `paste` for multi-line text or special characters. Use `type` for simple single-line input.

## Reference Docs

For detailed guides on specific topics, read:
- `references/recording.md` -- session recording, before/after pattern, reviewing recordings
- `references/performance.md` -- FPS measurement, latency testing, interpreting results
- `references/matrix-testing.md` -- cross-configuration testing
