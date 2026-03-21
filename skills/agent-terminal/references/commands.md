# agent-terminal CLI Reference

Complete reference for every command, flag, and option.

All commands default to `--session agent-terminal` when `--session` is not specified.

---

## Lifecycle

### `open <command>`

Launch a command inside a new tmux session.

**Syntax:**

```
agent-terminal open "<command>" [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Name for the tmux session |
| `--pane` | string | _(none)_ | Named pane within the session |
| `--env` | string | _(none)_ | Environment variable in `KEY=VAL` format. Repeatable. |
| `--size` | string | _(none)_ | Initial terminal dimensions as `COLSxROWS` |
| `--shell` | bool | `false` | Keep session alive after command exits. Wraps the command so a shell takes over when it finishes. |
| `--no-stderr` | bool | `false` | Don't capture stderr. Needed for bash/readline apps where PS1 prompts go through stderr. |

**Examples:**

```bash
# Basic launch
agent-terminal open "./my-app"

# Named session with custom size
agent-terminal open "./my-app" --session test1 --size 120x40

# With environment variables
agent-terminal open "./my-app" --session test1 --env NO_COLOR=1 --env TERM=dumb

# Named pane for multi-pane setups
agent-terminal open "./server" --session multi --pane server

# Keep session alive after a fast-exiting command (e.g., grep, curl)
agent-terminal open "grep -r 'TODO' src/" --session grep1 --shell

