# agent-terminal CLI Reference

All commands default to `--session agent-terminal` unless specified.

---

## Lifecycle

### `open`

Launch a command in a new tmux session.

```
agent-terminal open "<command>" [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--pane <name>` | none | Create a named pane within an existing session (splits horizontally) |
| `--env <KEY=VAL>` | none | Set environment variables (repeatable) |
| `--size <COLSxROWS>` | tmux default | Initial terminal size (e.g., `80x24`) |

The command is wrapped to capture stderr to `/tmp/agent-terminal-<session>-stderr` and exit code to `/tmp/agent-terminal-<session>-exit`.

Waits up to 2 seconds for the first non-empty render before returning.

Prints the session name on success.

```bash
agent-terminal open "htop"
agent-terminal open "cargo run" --session myapp --size 120x40
agent-terminal open "./server" --session myapp --pane server
agent-terminal open "./my-app" --env TERM=dumb --env NO_COLOR=1
agent-terminal open "python app.py" --session py --size 40x10 --env LANG=C
```

### `close`

Kill a tmux session and clean up temp files.

```
agent-terminal close [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session to close |

```bash
agent-terminal close
agent-terminal close --session myapp
```

### `list`

List all active tmux sessions.

```
agent-terminal list
```

No flags. Shows session name, creation timestamp, and window count. Sessions with names starting with `agent-terminal` are tagged.

```bash
agent-terminal list
# SESSION                        CREATED                  WINDOWS
# agent-terminal                 1711234567               1 [agent-terminal]
# myapp                          1711234500               2
```

---

## Observation

### `snapshot`

Capture the current terminal content.

```
agent-terminal snapshot [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--pane <name>` | none | Target pane |
| `--color` | off | Annotate lines with parsed color/style info |
| `--raw` | off | Raw byte stream with ANSI escapes, no formatting |
| `--ansi` | off | ANSI escapes preserved, with row numbers and header |
| `--json` | off | Structured JSON with text + color spans |
| `--diff` | off | Diff against the previous snapshot |
| `--scrollback <N>` | none | Include N lines of scrollback above viewport |

Output modes are mutually exclusive. Default is plain text with row numbers:

```
[size: 80x24  cursor: 3,12  session: agent-terminal]
-----------------------------------------
  1| File  Edit  View  Help
  2| ---------------------
  3| > item one
  4|   item two
  5|   item three
```

```bash
agent-terminal snapshot
agent-terminal snapshot --color --session myapp
agent-terminal snapshot --json --session myapp
agent-terminal snapshot --raw --session myapp
agent-terminal snapshot --diff --session myapp
agent-terminal snapshot --scrollback 50 --session myapp
```

### `screenshot`

Render the terminal as a PNG or HTML image.

```
agent-terminal screenshot [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--path <file>` | `screenshot.png` / `screenshot.html` | Output file path |
| `--annotate` | off | Overlay row/column grid numbers |
| `--html` | off | Save as HTML instead of PNG |
| `--theme <dark\|light>` | `dark` | Color theme |

```bash
agent-terminal screenshot
agent-terminal screenshot --path ./shots/test1.png --annotate
agent-terminal screenshot --html --theme light --path report.html
```

### `scrollback`

Read the tmux scrollback buffer (content that has scrolled off screen).

```
agent-terminal scrollback [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--lines <N>` | all | Limit to last N lines |
| `--search <text>` | none | Search scrollback for text, return matching lines with context |

```bash
agent-terminal scrollback --session myapp
agent-terminal scrollback --lines 200 --session myapp
agent-terminal scrollback --search "error" --session myapp
```

### `find`

Search the current screen content for text.

```
agent-terminal find "<pattern>" [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--all` | off | Return all matches (default: first only) |
| `--regex` | off | Interpret pattern as regex |
| `--color <style>` | none | Filter by color/style (e.g., `"fg:red"`, `"bold"`) |

Returns `row N, col N: "context"` for each match. Exits 0 on match, 1 on no match.

```bash
agent-terminal find "Error"
agent-terminal find "Error" --all --session myapp
agent-terminal find "v\d+\.\d+" --regex --session myapp
agent-terminal find "" --color "fg:red" --all --session myapp    # all red text
agent-terminal find "Warning" --color "fg:yellow" --session myapp
```

---

## Interaction

### `send`

Send one or more named key sequences to the terminal.

```
agent-terminal send <keys>... [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--pane <name>` | none | Target pane |

Keys are sent individually via `tmux send-keys`. Use key names for special keys.

**Key name examples**: `Enter`, `Escape`, `Tab`, `Space`, `BSpace` (backspace), `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PgUp`, `PgDn`, `F1`-`F12`, `C-c` (Ctrl+C), `C-a`, `C-z`, `M-x` (Alt+X).

```bash
agent-terminal send "j"                   # single key
agent-terminal send Enter                 # special key
agent-terminal send C-c                   # ctrl+c
agent-terminal send Up Up Up Enter        # multiple keys in sequence
agent-terminal send "M-x" --session myapp # alt+x
```

### `type`

Type literal text (no key-name interpretation).

```
agent-terminal type "<text>" [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--pane <name>` | none | Target pane |

Uses `tmux send-keys -l` under the hood. Characters are sent literally -- `"Enter"` types the five characters E-n-t-e-r, it does not press the Enter key.

```bash
agent-terminal type "hello world" --session myapp
agent-terminal type "grep -r 'pattern' ." --session myapp
```

### `paste`

Paste text via the tmux paste buffer. Handles special characters and multi-line text safely.

```
agent-terminal paste "<text>" [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--pane <name>` | none | Target pane |

Loads text into a tmux buffer then pastes it. More reliable than `type` for large or multi-line text.

```bash
agent-terminal paste "multi\nline\ntext" --session myapp
agent-terminal paste "function() { return 42; }" --session myapp
```

### `resize`

Resize the terminal pane and window.

```
agent-terminal resize <cols> <rows> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--pane <name>` | none | Target pane |

Resizes both the tmux window and pane to the specified dimensions.

```bash
agent-terminal resize 120 40 --session myapp
agent-terminal resize 40 10 --session myapp   # small terminal test
agent-terminal resize 200 50 --session myapp  # wide terminal test
```

### `click`

Send a mouse click at a position (1-indexed row and column).

```
agent-terminal click <row> <col> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--right` | off | Right click |
| `--double` | off | Double click |

Uses SGR mouse encoding. The app must have mouse support enabled.

```bash
agent-terminal click 5 10                          # left click at row 5, col 10
agent-terminal click 5 10 --right --session myapp  # right click
agent-terminal click 3 15 --double --session myapp # double click
```

### `drag`

Send a mouse drag from one position to another.

```
agent-terminal drag <r1> <c1> <r2> <c2> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |

Emits a button press at (r1,c1) and release at (r2,c2).

```bash
agent-terminal drag 3 1 3 20 --session myapp    # select text on row 3
agent-terminal drag 5 10 15 10 --session myapp   # drag vertically
```

### `scroll-wheel`

Send a scroll wheel event at a position.

```
agent-terminal scroll-wheel <up|down> <row> <col> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |

```bash
agent-terminal scroll-wheel up 10 40 --session myapp
agent-terminal scroll-wheel down 10 40 --session myapp
```

---

## Waiting

### `wait`

Wait for a condition to be met. Exactly one condition should be specified.

```
agent-terminal wait [<ms>] [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `<ms>` | none | Hard wait in milliseconds (positional argument) |
| `--text <str>` | none | Wait until text appears on screen |
| `--text-gone <str>` | none | Wait until text disappears from screen |
| `--stable <ms>` | none | Wait until screen unchanged for N ms |
| `--cursor <row,col>` | none | Wait until cursor reaches position |
| `--regex <pattern>` | none | Wait until regex matches screen content |
| `--session <name>` | `agent-terminal` | Session name |
| `--timeout <ms>` | `10000` | Maximum wait time before failing |
| `--interval <ms>` | `50` | Polling interval |

On success, prints the final snapshot. On timeout, prints a diagnostic error including the last snapshot, session state, and hints.

```bash
agent-terminal wait --text "Ready" --session myapp
agent-terminal wait --text-gone "Loading..." --session myapp --timeout 30000
agent-terminal wait --stable 500 --session myapp
agent-terminal wait --cursor 5,10 --session myapp
agent-terminal wait --regex "v\d+\.\d+" --session myapp
agent-terminal wait 2000    # hard wait, use as last resort
```

---

## Assertion

### `assert`

Check a condition. Exits 0 on pass, exits 1 with diagnostic output on fail.

```
agent-terminal assert [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--text <str>` | none | Assert text is present on screen |
| `--no-text <str>` | none | Assert text is absent from screen |
| `--row <N>` | none | Row number for row-specific assertion |
| `--row-text <str>` | none | Text to check in the specified row (requires `--row`) |
| `--cursor-row <N>` | none | Assert cursor is on this row |
| `--color <N>` | none | Row number for color/style assertion |
| `--color-style <str>` | none | Style spec to check (requires `--color`). Format: `"fg:red,bold"` |
| `--style <str>` | none | Find this text and check its style |
| `--style-check <str>` | none | Expected style for `--style` text. Format: `"fg:red,bold"` |
| `--session <name>` | `agent-terminal` | Session name |

Style spec format: comma-separated list of `fg:<color>`, `bg:<color>`, `bold`, `dim`, `italic`, `underline`, `reverse`, `strikethrough`.

```bash
agent-terminal assert --text "Welcome" --session myapp
agent-terminal assert --no-text "Error" --session myapp
agent-terminal assert --row 1 --row-text "File  Edit" --session myapp
agent-terminal assert --cursor-row 3 --session myapp
agent-terminal assert --color 5 --color-style "fg:red" --session myapp
agent-terminal assert --color 3 --color-style "fg:green,bold,reverse" --session myapp
agent-terminal assert --style "Error" --style-check "fg:red" --session myapp
```

---

## Process Health

### `status`

Get process status information.

```
agent-terminal status [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--pane <name>` | none | Target pane |
| `--json` | off | Output as JSON |

JSON output: `{"alive": bool, "pid": int, "exit_code": int|null, "signal": null, "runtime_ms": int}`

```bash
agent-terminal status --session myapp
agent-terminal status --session myapp --json
```

### `exit-code`

Get the exit code of the terminated process.

```
agent-terminal exit-code [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |

Prints the exit code if available, "Process still running" if alive, or an error with diagnostic info.

```bash
agent-terminal exit-code --session myapp
```

### `logs`

Read captured stderr and stdout scrollback.

```
agent-terminal logs [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--stderr` | off | Show stderr only (skip stdout scrollback) |

Without `--stderr`, shows both stderr and stdout (tmux scrollback) sections.

```bash
agent-terminal logs --session myapp
agent-terminal logs --stderr --session myapp
```

### `signal`

Send a Unix signal to the process running in the pane.

```
agent-terminal signal <SIGNAL> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |

Accepts signal names with or without `SIG` prefix, case-insensitive. Supported: SIGINT, SIGTERM, SIGWINCH, SIGTSTP, SIGCONT, SIGHUP, SIGKILL, SIGUSR1, SIGUSR2.

```bash
agent-terminal signal SIGINT --session myapp
agent-terminal signal TERM --session myapp
agent-terminal signal SIGWINCH --session myapp
agent-terminal signal SIGTSTP --session myapp    # suspend
agent-terminal signal SIGCONT --session myapp    # resume
```

---

## Clipboard

### `clipboard`

Clipboard operations via tmux paste buffer.

```
agent-terminal clipboard <read|write|paste> [text] [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |

```bash
agent-terminal clipboard read --session myapp           # read paste buffer
agent-terminal clipboard write "copied text" --session myapp  # set paste buffer
agent-terminal clipboard paste --session myapp          # paste buffer into pane
```

---

## Performance

### `perf start`

Start background frame recording.

```
agent-terminal perf start [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |

Spawns a background poller that records frame changes every ~10ms. Use `perf stop` to retrieve metrics.

### `perf stop`

Stop frame recording and return metrics.

```
agent-terminal perf stop [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--json` | off | Output as JSON |

Human-readable output includes FPS, frame count, frame time stats, and a rating.

JSON output:
```json
{
  "fps": 24.5,
  "frame_count": 73,
  "duration_ms": 2980,
  "min_frame_ms": 33,
  "max_frame_ms": 120,
  "mean_frame_ms": 40.0,
  "p95_frame_ms": 88,
  "idle_ms": 450,
  "timeline": [{"t_ms": 0, "frame_ms": 33}, ...]
}
```

### `perf fps`

Measure FPS inline (single command).

```
agent-terminal perf fps [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--during <cmds>` | none | Agent-terminal commands to run while measuring (joined with `&&`) |
| `--during-batch` | off | Read JSON batch of commands from stdin |
| `--duration <ms>` | none | Passively observe for N milliseconds |

Exactly one of `--during`, `--during-batch`, or `--duration` is required. Always outputs JSON.

```bash
agent-terminal perf fps --duration 3000 --session myapp
agent-terminal perf fps --during 'send "j" && send "k"' --session myapp
echo '[{"cmd":"send","args":["j"]},{"cmd":"wait","args":["--stable","100"]}]' | agent-terminal perf fps --during-batch --session myapp
```

### `perf latency`

Measure input latency (keystroke to screen update).

```
agent-terminal perf latency [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--session <name>` | `agent-terminal` | Session name |
| `--key <key>` | `space` | Key to test |
| `--samples <N>` | `5` | Number of measurements |
| `--json` | off | Output as JSON |

Sends the key, polls for screen change at 1ms intervals, reports statistics. Default key is space (cancelled with backspace).

JSON output:
```json
{
  "mean_ms": 18.0,
  "min_ms": 8,
  "max_ms": 45,
  "p95_ms": 38,
  "samples": 10,
  "measurements": [8, 12, 15, 18, 20, 22, 25, 30, 38, 45]
}
```

Latency ratings: <16ms excellent, 16-50ms good, 50-100ms fair, 100-200ms poor, >200ms bad.

---

## Environment & Testing

### `doctor`

Validate the environment for agent-terminal compatibility.

```
agent-terminal doctor
```

Checks: tmux version (>= 3.0), session creation, capture-pane, ANSI capture, mouse support, resize, send-keys, paste-buffer. Each failure includes a fix suggestion.

### `init`

Detect the TUI framework in the current directory and generate a starter test.

```
agent-terminal init
```

Detects frameworks from: `Cargo.toml` (ratatui, crossterm, cursive), `go.mod` (bubbletea, tview, termui), `package.json` (ink, blessed, terminal-kit), `requirements.txt`/`pyproject.toml` (textual, rich, curses).

Creates `tests/tui/basic_test.sh` with appropriate run command and wait times.

### `test-matrix`

Run tests across multiple terminal configurations.

```
agent-terminal test-matrix [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--command <cmd>` | required | Command to test |
| `--test <cmds>` | required | Semicolon-separated test commands to run. Use `{session}` as placeholder |
| `--sizes <list>` | `80x24,120x40,40x10` | Comma-separated terminal sizes |
| `--terms <list>` | `xterm-256color,dumb` | Comma-separated TERM values |
| `--colors <list>` | `default,NO_COLOR=1` | Comma-separated color modes (env var settings) |

Runs every combination, reports pass/fail, saves failure snapshots to `./agent-terminal-matrix/`.

```bash
agent-terminal test-matrix \
  --command "./my-app" \
  --sizes "80x24,40x10" \
  --terms "xterm-256color,dumb" \
  --colors "default,NO_COLOR=1" \
  --test "agent-terminal assert --text 'Ready' --session {session}"
```

### `a11y-check`

Run accessibility checks against a TUI application.

```
agent-terminal a11y-check "<command>"
```

Runs 5 checks:
1. **NO_COLOR respected** -- no ANSI color codes with `NO_COLOR=1`
2. **TERM=dumb fallback** -- app doesn't crash with `TERM=dumb`
3. **Resize handling** -- app survives resize to 40x10
4. **Focus visible** -- skipped (manual verification recommended)
5. **Contrast** -- warns if dim/faint text detected

Saves failure details to `./a11y-report/`.

```bash
agent-terminal a11y-check "./my-app"
agent-terminal a11y-check "cargo run"
```