# Test bash with visible prompts (readline sends PS1 via stderr)
agent-terminal open "bash" --session bash1 --no-stderr --env PS1='$ '
```

---

### `close`

Kill a tmux session and clean up associated temp files.

**Syntax:**

```
agent-terminal close [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Session to kill |

**Examples:**

```bash
agent-terminal close
agent-terminal close --session test1
```

---

### `list`

List all active tmux sessions managed by agent-terminal.

**Syntax:**

```
agent-terminal list
```

**Flags:** None.

**Example:**

```bash
agent-terminal list
# Output:
# agent-terminal  (80x24, pid 12345, alive)
# test1           (120x40, pid 12346, alive)
```

---

## Observation

### `snapshot`

Capture the current terminal screen contents. This is the primary observation command.

**Syntax:**

```
agent-terminal snapshot [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Session to capture |
| `--pane` | string | _(none)_ | Named pane to capture |
| `--color` | bool | `false` | Include style annotations per line |
| `--raw` | bool | `false` | Raw byte stream with no formatting |
| `--ansi` | bool | `false` | Raw ANSI escape sequences with row numbers |
| `--json` | bool | `false` | Full structured JSON output |
| `--diff` | bool | `false` | Show only rows that changed since last snapshot |
| `--scrollback` | integer | _(none)_ | Include N lines of scrollback above viewport |

Output format flags (`--color`, `--raw`, `--ansi`, `--json`, `--diff`) are mutually exclusive. See [snapshot-format.md](snapshot-format.md) for format details.

**Examples:**

```bash
# Default plain-text snapshot
agent-terminal snapshot --session test1
# [size: 80x24  cursor: 1,0  session: test1]
# -----------------------------------------
#   1| Welcome to my-app
#   2| > Option A
#   3|   Option B

# With color/style annotations
agent-terminal snapshot --color --session test1

# JSON for programmatic parsing
agent-terminal snapshot --json --session test1

# Diff to see what changed
agent-terminal snapshot --diff --session test1

# Include 50 lines of scrollback
agent-terminal snapshot --scrollback 50 --session test1
```

---

### `screenshot`

Render the terminal as a PNG image or HTML file.

**Syntax:**

```
agent-terminal screenshot [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Session to capture |
| `--path` | string | auto-generated | Output file path |
| `--annotate` | bool | `false` | Overlay row/column grid on the image |
| `--html` | bool | `false` | Save as HTML instead of PNG |
| `--theme` | string | `dark` | Color theme: `dark` or `light` |

**Examples:**

```bash
# Default PNG screenshot
agent-terminal screenshot --session test1

# Annotated with grid overlay
agent-terminal screenshot --annotate --path ./debug.png --session test1

# HTML output with light theme
agent-terminal screenshot --html --theme light --session test1
```

---

### `scrollback`

Read the tmux scrollback buffer (content that has scrolled off the visible viewport).

**Syntax:**

```
agent-terminal scrollback [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Session to read |
| `--lines` | integer | _(all)_ | Number of lines to return |
| `--search` | string | _(none)_ | Search scrollback for matching text |

**Examples:**

```bash
# Read last 100 lines of scrollback
agent-terminal scrollback --lines 100 --session test1

# Search for errors in scrollback
agent-terminal scrollback --search "error" --session test1
```

---

### `find <pattern>`

Search the visible screen for text and return matching positions.

**Syntax:**

```
agent-terminal find "<pattern>" [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Session to search |
| `--all` | bool | `false` | Return all matches (not just first) |
| `--regex` | bool | `false` | Treat pattern as a regular expression |
| `--color` | string | _(none)_ | Search by color style (e.g., `"fg:red"`) |

**Examples:**

```bash
# Find first occurrence
agent-terminal find "Error" --session test1
# Output: 5,12  (row 5, column 12)

# Find all occurrences
agent-terminal find ">" --all --session test1
# Output:
# 2,1
# 7,1

# Regex search
agent-terminal find "v[0-9]+\.[0-9]+" --regex --session test1

# Find by color
agent-terminal find "Error" --color "fg:red" --session test1
```

---

## Interaction

### `send <keys...>`

Send named key sequences to the terminal. Keys are interpreted as tmux key names.

**Syntax:**

```
agent-terminal send <key> [<key>...] [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--pane` | string | _(none)_ | Target pane |

**Common key names:** `Enter`, `Escape`, `Tab`, `Space`, `BSpace` (backspace), `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PgUp`, `PgDn`, `F1`-`F12`, `C-c` (Ctrl+C), `C-d`, `C-z`, `M-x` (Alt+X).

Single characters (`j`, `k`, `q`, `/`) are sent as-is.

**Examples:**

```bash
# Send a single key
agent-terminal send Enter --session test1

# Send multiple keys in sequence
agent-terminal send Down Down Down Enter --session test1

# Ctrl+C to interrupt
agent-terminal send C-c --session test1

# Vim-style navigation
agent-terminal send g g --session test1    # go to top
agent-terminal send G --session test1      # go to bottom
```

---

### `type <text>`

Type literal text character-by-character. Unlike `send`, no key-name interpretation occurs.

**Syntax:**

```
agent-terminal type "<text>" [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--pane` | string | _(none)_ | Target pane |

**Examples:**

```bash
# Type into an input field
agent-terminal type "hello world" --session test1

# Type a search query
agent-terminal type "/search term" --session test1

# type vs send: "Enter" types the five characters E-n-t-e-r
agent-terminal type "Enter" --session test1    # types literal "Enter"
agent-terminal send Enter --session test1      # presses the Enter key
```

---

### `paste <text>`

Paste text via the tmux paste buffer. Safer than `type` for text containing special characters, newlines, or shell metacharacters.

**Syntax:**

```
agent-terminal paste "<text>" [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--pane` | string | _(none)_ | Target pane |

**Examples:**

```bash
# Paste multi-line content
agent-terminal paste "line1\nline2\nline3" --session test1

# Paste text with special characters
agent-terminal paste 'echo "hello $USER"' --session test1
```

---

### `resize <cols> <rows>`

Resize the terminal to the specified dimensions. Triggers SIGWINCH in the running process.

**Syntax:**

```
agent-terminal resize <cols> <rows> [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--pane` | string | _(none)_ | Target pane |

**Examples:**

```bash
# Standard size
agent-terminal resize 80 24 --session test1

# Wide terminal
agent-terminal resize 200 50 --session test1

# Narrow mobile-like size
agent-terminal resize 40 10 --session test1
```

---

### `click <row> <col>`

Send a mouse click at the given position. Coordinates are 1-indexed.

**Syntax:**

```
agent-terminal click <row> <col> [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--right` | bool | `false` | Right-click instead of left-click |
| `--double` | bool | `false` | Double-click |

**Examples:**

```bash
# Left-click at row 3, column 10
agent-terminal click 3 10 --session test1

# Right-click for context menu
agent-terminal click 5 20 --right --session test1

# Double-click to select word
agent-terminal click 3 10 --double --session test1
```

---

### `drag <r1> <c1> <r2> <c2>`

Mouse drag from one position to another. Useful for text selection or slider interaction.

**Syntax:**

```
agent-terminal drag <r1> <c1> <r2> <c2> [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |

**Examples:**

```bash
# Select text from row 3 col 1 to row 3 col 20
agent-terminal drag 3 1 3 20 --session test1

# Drag slider from row 10 col 5 to row 10 col 40
agent-terminal drag 10 5 10 40 --session test1
```

---

### `scroll-wheel <up|down> <row> <col>`

Send a scroll wheel event at the given position.

**Syntax:**

```
agent-terminal scroll-wheel <direction> <row> <col> [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |

**Examples:**

```bash
# Scroll up at center of screen
agent-terminal scroll-wheel up 12 40 --session test1

# Scroll down in a list area
agent-terminal scroll-wheel down 15 10 --session test1
```

---

## Waiting

### `wait`

Wait for a condition before proceeding. Essential for synchronizing with async terminal updates.

**Syntax:**

```
agent-terminal wait [<ms>] [flags]
```

**Modes (mutually exclusive):**

| Flag | Type | Description |
|------|------|-------------|
| _(positional)_ | integer | Hard wait for N milliseconds (last resort) |
| `--text` | string | Poll until text appears on screen |
| `--text-gone` | string | Poll until text disappears from screen |
| `--stable` | integer | Poll until screen unchanged for N milliseconds |
| `--cursor` | string | Poll until cursor reaches `row,col` position |
| `--regex` | string | Poll until regex pattern matches screen content |
| `--exit` | bool | Poll until the process exits (checks tmux `#{pane_dead}`) |

**Common flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--timeout` | integer | `10000` | Maximum wait time in ms before failing |
| `--interval` | integer | `50` | Poll interval in ms |

**Examples:**

```bash
# Wait for text to appear
agent-terminal wait --text "Ready" --session test1

# Wait for loading to finish
agent-terminal wait --text-gone "Loading..." --session test1

# Wait for screen to stabilize (no changes for 500ms)
agent-terminal wait --stable 500 --session test1

# Wait for cursor position
agent-terminal wait --cursor 5,1 --session test1

# Wait for regex match
agent-terminal wait --regex "v[0-9]+\.[0-9]+\.[0-9]+" --session test1

# Custom timeout
agent-terminal wait --text "Compiled" --timeout 30000 --session test1

# Wait for process to exit (more reliable than sleep)
agent-terminal wait --exit --session test1

# Hard wait (avoid when possible)
agent-terminal wait 1000 --session test1
```

---

## Assertion

### `assert`

Check a condition and exit 0 (pass) or exit 1 (fail). On failure, prints a diagnostic snapshot.

**Syntax:**

```
agent-terminal assert [flags]
```

**Assertion modes:**

| Flag | Type | Description |
|------|------|-------------|
| `--text` | string | Assert text is present on screen |
| `--no-text` | string | Assert text is absent from screen |
| `--row` + `--row-text` | int + string | Assert specific row contains text |
| `--cursor-row` | integer | Assert cursor is on the given row |
| `--color` + `--color-style` | int + string | Assert row N has the given style |
| `--style` + `--style-check` | string + string | Assert specific text has a style |

**Common flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |

**Examples:**

```bash
# Check text is present
agent-terminal assert --text "Welcome" --session test1

# Check text is absent
agent-terminal assert --no-text "Error" --session test1

# Check specific row content
agent-terminal assert --row 1 --row-text "Title Bar" --session test1

# Check cursor position
agent-terminal assert --cursor-row 3 --session test1

# Check color on a row
agent-terminal assert --color 5 --color-style "fg:red,bold" --session test1

# Check style of specific text
agent-terminal assert --style "Error" --style-check "fg:red" --session test1
```

---

## Process Health

### `status`

Check whether the process is alive, dead, or crashed.

**Syntax:**

```
agent-terminal status [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--pane` | string | _(none)_ | Target pane |
| `--json` | bool | `false` | Output as JSON |

**Examples:**

```bash
# Human-readable
agent-terminal status --session test1
# Output: alive (pid 12345)

# JSON for scripting
agent-terminal status --json --session test1
# Output: {"alive": true, "pid": 12345, "command": "./my-app"}
```

---

### `exit-code`

Get the exit code of a terminated process. Only valid after the process has exited.

**Syntax:**

```
agent-terminal exit-code [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |

**Examples:**

```bash
agent-terminal exit-code --session test1
# Output: 0
```

---

### `logs`

Read captured stdout/stderr from the process.

**Syntax:**

```
agent-terminal logs [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--stderr` | bool | `false` | Show only stderr output |

**Examples:**

```bash
# Read all logs
agent-terminal logs --session test1

# Read only stderr (useful for crash diagnostics)
agent-terminal logs --stderr --session test1
```

---

### `signal <SIGNAL>`

Send a Unix signal to the running process.

**Syntax:**

```
agent-terminal signal <SIGNAL> [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |

**Common signals:** `SIGINT`, `SIGTERM`, `SIGKILL`, `SIGWINCH`, `SIGSTOP`, `SIGCONT`, `SIGHUP`, `SIGUSR1`, `SIGUSR2`.

**Examples:**

```bash
# Graceful termination
agent-terminal signal SIGTERM --session test1

# Force kill
agent-terminal signal SIGKILL --session test1

# Trigger resize handler
agent-terminal signal SIGWINCH --session test1

# Test interrupt handling
agent-terminal signal SIGINT --session test1
```

---

## Clipboard

### `clipboard <operation> [text]`

Interact with the tmux paste buffer (clipboard).

**Syntax:**

```
agent-terminal clipboard <read|write|paste> [text] [flags]
```

**Operations:**

| Operation | Description |
|-----------|-------------|
| `read` | Read the current tmux paste buffer content |
| `write` | Write text to the tmux paste buffer |
| `paste` | Paste from the buffer into the active pane |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |

**Examples:**

```bash
# Write to clipboard, then paste
agent-terminal clipboard write "some text" --session test1
agent-terminal clipboard paste --session test1

# Read clipboard contents
agent-terminal clipboard read --session test1
```

---

## Performance

### `perf start`

Begin background frame recording. Frames are sampled until `perf stop` is called.

**Syntax:**

```
agent-terminal perf start [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |

---

### `perf stop`

Stop frame recording and return performance metrics.

**Syntax:**

```
agent-terminal perf stop [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--json` | bool | `false` | Output as JSON |

**Example:**

```bash
agent-terminal perf start --session test1
# ... perform actions ...
agent-terminal perf stop --json --session test1
# {"fps": 24.5, "frame_count": 12, "p95_frame_ms": 88, ...}
```

---

### `perf fps`

Measure frames per second using various methods.

**Syntax:**

```
agent-terminal perf fps [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--during` | string | _(none)_ | Measure FPS while running a command string |
| `--during-batch` | bool | `false` | Read batch commands from stdin while measuring |
| `--duration` | integer | _(none)_ | Passively observe FPS for N milliseconds |

**Examples:**

```bash
# Passive observation for 3 seconds
agent-terminal perf fps --duration 3000 --session test1

# Measure during actions
agent-terminal perf fps --during 'send "j" && send "k"' --session test1
```

---

### `perf latency`

Measure keystroke-to-render input latency.

**Syntax:**

```
agent-terminal perf latency [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--session` | string | `agent-terminal` | Target session |
| `--key` | string | _(auto)_ | Key to test |
| `--samples` | integer | `5` | Number of measurement samples |
| `--json` | bool | `false` | Output as JSON |

**Example:**

```bash
agent-terminal perf latency --key "j" --samples 10 --json --session test1
# {"mean_ms": 18, "p95_ms": 38, "min_ms": 12, "max_ms": 45}
```

**Interpreting results:**

| Latency | Rating |
|---------|--------|
| < 16ms | Excellent |
| 16-50ms | Good |
| 50-100ms | Noticeable |
| > 100ms | Sluggish |

---

## Environment & Testing

### `doctor`

Validate the environment: check tmux version, capabilities, and dependencies.

**Syntax:**

```
agent-terminal doctor
```

**Flags:** None.

**Example:**

```bash
agent-terminal doctor
# tmux 3.4 ... OK
# capture-pane -e ... OK
# /tmp writable ... OK
# All checks passed
```

---

### `init`

Detect the project framework and generate a starter test file.

**Syntax:**

```
agent-terminal init
```

**Flags:** None.

**Example:**

```bash
agent-terminal init
# Detected: ratatui (Rust)
# Created: tests/tui-test.sh
```

---

### `test-matrix`

Run tests across multiple terminal configurations (sizes, TERM values, color modes).

**Syntax:**

```
agent-terminal test-matrix [flags]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--command` | string | _(required)_ | Command to launch for each combination |
| `--sizes` | string | `"80x24"` | Comma-separated terminal sizes (e.g., `"80x24,120x40,40x10"`) |
| `--terms` | string | `"xterm-256color"` | Comma-separated TERM values (e.g., `"xterm-256color,dumb"`) |
| `--colors` | string | `"default"` | Comma-separated color modes (e.g., `"default,NO_COLOR=1"`) |
| `--test` | string | _(required)_ | Test commands to run. Use `{session}` as placeholder. |

**Example:**

```bash
agent-terminal test-matrix \
  --command "./my-app" \
  --sizes "80x24,120x40,40x10" \
  --terms "xterm-256color,dumb" \
  --colors "default,NO_COLOR=1" \
  --test "agent-terminal assert --text 'Welcome' --session {session}"
# Runs 12 combinations (3 sizes x 2 terms x 2 colors)
```

---

### `a11y-check <command>`

Run an accessibility audit against the given command. Tests NO_COLOR support, TERM=dumb behavior, and resize handling.

**Syntax:**

```
agent-terminal a11y-check "<command>"
```

**Flags:** None (command is positional).

**Example:**

```bash
agent-terminal a11y-check "./my-app"
# Checking NO_COLOR support ... PASS
# Checking TERM=dumb fallback ... PASS
# Checking resize to 40x10 ... PASS
# Accessibility: 3/3 passed
```
