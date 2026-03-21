# agent-terminal

## The Problem

TUI apps are the only major category of software with **zero testing infrastructure**. Web apps have Playwright and Cypress. Mobile has Appium and XCUITest. Desktop has Accessibility APIs. Terminal apps have nothing.

The result is predictable: TUI apps ship with bugs that are trivial to catch but nobody catches because there's no way to automate the check. Common issues that plague TUI apps today:

- **Crash on resize** — shrink the terminal and the app panics, corrupts state, or renders garbage
- **Break on small terminals** — layouts that assume 80+ columns fall apart on narrow viewports
- **Ignore NO_COLOR** — apps that blast ANSI escapes into pipes and log files
- **Fail on TERM=dumb** — apps that crash instead of degrading gracefully when terminal capabilities are limited
- **Mouse interactions untested** — click handlers that work in manual testing but break under edge cases
- **Performance regressions invisible** — no way to measure if a change made the UI sluggish
- **No CI coverage** — TUI code is the only untested path in otherwise well-tested codebases

These aren't hard problems. They're *untested* problems. Every one of them is catchable with basic automation.

## The Vision

**Make TUI testing so easy that it becomes the default, not the exception.** If we succeed, the baseline quality of terminal applications rises across the entire ecosystem — from small CLI tools to complex TUI frameworks.

## Goals

1. **Enable fully autonomous agent-driven TUI testing** — Claude writes code, runs the TUI, observes the screen, diagnoses issues, and iterates, with zero human intervention. The agent never gets stuck because it can't see the screen or can't tell if the app crashed.

2. **Make the first test trivially easy** — a developer with an existing TUI app should go from zero to a passing test in under a minute. `agent-terminal init` detects the framework and generates a starter test. `agent-terminal doctor` validates the environment. No docs required.

3. **Catch the bugs nobody catches today** — `agent-terminal test-matrix` runs the same test across terminal sizes, TERM values, and color modes in one command. `agent-terminal a11y-check` audits NO_COLOR compliance, TERM=dumb fallback, resize handling, and contrast. These are the bugs that slip through because manual testing can't cover the matrix.

4. **Be the testing standard for TUI frameworks** — ratatui, bubbletea, textual, ink, and cursive maintainers should recommend agent-terminal in their docs and run it in their CI. The a11y-check and matrix testing features are specifically designed to serve framework maintainers.

5. **Performance as a first-class metric** — TUI responsiveness is critical to user experience but currently unmeasurable. FPS and input latency measurement let developers set performance budgets and catch regressions automatically.

## Success Criteria

- A ratatui/bubbletea/textual app can be tested end-to-end with agent-terminal in CI
- Claude Code can autonomously build, test, and iterate on a TUI app using agent-terminal as feedback
- `test-matrix` catches resize/TERM/color bugs that would otherwise ship to users
- At least one major TUI framework recommends agent-terminal in their testing docs

## Who This Is For

| User | Primary use case |
|---|---|
| **Claude Code** (agentic loop) | Autonomous TUI development: write → build → test → observe → fix → repeat |
| **TUI app developers** | E2E testing in CI, catch resize/color/accessibility bugs |
| **TUI framework maintainers** | Validate framework behavior across terminal configurations |
| **Anyone building CLI tools** | Test interactive prompts, progress bars, table output |

## Status Legend

| Mark | Meaning |
|---|---|
| `[ ]` | Not started |
| `[~]` | Partially implemented |
| `[x]` | Implemented |
| `[T]` | Implemented + tested |

---

## 1. Core Loop

```
open → snapshot → interact → wait → snapshot → assert → cleanup
```

---

## 2. Architecture

```
agent-terminal/
├── SKILL.md                     # Claude Code skill definition
├── references/
│   ├── commands.md              # Full CLI reference
│   ├── snapshot-format.md       # How to read snapshot output
│   ├── session-management.md    # Parallel sessions, isolation
│   └── patterns.md              # Common TUI testing patterns
├── templates/
│   ├── basic-test.sh            # Open → interact → assert → close
│   ├── curses-app.sh            # ncurses/curses-style app testing
│   └── repl-test.sh             # REPL/readline app testing
├── src/
│   ├── main.rs                  # CLI entrypoint (or main.ts — see §7)
│   ├── session.rs               # tmux session lifecycle
│   ├── snapshot.rs              # capture-pane + parse into structured output
│   ├── interact.rs              # send-keys, type, resize
│   ├── wait.rs                  # poll-based wait strategies
│   └── annotate.rs              # optional: render text snapshot as PNG with coords
└── tests/
    └── ...                      # e2e tests using the tool on sample TUI apps
```

---

## 3. CLI Command Design

All commands operate on a tmux session. Default session name: `agent-terminal`.

### Lifecycle

```bash
agent-terminal open "htop"                    # launch command in new tmux session
agent-terminal open "vim file.txt" --session s1  # named session
agent-terminal close                          # kill session
agent-terminal close --session s1
agent-terminal list                           # list active sessions
```

**open** creates a detached tmux session, runs the command inside it, and waits for first render (poll until `capture-pane` output is non-empty / stable for N ms).

### Snapshot

```bash
agent-terminal snapshot                       # text dump of current pane (plain text, no escapes)
agent-terminal snapshot --color                # annotate each line with parsed color/style info
agent-terminal snapshot --raw                  # raw byte stream: exact tmux pane content with all escape sequences
agent-terminal snapshot --ansi                 # raw ANSI escapes, but with row-number formatting applied
agent-terminal snapshot --json                 # structured JSON with text + color spans
agent-terminal snapshot --diff                 # diff against last snapshot (highlight what changed)
agent-terminal snapshot --scrollback 100       # include N lines of scrollback above viewport
```

**Output format** (default text mode):
```
[size: 120x40  cursor: 3,12  session: agent-terminal]
─────────────────────────────────────────────
  1│ File  Edit  View  Help
  2│ ──────────────────────
  3│ > item one
  4│   item two
  5│   item three
  6│
  ...
```

Row numbers give Claude a coordinate system for reasoning ("the cursor is on row 3, the selected item is 'item one'"). This mirrors how agent-browser's @refs give Claude handles for interaction — here, the numbered rows + visual structure serve the same purpose.

**Color mode** (`snapshot --color`):
```
[size: 120x40  cursor: 3,12  session: agent-terminal]
─────────────────────────────────────────────
  1│ File  Edit  View  Help              [fg:white bold]
  2│ ──────────────────────              [fg:gray]
  3│ > item one                          [fg:green bold reverse]
  4│   item two                          [fg:default]
  5│   Error: file not found             [fg:red]
  6│   ██████░░░░ 60%                    [fg:blue]
```

Style annotations appear at the end of each line, only when the line's style differs from default. This gives Claude enough information to assert on colors ("error should be red", "selected item should be green+bold") without the noise of raw ANSI escapes.

**JSON mode with color spans** (`snapshot --json`):
```json
{
  "session": "agent-terminal",
  "size": { "cols": 120, "rows": 40 },
  "cursor": { "row": 3, "col": 12 },
  "lines": [
    {
      "row": 1,
      "text": "File  Edit  View  Help",
      "spans": [
        { "start": 0, "end": 22, "fg": "white", "bold": true }
      ]
    },
    {
      "row": 3,
      "text": "> item one",
      "spans": [
        { "start": 0, "end": 10, "fg": "green", "bold": true, "reverse": true }
      ]
    },
    {
      "row": 5,
      "text": "Error: file not found",
      "spans": [
        { "start": 0, "end": 6, "fg": "red", "bold": true },
        { "start": 6, "end": 21, "fg": "red" }
      ]
    }
  ]
}
```

Each line is split into spans where style attributes change. This supports:
- **Named colors**: black, red, green, yellow, blue, magenta, cyan, white, default
- **256-color**: `"fg": "color(178)"` (xterm-256 palette index)
- **True color**: `"fg": "rgb(255,128,0)"` (24-bit)
- **Attributes**: `bold`, `dim`, `italic`, `underline`, `blink`, `reverse`, `strikethrough`
- **Background**: `"bg": "red"` (same formats as fg)

**Implementation**: `tmux capture-pane -e -p` returns raw ANSI escapes. A VT100 state machine (Rust: `vte` + `vt100` crate; TS: `xterm-headless` or `node-pty` + parser) walks the byte stream, tracks the current SGR state, and emits `(text, style)` spans per line. This is a solved problem — terminal emulator libraries already do exactly this.

**Color-aware assertions**:
```bash
agent-terminal assert --color 5 "fg:red"           # row 5 has red foreground
agent-terminal assert --color 3 "bold"              # row 3 is bold
agent-terminal assert --color 3 "fg:green,reverse"  # compound style check
agent-terminal assert --style "Error" "fg:red"      # text "Error" is styled red (any row)
```

**Raw output modes** (`snapshot --raw` and `snapshot --ansi`):

For cases where the parsed/annotated formats lose information — e.g., testing that your app emits correct escape sequences, debugging rendering issues, or piping output to external tools — two raw modes are available:

`--raw` dumps the exact byte stream from `tmux capture-pane -e -p` with zero post-processing. No row numbers, no metadata header, no parsing. What tmux captured is exactly what you get:
```
\033[1;32m> item one\033[0m
\033[0m  item two\033[0m
\033[31mError: file not found\033[0m
```

`--ansi` is a middle ground: preserves the raw ANSI escapes but adds the row-number formatting and metadata header that the other modes use, so Claude can still orient spatially:
```
[size: 120x40  cursor: 3,12  session: agent-terminal]
─────────────────────────────────────────────
  1│ \033[1;37mFile  Edit  View  Help\033[0m
  2│ \033[90m──────────────────────\033[0m
  3│ \033[1;32;7m> item one\033[0m
  5│ \033[31mError: file not found\033[0m
```

Use cases for raw modes:
- **Escape sequence testing**: verify your app emits the correct SGR codes (e.g., does it use true-color `\033[38;2;r;g;bm` or fall back to 256-color?)
- **Piping to external tools**: `agent-terminal snapshot --raw | aha > output.html`
- **Cursor/movement sequence debugging**: raw mode preserves cursor movement (`\033[H`, `\033[2J`) and other non-SGR escapes that the parsed modes discard
- **Byte-level regression testing**: diff raw snapshots across versions to catch unintended rendering changes

### Interaction

```bash
# Raw key sequences
agent-terminal send "j"                       # single key
agent-terminal send "jjj"                     # multiple keys in sequence
agent-terminal send Enter                     # special key name
agent-terminal send C-c                       # ctrl+c
agent-terminal send Escape                    # escape
agent-terminal send Up Down Left Right        # arrow keys
agent-terminal send C-a C-e                   # readline: home, end
agent-terminal send Tab                       # tab completion
agent-terminal send F1                        # function key
agent-terminal send M-x                       # alt+x

# Typing text (literal, no key-name interpretation)
agent-terminal type "hello world"             # types literally, like send-keys -l

# Paste (sends text via tmux paste buffer — handles special chars safely)
agent-terminal paste "multi\nline\ntext"

# Resize terminal
agent-terminal resize 120 40                  # cols rows
```

**Key name mapping**: agent-terminal translates readable names to tmux key names. e.g. `Enter` → `Enter`, `C-c` → `C-c`, `Escape` → `Escape`, `Tab` → `Tab`, `Up` → `Up`. Passthrough to `tmux send-keys` under the hood.

### Waiting

```bash
agent-terminal wait --text "Success"          # poll until text appears on screen
agent-terminal wait --text-gone "Loading..."  # poll until text disappears
agent-terminal wait --stable 500              # wait until screen content unchanged for 500ms
agent-terminal wait --cursor 5,10             # wait until cursor at row,col
agent-terminal wait --regex "v\d+\.\d+"       # regex match
agent-terminal wait 2000                      # hard wait (ms), last resort
```

**Implementation**: poll `capture-pane` every 50ms (configurable), timeout after 10s (configurable). Returns the final snapshot on success, error + last snapshot on timeout. `--stable` works by comparing consecutive captures — if identical for N ms, the screen is considered stable. This is the most important primitive for reliable tests.

### Assertion (optional convenience)

```bash
agent-terminal assert --text "Success"        # exit 0 if present, exit 1 + show snapshot if not
agent-terminal assert --no-text "Error"       # exit 0 if absent
agent-terminal assert --cursor-row 3          # cursor on expected row
agent-terminal assert --row 1 "File  Edit"    # specific row contains text
```

These are thin wrappers over `snapshot` + grep, but they streamline Claude's reasoning by giving a clear pass/fail signal.

### Screenshot (visual PNG capture)

tmux has no image output. agent-terminal renders the ANSI-captured pane content to a PNG using an embedded terminal renderer.

```bash
agent-terminal screenshot                     # save PNG to auto-named file in cwd
agent-terminal screenshot --path ./shot.png   # explicit path
agent-terminal screenshot --annotate          # overlay row/col grid numbers
agent-terminal screenshot --html              # save as HTML instead of PNG (lighter, inspectable)
agent-terminal screenshot --theme dark        # dark background (default)
agent-terminal screenshot --theme light       # light background
```

**Implementation**: `vt100` crate parses ANSI → cell grid, `ab_glyph` + `image` crates rasterize with an embedded monospace font (~300KB compiled in). Single binary, no external dependencies. The `--html` path is simpler (ANSI → HTML with inline CSS) and useful on its own.

Claude can read PNGs directly (multimodal). For visually complex TUIs, a screenshot can convey layout faster than 80x40 text. Also useful as visual evidence in test reports.

---

---

## 4. Implementation & Distribution

**Language**: Rust. Single static binary (~6-8MB), ~3ms startup. Critical because Claude calls it 10-30 times per session.

**Key crates**: `clap` (CLI), `vt100` (ANSI parsing), `vte` (escape sequence parser), `image` + `ab_glyph` (screenshot rendering), `serde_json` (JSON output), `nix` (signals/process).

**Binary**: ships with an embedded monospace font (~300KB) for screenshot rendering. Only runtime dependency is tmux >= 3.0.

**Compile targets**: `{x86_64,aarch64}-apple-darwin`, `{x86_64,aarch64}-unknown-linux-{gnu,musl}`. No Windows (tmux doesn't run natively).

**Distribution** (same pattern as agent-browser — native binary distributed via npm):
- `npm i -g agent-terminal` — npm package ships prebuilt binaries for all platforms + thin JS shim for platform detection. Postinstall patches symlink to native binary (zero Node overhead).
- `brew install agent-terminal` — pulls from GitHub releases.
- `cargo install agent-terminal` — builds from source.

**Skill**: `npx skills add <owner>/agent-terminal -g -y` installs SKILL.md + references into `~/.claude/skills/agent-terminal/`. Separate from the binary install.

### Development phases

```
Phase 1 — Core implementation (Rust):
  Scaffold Cargo project with clap CLI.
  Write Rust test fixtures (fixture-counter, fixture-echo, etc.).
  Implement + test lifecycle: open, close, list, status.
  Implement + test snapshot (plain text, --color, --raw, --ansi, --json, --diff).
  Implement + test interaction: send, type, paste, click, drag, scroll-wheel.
  Implement + test wait system (text, stable, cursor, regex, text-gone).
  Implement + test process health: status, exit-code, logs.
  Set up CI: cross-compilation (6 targets) + automated test suite.

Phase 2 — Advanced features (Rust):
  Implement + test screenshot rendering (vt100 → image with embedded font).
  Implement + test perf: start/stop, fps --during, latency.
  Implement + test assertions, find, signals, clipboard, resize.
  Implement + test multi-pane support, environment control.

Phase 3 — Adoption & differentiation (Rust):
  Implement + test doctor (env validation).
  Implement + test init (framework detection + scaffolding).
  Implement + test matrix testing (multi-config runner).
  Implement + test a11y-check (accessibility audit).
  Implement error output design (snapshot-in-error across all commands).

Phase 4 — Distribution & skill:
  SKILL.md with worked examples, failure recovery, framework tips.
  References + templates.
  npm package with platform binaries + JS shim (agent-browser pattern).
  Homebrew formula.
  Cargo publish.
  Skills registry entry.
```

Every feature must be `[T]` (implemented + tested) before moving to the next phase. See §7 for the testing strategy.

---

## 5. Example: End-to-End Test of a TUI App

Testing a `todo-tui` app that has a list view, add dialog, and delete confirmation:

```bash
# Launch
agent-terminal open "./todo-tui"
agent-terminal wait --stable 500

# Verify initial state
agent-terminal snapshot
# Output:
# [80x24 cursor:1,0 session:agent-terminal]
#   1│ TODO List (0 items)
#   2│ ──────────────────
#   3│   No items yet. Press 'a' to add.
#   4│
#   5│ [a]dd  [d]elete  [q]uit

# Add an item
agent-terminal send "a"
agent-terminal wait --text "New item:"
agent-terminal snapshot
# Output:
# [80x24 cursor:3,11 session:agent-terminal]
#   1│ TODO List (0 items)
#   2│ ──────────────────
#   3│ New item: _
#   4│
#   5│ [Enter] save  [Esc] cancel

agent-terminal type "Buy groceries"
agent-terminal send Enter
agent-terminal wait --text "1 items"

# Verify item was added
agent-terminal snapshot
# Output:
# [80x24 cursor:3,0 session:agent-terminal]
#   1│ TODO List (1 items)
#   2│ ──────────────────
#   3│ > [ ] Buy groceries
#   4│
#   5│ [a]dd  [d]elete  [q]uit

# Assert
agent-terminal assert --text "Buy groceries"
agent-terminal assert --row 1 "1 items"

# Cleanup
agent-terminal send "q"
agent-terminal close
```

Claude would run this sequence, reading each snapshot to decide the next action — exactly like agent-browser's snapshot→interact loop.

---

## 6. Extended Features

Each feature below maps to a specific goal. Features are tagged with the goal they serve:

- **(G1)** Autonomous agent loop — Claude never gets stuck
- **(G2)** First test under a minute — zero friction onboarding
- **(G3)** Catch untested bugs — matrix/a11y coverage
- **(G4)** Framework standard — features that serve library maintainers
- **(G5)** Performance as metric — FPS and latency measurement

### 6.1 Process Health & Lifecycle (G1 — without this the agent loop breaks)

The biggest gap in a naive tmux wrapper: **Claude can't tell if the app crashed.** It sends keys into a dead session and reads a stale snapshot forever.

```bash
agent-terminal status                         # is the process alive, exited, or crashed?
agent-terminal status --json                  # structured: { alive: bool, pid, exit_code, signal, runtime_ms }
agent-terminal exit-code                      # get exit code (blocks until process exits, or returns null if alive)
agent-terminal logs                           # capture stderr/stdout written before the TUI took over the screen
agent-terminal logs --stderr                  # stderr only (where panics, stack traces, and log lines go)
```

**Implementation**: `tmux` runs the command inside a shell. We wrap the target command:
```bash
tmux new-session -d -s <session> "(<command>) 2>/tmp/agent-terminal-<session>-stderr; echo $? > /tmp/agent-terminal-<session>-exit"
```
This captures stderr to a side-channel and records the exit code on termination. `status` checks if the tmux pane's PID is alive. `logs --stderr` reads the side-channel file.

**Why this is MVP**: without process health, Claude's agentic loop degrades to:
1. Send keys → snapshot shows nothing changed → send keys again → still nothing → retry 5 times → give up and ask user.
With process health: send keys → `status` returns `{ alive: false, exit_code: 1 }` → read `logs --stderr` → see `panic at src/main.rs:42` → fix the bug → restart. Fully autonomous.

### 6.2 Scroll & Viewport (G1)

Many TUIs have scrollable content that extends beyond the visible viewport. Claude needs to access off-screen content.

```bash
# App-level scrolling (sends keys the app interprets)
agent-terminal send PgUp                      # page up
agent-terminal send PgDn                      # page down
agent-terminal send Home                      # scroll to top (app-dependent)
agent-terminal send End                       # scroll to bottom (app-dependent)

# tmux-level scrollback (reads tmux's buffer, not the app's)
agent-terminal scrollback                     # dump full tmux scrollback buffer
agent-terminal scrollback --lines 200         # last 200 lines
agent-terminal scrollback --search "error"    # search scrollback, return matching lines with context
```

**Distinction**: app-level scroll (PgUp/PgDn) moves the viewport within the app — the app redraws. tmux scrollback captures output that has scrolled off the tmux pane (e.g., build output, log lines printed before the TUI started). Both are needed.

### 6.3 Mouse Support (G1, G3)

Modern TUI frameworks (bubbletea, tui-rs/ratatui, textual, blessed) support mouse interaction. tmux can forward mouse events to apps.

```bash
agent-terminal click <row> <col>              # left click at row, col (1-indexed)
agent-terminal click <row> <col> --right      # right click
agent-terminal click <row> <col> --double     # double click
agent-terminal drag <r1> <c1> <r2> <c2>       # click-drag from (r1,c1) to (r2,c2)
agent-terminal scroll-wheel up <row> <col>    # scroll wheel up at position
agent-terminal scroll-wheel down <row> <col>  # scroll wheel down at position
```

**Implementation**: tmux supports mouse events via escape sequences sent to the pane. The `send-keys` command accepts mouse escape sequences directly:
```bash
# Example: click at row 5, col 10 (translated to SGR mouse encoding)
tmux send-keys -t <session> -l $'\033[<0;10;5M\033[<0;10;5m'
```
agent-terminal handles the encoding — Claude just says `click 5 10`.

**Why MVP**: testing a TUI without mouse is like testing a web app without clicking. Many modern TUIs are mouse-first (especially Python Textual apps, lazygit, etc.).

### 6.4 Signals (G1, G3)

Testing signal handling is essential — apps should handle SIGINT, SIGTERM, SIGWINCH gracefully.

```bash
agent-terminal signal SIGINT                  # ctrl+c equivalent, but at process level
agent-terminal signal SIGTERM                 # graceful shutdown request
agent-terminal signal SIGWINCH                # window resize signal (pair with `resize`)
agent-terminal signal SIGTSTP                 # ctrl+z (suspend)
agent-terminal signal SIGCONT                 # resume from suspend
agent-terminal signal SIGHUP                  # terminal hangup
```

**Implementation**: `tmux send-keys C-c` sends a ctrl+c character; `signal SIGINT` sends the actual signal via `kill -s SIGINT <pid>`. These are different — some apps handle the keystroke but not the signal, or vice versa. Both need testing.

### 6.5 Environment & Terminal Capability Control (G3, G4)

TUI apps behave differently based on terminal capabilities. Claude needs to control this to test compatibility.

```bash
agent-terminal open "./my-app" --env TERM=xterm-256color   # default
agent-terminal open "./my-app" --env TERM=dumb             # test graceful degradation
agent-terminal open "./my-app" --env TERM=screen           # tmux's native TERM
agent-terminal open "./my-app" --env NO_COLOR=1            # test NO_COLOR standard compliance
agent-terminal open "./my-app" --env COLORTERM=truecolor   # advertise 24-bit color support
agent-terminal open "./my-app" --env COLUMNS=40 --env LINES=10  # small viewport test
agent-terminal open "./my-app" --env LC_ALL=en_US.UTF-8    # unicode support
agent-terminal open "./my-app" --env LC_ALL=C              # ASCII-only fallback test
agent-terminal open "./my-app" --size 40x10                # set initial tmux pane size (shorthand)
```

**Why MVP**: a common agentic loop failure is Claude building a TUI that looks fine in 256-color but crashes or looks broken when `TERM=dumb` or `NO_COLOR=1`. Environment control lets Claude test these variants automatically.

### 6.6 Multi-Pane / Multi-Process (G1)

Many apps need a companion process — a server and a client, a watcher and an editor, a database and an app. tmux's pane system handles this natively.

```bash
agent-terminal open "npm run dev" --session myapp --pane server
agent-terminal open "npm run test:e2e" --session myapp --pane tests
agent-terminal snapshot --pane server         # read server output
agent-terminal snapshot --pane tests          # read test output
agent-terminal send --pane server "q"         # interact with specific pane
agent-terminal status --pane server           # check if server is still running
```

**Implementation**: maps to tmux panes within a session. `--pane` creates or targets a named pane (via tmux `split-window` + `select-pane -T`).

### 6.7 Resize & Responsive Testing (G3)

TUI apps should handle terminal resize gracefully. tmux makes this easy to test.

```bash
agent-terminal resize 40 10                   # resize to 40 cols × 10 rows
agent-terminal wait --stable 300              # wait for app to re-render
agent-terminal snapshot                       # verify layout adapted
agent-terminal resize 200 50                  # test wide terminal
agent-terminal resize 80 24                   # back to standard
```

Already in the plan, but promoting to explicit MVP because responsive layout bugs are extremely common in TUIs and a frequent source of "it works on my machine" issues.

### 6.8 Search & Find (G1)

Instead of Claude parsing snapshots manually every time, give it a targeted search:

```bash
agent-terminal find "Error"                   # return row,col of first match + surrounding context
agent-terminal find "Error" --all             # all matches with positions
agent-terminal find --regex "v\d+\.\d+"       # regex search
agent-terminal find --color "fg:red"          # find all red-colored text (regardless of content)
```

This is a token-saver: instead of snapshot (200 tokens) → Claude parses the grid → finds the error, it's one command that returns a focused result (~30 tokens).

### 6.9 Clipboard (G1)

```bash
agent-terminal clipboard read                 # read tmux paste buffer
agent-terminal clipboard write "text"         # set tmux paste buffer
agent-terminal clipboard paste                # paste from buffer into pane
```

Low-hanging fruit — all three commands map directly to existing tmux primitives (`tmux show-buffer`, `tmux set-buffer`, `tmux paste-buffer`). Useful for testing TUI copy/paste features (e.g., yank in vim-style apps, Ctrl+C/V in textual apps) and for injecting large text blocks more reliably than `type` (paste buffer avoids keystroke-level timing issues).

### 6.10 Record & Replay (Post-MVP)

```bash
agent-terminal record start                   # start recording all commands + snapshots
agent-terminal record stop --path test.json   # save recording
agent-terminal replay test.json               # replay and compare snapshots (regression test)
agent-terminal replay test.json --update      # replay and update expected snapshots
```

Enables snapshot-based regression testing. Claude records a passing session, then replays it after code changes to verify nothing broke. Lower priority because Claude can just re-run the test sequence.

### 6.11 Performance Metrics — FPS & Input Latency (G5)

Two core metrics that cover the majority of TUI performance concerns. The critical design question is **how Claude composes these with other commands**, since Claude Code calls `Bash` one tool invocation at a time.

#### Interaction Model: Three Ways to Measure

**1. Start/stop mode** — best fit for Claude Code's sequential tool calls:
```bash
# Claude calls each as a separate Bash tool invocation:
agent-terminal perf start                     # begin recording frames in background
agent-terminal send "j"                       # Claude's normal interaction...
agent-terminal send "j"                       # ...continues as usual...
agent-terminal send "G"                       # ...agent-terminal is measuring the whole time
agent-terminal perf stop                      # stop recording, return metrics
# → { "fps": 24.5, "frame_count": 73, "duration_ms": 3000, ... }
```

This is the **primary mode**. `perf start` spawns a background poller that records every frame change. Claude continues issuing commands normally across separate tool calls. `perf stop` kills the poller and returns aggregated metrics. No shell backgrounding, no `&`, no script composition.

**2. Inline mode** — single Bash call with a batch of actions:
```bash
# When Claude wants to measure a specific action sequence in one shot:
agent-terminal perf fps --during 'send "j" && send "j" && send "G"'
# → runs the commands while measuring, returns metrics when done

# Or with batch JSON for more complex sequences:
echo '[
  {"cmd": "send", "args": ["j"]},
  {"cmd": "wait", "args": ["--stable", "100"]},
  {"cmd": "send", "args": ["G"]}
]' | agent-terminal perf fps --during-batch
```

`--during` takes a command string (or `--during-batch` takes JSON on stdin), executes it while measuring FPS, and returns metrics. One Bash tool call, one result. Best for scripted sequences where Claude already knows what to do.

**3. Self-contained latency probe** — no composition needed:
```bash
agent-terminal perf latency --key "j" --samples 10
# → internally: snapshot → send "j" → poll until change → repeat 10x → return stats
```

`perf latency` is inherently self-contained: it sends the key and measures the response internally. No concurrency needed. Claude just calls it and reads the result.

#### Frames Per Second

How often is the screen content actually changing?

```bash
# Start/stop (recommended for Claude Code):
agent-terminal perf start                     # begin frame recording
# ... interact with the app across multiple tool calls ...
agent-terminal perf stop                      # get results
agent-terminal perf stop --json               # structured output

# Inline (single tool call):
agent-terminal perf fps --during 'send "G"'   # measure during a specific action
agent-terminal perf fps --duration 3000       # passive: just observe for N ms, no actions
```

**Implementation**: poll `tmux capture-pane -p` at high frequency (every 10ms) and count how many times the output changes per second. Each change = one "frame."

FPS interpretation:
- **0 FPS** → app is frozen / hung
- **1-5 FPS** → sluggish, likely blocking on something
- **10-30 FPS** → normal for most TUIs (terminal refresh is typically 30-60 Hz)
- **Sudden FPS drop** during scrolling or input → performance regression

**`perf stop` output**:
```json
{
  "fps": 24.5,
  "frame_count": 73,
  "duration_ms": 2980,
  "min_frame_ms": 33,
  "max_frame_ms": 120,
  "mean_frame_ms": 40,
  "p95_frame_ms": 88,
  "idle_ms": 450,
  "timeline": [
    { "t_ms": 0, "frame_ms": 33 },
    { "t_ms": 33, "frame_ms": 35 },
    { "t_ms": 68, "frame_ms": 120 }
  ]
}
```

The `timeline` array lets Claude spot *when* frame drops happened — correlating with which keystroke caused the spike.

#### Input Latency

How fast does the app respond to a keystroke? Self-contained — no composition needed.

```bash
agent-terminal perf latency                   # default probe: space → backspace (neutral)
agent-terminal perf latency --key "j"         # measure latency for specific key
agent-terminal perf latency --key "j" --samples 10   # repeat N times, report stats
agent-terminal perf latency --json            # structured output
```

**Implementation**: snapshot → `tmux send-keys` → poll `capture-pane` at 1ms intervals until output changes → report delta. The default probe sends space then backspace to avoid side effects.

```json
{ "mean_ms": 18, "min_ms": 8, "max_ms": 45, "p95_ms": 38, "samples": 10 }
```

Latency thresholds for TUI apps:
- **< 16ms** → imperceptible, excellent
- **16-50ms** → responsive, good
- **50-100ms** → noticeable lag
- **100-200ms** → sluggish, needs investigation
- **> 200ms** → broken, likely blocking the render loop

#### How Claude Uses This in the Agentic Loop

```
Typical Claude Code interaction (each line = one Bash tool call):

  Tool call 1:  agent-terminal open "./my-app" && agent-terminal wait --stable 500
  Tool call 2:  agent-terminal perf start
  Tool call 3:  agent-terminal send "j"
  Tool call 4:  agent-terminal send "j"
  Tool call 5:  agent-terminal send "G"
  Tool call 6:  agent-terminal perf stop --json
                → { "fps": 22.1, "p95_frame_ms": 88 }
  Tool call 7:  agent-terminal perf latency --key "j" --samples 5
                → { "mean_ms": 35, "p95_ms": 42 }

  Claude: "FPS and latency look healthy."

  ... Claude makes a code change, rebuilds ...

  Tool call 8:  agent-terminal close
  Tool call 9:  agent-terminal open "./my-app" && agent-terminal wait --stable 500
  Tool call 10: agent-terminal perf latency --key "j" --samples 5
                → { "mean_ms": 180, "p95_ms": 210 }

  Claude: "Latency regressed from 35ms to 180ms after my change. Investigating."
```

The start/stop pattern means Claude doesn't need to compose bash scripts or manage background processes. It fits naturally into sequential tool calls — which is exactly how Claude Code works.

**Why MVP**: these two metrics turn performance from subjective ("feels slow") into an objective signal that Claude can act on autonomously. If `perf latency` returns 180ms after a code change, Claude knows it introduced a regression and can bisect. Without explicit measurement, Claude would only catch severe freezes (via `wait --stable` timing out) and miss gradual degradation.

### 6.12 Doctor — Environment Validation (G2)

First-run experience matters. If tmux is the wrong version or missing, the user shouldn't discover this 20 minutes into writing their first test.

```bash
agent-terminal doctor
```

Output:
```
agent-terminal doctor
  ✓ tmux installed: 3.4 (>= 3.0 required)
  ✓ tmux server: can create/destroy sessions
  ✓ capture-pane: text capture works
  ✓ capture-pane -e: ANSI escape capture works
  ✓ mouse support: SGR mouse encoding supported
  ✓ pane resize: resize-pane works
  ✓ send-keys: keystroke delivery works
  ✓ paste-buffer: clipboard primitives work

  All checks passed. agent-terminal is ready to use.
```

On failure:
```
agent-terminal doctor
  ✓ tmux installed: 2.8
  ✗ capture-pane -e: ANSI escape capture requires tmux >= 3.0
    → Run: brew install tmux (or apt-get install tmux)
  ✗ mouse support: SGR mouse encoding not available in tmux 2.8
    → Upgrade tmux to >= 3.0

  2 checks failed. See above for fix instructions.
```

Every failure includes a concrete fix command. This is table-stakes for adoption — Playwright has `npx playwright install`, Rust has `rustup doctor`.

### 6.13 Init — Project Scaffolding (G2)

Lower the barrier from "read the docs" to "run one command":

```bash
agent-terminal init
```

Detects the TUI framework from project files and generates a starter test:

```
agent-terminal init
  Detected: ratatui (from Cargo.toml dependency)
  Created:  tests/tui/basic_test.sh
  Created:  tests/tui/README.md

  Run your first test:
    agent-terminal open "cargo run" && agent-terminal wait --stable 500 && agent-terminal snapshot
```

Framework detection:
| File | Dependency | Framework |
|---|---|---|
| `Cargo.toml` | `ratatui`, `crossterm`, `cursive` | Rust TUI |
| `go.mod` | `bubbletea`, `tview`, `termui` | Go TUI |
| `package.json` | `ink`, `blessed`, `terminal-kit` | Node TUI |
| `requirements.txt` / `pyproject.toml` | `textual`, `rich`, `curses` | Python TUI |

The generated test includes framework-specific tips: e.g., for bubbletea apps, the test waits for the initial `tea.Program` render; for textual apps, it accounts for the CSS-based styling warmup.

### 6.14 Matrix Testing (G3, G4)

The killer feature. Test a TUI across multiple configurations in one command:

```bash
agent-terminal test-matrix \
  --command "./my-app" \
  --sizes "80x24,120x40,40x10" \
  --terms "xterm-256color,screen-256color,dumb" \
  --colors "default,NO_COLOR=1,COLORTERM=truecolor" \
  --test 'wait --stable 500 && assert --text "Ready" && snapshot'
```

Output:
```
Matrix: 3 sizes × 3 terms × 3 colors = 27 combinations

                     xterm-256color  screen-256color  dumb
  80x24  default     ✓               ✓                ✓
  80x24  NO_COLOR    ✓               ✓                ✓
  80x24  truecolor   ✓               ✓                ✗ crash
  120x40 default     ✓               ✓                ✓
  120x40 NO_COLOR    ✓               ✓                ✓
  120x40 truecolor   ✓               ✓                ✗ crash
  40x10  default     ✗ layout        ✗ layout         ✗ crash
  40x10  NO_COLOR    ✗ layout        ✗ layout         ✗ crash
  40x10  truecolor   ✗ layout        ✗ layout         ✗ crash

  18/27 passed, 9 failed
  Failures saved to: ./agent-terminal-matrix/
    dumb+truecolor/snapshot.txt    ← crash: "COLORTERM set but TERM=dumb"
    40x10/snapshot.txt             ← layout: "Ready" text not found (too small?)
```

Each failure includes the snapshot at the point of failure. Claude can read the matrix output and immediately know: "the app crashes when TERM=dumb + COLORTERM=truecolor, and the layout breaks below 80 cols."

**Implementation**: spawns N tmux sessions in parallel (one per combination), runs the test in each, collects results. Fast because tmux sessions are lightweight (~50ms to create).

### 6.15 Accessibility Check (G3, G4)

Automated accessibility audit for TUI apps — something nobody offers today:

```bash
agent-terminal a11y-check "./my-app"
```

Checks:
| Check | What it tests | How |
|---|---|---|
| **NO_COLOR** | App respects [no-color.org](https://no-color.org) standard | Run with `NO_COLOR=1`, verify no ANSI color codes in `--raw` output |
| **TERM=dumb** | Graceful degradation without terminal capabilities | Run with `TERM=dumb`, verify app starts without crash |
| **Resize** | Handles terminal resize without crash | Start at 80x24, resize to 40x10, check process still alive |
| **Focus visible** | Selected/focused items are visually distinct | Compare snapshot of focused vs unfocused item, verify style difference |
| **Contrast** | Text is readable (not dim-on-dark, etc.) | Parse color spans, flag low-contrast fg/bg combinations |
| **Keyboard-only** | All actions reachable without mouse | Implicit — if the app works via `send` commands, it's keyboard accessible |

Output:
```
agent-terminal a11y-check ./my-app

  ✓ NO_COLOR: app renders without ANSI color codes when NO_COLOR=1
  ✓ TERM=dumb: app starts and renders without crash
  ✗ Resize: app crashes when resized to 40x10
    → snapshot at crash: [saved to a11y-report/resize-crash.txt]
  ✓ Focus visible: selected item has distinct style (reverse video)
  ⚠ Contrast: dim text on default background (row 12) may be hard to read

  3/5 passed, 1 failed, 1 warning
```

This alone could drive adoption — TUI library maintainers (ratatui, bubbletea, textual) could add `agent-terminal a11y-check` to their CI and recommend it in their docs.

### 6.16 Error Output Design (G1 — applies to all commands)

Every error from agent-terminal must be Claude-actionable. When a command fails, the output includes:

```
ERROR: wait --text "Ready" timed out after 10000ms

Session: my-app (alive, pid 12345, runtime 11.2s)
Last snapshot:
  1│ Loading...
  2│ Connecting to database...
  3│ Error: connection refused (localhost:5432)
  4│
Hint: The app appears to be stuck on startup. Check if the database is running.
```

Structure:
1. **What failed** — the command and why (timeout, session not found, process dead)
2. **Session state** — alive/dead, PID, runtime, exit code if dead
3. **Last snapshot** — what the screen actually shows (this is the key debugging signal)
4. **Hint** — pattern-matched suggestion (e.g., "connection refused" → "check if the service is running")

For Claude, the last snapshot in the error output often contains the answer. Without it, Claude would need an extra `snapshot` call to figure out what went wrong — wasting a tool call cycle.

### 6.17 SKILL.md Quality (G1 — the make-or-break)

The SKILL.md prompt determines whether Claude uses agent-terminal effectively or flails. It needs more than a command reference — it needs worked patterns and failure recovery:

**Required sections in SKILL.md**:

1. **Core loop** — always: snapshot → reason → act → wait → snapshot. Never fire-and-forget.
2. **Failure recovery flowchart**:
   ```
   snapshot shows nothing useful?
     → check `status` — is the process alive?
       → dead? → read `logs --stderr` for crash reason
       → alive? → try `wait --stable 1000` — maybe it's still loading
   ```
3. **Framework-specific tips**:
   - bubbletea: initial render may take 100-200ms, use `wait --stable 300`
   - textual: CSS styling warmup can delay first paint, use `wait --text` for a known element
   - ratatui: if using crossterm backend, `TERM=xterm-256color` is required
4. **Common mistakes**:
   - Don't `send` multiple keys without `wait` between them (renders may overlap)
   - Don't `assert` immediately after `send` — always `wait` first
   - Don't forget to `close` — leaked sessions accumulate
5. **Performance testing pattern**: perf start → interact → perf stop (not bash backgrounding)
6. **Matrix testing pattern**: when to use `test-matrix` vs manual env control
7. **Scrolling CLI vs full-screen TUI** — the same tool handles both, but the right primitives differ:
   - **Scrolling CLI output** (build tools, log streams, REPLs): prefer `scrollback --search "error"` to find content that scrolled past, `wait --text` for expected output, `logs --stderr` for crash diagnostics. The viewport only shows the last N rows — important output may have scrolled off.
   - **Full-screen TUI** (ratatui, bubbletea, textual, ncurses): prefer `snapshot` for layout verification, `assert --row` for positional checks, `find` for locating elements, `resize` for responsive testing. The screen buffer is the complete picture — nothing scrolls off.
   - Key difference: with scrolling output, always check `scrollback` before concluding something isn't there. With full-screen TUIs, `snapshot` is authoritative.

### 6.18 CI Integration (Post-MVP)

tmux works without a real TTY, which means agent-terminal works in CI natively. The only additions needed:
- `--timeout <ms>` global flag on all commands (CI must not hang)
- JUnit XML output for assertions: `agent-terminal assert --junit results.xml ...`
- Non-interactive mode: error on any command that would block for input

---

## Implementation Status

**Legend**: `[ ]` Not started · `[x]` Implemented (code exists, not tested) · `[T]` Implemented + tested · `[~]` Partially implemented

### MVP Features

| Status | Feature | Goal | Subcommands | Test Status | Notes |
|---|---|---|---|---|---|
| `[T]` | **Lifecycle** | G1 | `open`, `close`, `list` | `[T]` 6 tests | open w/ env, size; close; list; status json |
| `[T]` | **Snapshot (plain text)** | G1 | `snapshot` | `[T]` | Row numbers, metadata header, trailing-line trim |
| `[T]` | **Snapshot (color)** | G1, G3 | `snapshot --color` | `[T]` | Style annotations `[fg:red bold]` verified |
| `[T]` | **Snapshot (raw)** | G1 | `snapshot --raw`, `--ansi` | `[T]` | Raw ANSI passthrough + ANSI-with-row-numbers |
| `[T]` | **Snapshot (JSON)** | G1 | `snapshot --json` | `[T]` 2 tests | Structure + color spans verified |
| `[T]` | **Snapshot (diff)** | G1 | `snapshot --diff` | `[T]` | +/- markers, baseline storage in /tmp |
| `[T]` | **Snapshot (scrollback)** | G1 | `snapshot --scrollback` | `[T]` | Tested in snapshot_test.rs |
| `[T]` | **Keyboard interaction** | G1 | `send`, `type`, `paste` | `[T]` 6 tests | Key mapping passes through to tmux natively (Enter, Up, C-c etc. work) |
| `[T]` | **Wait** | G1 | `--text`, `--text-gone`, `--stable`, `--regex`, `<ms>` | `[T]` 6 tests | All with timeout/error output |
| `[T]` | **Wait (cursor)** | G1 | `--cursor row,col` | `[T]` | Tested in wait_test.rs |
| `[T]` | **Assert (basic)** | G1, G3 | `--text`, `--no-text`, `--row` | `[T]` 5 tests | Pass/fail with snapshot on failure |
| `[T]` | **Assert (color/style)** | G3 | `--color`, `--style`, `--cursor-row` | `[T]` | cursor-row, color style, style text all tested |
| `[T]` | **Screenshot (HTML)** | G1 | `screenshot --html` | `[T]` | Full ANSI color → inline CSS |
| `[T]` | **Screenshot (PNG)** | G1 | `screenshot --path` | `[T]` | Basic bitmap renderer (not full glyph raster) |
| `[T]` | **Screenshot (annotate/theme)** | G1 | `--annotate`, `--theme` | `[T]` | HTML annotate, PNG annotate, light theme tested |
| `[T]` | **Process health** | G1 | `status`, `status --json`, `exit-code` | `[T]` 5 tests | Crash detection, exit code, alive/dead |
| `[T]` | **Logs** | G1 | `logs`, `--stderr` | `[T]` | stderr capture via temp file |
| `[T]` | **Scrollback** | G1 | `scrollback`, `--lines`, `--search` | `[T]` 2 tests | Buffer capture + text search |
| `[T]` | **Mouse (click)** | G1, G3 | `click`, `--right`, `--double` | `[T]` 5 tests | Left, right, double click, coordinates verified |
| `[T]` | **Mouse (drag/scroll)** | G1, G3 | `drag`, `scroll-wheel` | `[T]` 4 tests | Drag, scroll up/down tested |
| `[T]` | **Signals** | G1, G3 | `signal SIGTERM/SIGKILL/...` | `[T]` 2 tests | Real signal via nix::sys::signal::kill |
| `[T]` | **Environment control** | G3, G4 | `open --env KEY=VAL`, `open --size COLSxROWS` | `[T]` | --size tested; --env tested in lifecycle; TERM=dumb in env_test |
| `[T]` | **Multi-pane** | G1 | `open --pane`, `snapshot --pane`, `send --pane` | `[T]` 9 tests | Open, snapshot, send, type, status per-pane, independence |
| `[T]` | **Resize** | G3 | `resize <cols> <rows>` | `[T]` 2 tests | Pane + window resize, verified in snapshot |
| `[T]` | **Find (basic)** | G1 | `find`, `--all`, `--regex` | `[T]` 4 tests | Text search, all matches, regex |
| `[T]` | **Find (color)** | G1 | `find --color` | `[T]` | Find by color style tested |
| `[T]` | **Clipboard** | G1 | `clipboard read/write/paste` | `[T]` 2 tests | All three operations |
| `[T]` | **Perf: FPS (start/stop)** | G5 | `perf start`, `perf stop` | `[T]` | Background poller + metric aggregation |
| `[T]` | **Perf: FPS (duration)** | G5 | `perf fps --duration` | `[T]` | Passive observation mode |
| `[T]` | **Perf: FPS (during)** | G5 | `perf fps --during`, `--during-batch` | `[T]` | --during tested |
| `[T]` | **Perf: latency** | G5 | `perf latency --key`, `--samples` | `[T]` | Mean/min/max/p95 with JSON output |
| `[T]` | **Doctor** | G2 | `doctor` | `[T]` | All 8 checks with ✓/✗ output |
| `[T]` | **Init** | G2 | `init` | `[T]` 4 tests | Detects ratatui, bubbletea, textual, no-framework |
| `[T]` | **Error output** | G1 | (all commands) | `[T]` | rich_error() in session.rs; wait timeout shows snapshot; assert failure shows snapshot |
| `[T]` | **Matrix testing** | G3, G4 | `test-matrix --sizes --terms --colors` | `[T]` | All-pass, failure, multi-term scenarios tested |
| `[T]` | **Accessibility check** | G3, G4 | `a11y-check` | `[T]` 3 tests | Runs checks, resize survives, TERM=dumb survives |

### Post-MVP Features

| Status | Feature | Subcommands | Test Status | Notes |
|---|---|---|---|---|
| `[ ]` | **Batch** | `batch --json` | `[ ]` | Not implemented. Commands are ~3ms, marginal savings |
| `[ ]` | **Record & replay** | `record start/stop`, `replay` | `[ ]` | Not implemented |
| `[ ]` | **CI / JUnit** | `assert --junit`, `--timeout` global | `[ ]` | Not implemented |

### Infrastructure

| Status | Item | Test Status | Notes |
|---|---|---|---|
| `[T]` | Cargo project scaffold + clap CLI | `[T]` | 26 subcommands, workspace with fixtures |
| `[x]` | CI: cross-compilation (6 targets) | n/a | `.github/workflows/release.yml` — not run yet (needs first tag push) |
| `[x]` | CI: automated test suite | n/a | `.github/workflows/ci.yml` — not run yet (needs GitHub push) |
| `[x]` | npm package + JS platform shim | n/a | `package.json`, `bin/agent-terminal.js`, `scripts/postinstall.js` — not published |
| `[ ]` | Homebrew formula | `[ ]` | Not started |
| `[ ]` | Cargo publish | `[ ]` | Not started |
| `[T]` | SKILL.md + references + templates | `[T]` | SKILL.md w/ frontmatter, 4 references, 3 templates |
| `[ ]` | Skills registry entry | `[ ]` | Not started |

### Test Fixtures

| Status | Fixture | Purpose | Notes |
|---|---|---|---|
| `[T]` | `fixture-echo` | Lifecycle tests | Prints args, waits for 'q' |
| `[T]` | `fixture-counter` | Interaction/wait/assert tests | j/k increment/decrement, green bold number |
| `[T]` | `fixture-color` | Snapshot color parsing | 6 lines with different ANSI styles |
| `[T]` | `fixture-mouse` | Mouse tests | SGR mouse tracking, tested in mouse_test.rs |
| `[T]` | `fixture-crash` | Process health tests | Exits with code 42 after 500ms |
| `[T]` | `fixture-slow` | Perf tests | Auto-updating frame counter every 100ms |
| `[T]` | `fixture-resize` | Resize tests | Shows terminal dimensions, handles SIGWINCH |

### Test Suite Summary

**143 tests across 21 test files, all passing.**

| Test File | Tests | Coverage |
|---|---|---|
| `lifecycle_test.rs` | 6 | open, close, list, env, size, status json |
| `snapshot_test.rs` | 8 | plain, color, raw, ansi, json, json spans, diff, scrollback |
| `interaction_test.rs` | 6 | send single/multi, decrement, type, paste, resize |
| `wait_test.rs` | 9 | text, timeout, text-gone, stable, regex, hard wait, cursor, regex multiple |
| `assert_test.rs` | 17 | text pass/fail, no-text, row, cursor-row, color style, style text |
| `process_health_test.rs` | 5 | alive, json, crash, exit code, stderr |
| `find_test.rs` | 10 | text, not found, all, regex, color style |
| `resize_test.rs` | 2 | change size, restore |
| `clipboard_test.rs` | 2 | write/read, paste |
| `signal_test.rs` | 2 | SIGTERM, SIGKILL |
| `screenshot_test.rs` | 5 | HTML, PNG, annotate HTML, annotate PNG, light theme |
| `doctor_test.rs` | 1 | passes |
| `perf_test.rs` | 4 | latency, fps duration, start/stop, fps --during |
| `env_test.rs` | 2 | size, TERM=dumb |
| `scrollback_test.rs` | 2 | basic, search |
| `mouse_test.rs` | 9 | click left/right/double, coordinates, scroll up/down, drag |
| `multi_pane_test.rs` | 9 | open pane, split, snapshot per-pane, send per-pane, independence |
| `init_test.rs` | 4 | detect ratatui, bubbletea, textual, no framework |
| `matrix_test.rs` | 1 | all-pass, failure, multi-term (combined) |
| `a11y_test.rs` | 3 | runs checks, resize survives, TERM=dumb survives |
| *(unit tests in snapshot.rs)* | 36 | ANSI parsing, style serialization, spans |

### Remaining Gaps

**All previously untested features now have test coverage.** No `[x]` items remain.

**Architecture debt:**
- ANSI parsing duplicated in `snapshot.rs` and `wait.rs` — `snapshot.rs` is canonical
- PNG screenshot uses basic bitmap blocks, not real font rasterization via `ab_glyph`

**Not implemented (post-MVP):**
- Batch command (`batch --json`)
- Record & replay (`record start/stop`, `replay`)
- CI / JUnit output (`assert --junit`, global `--timeout`)
- Homebrew formula
- Cargo publish
- Skills registry entry

---

## 7. Automated Testing Strategy

agent-terminal is a CLI that drives tmux — we can test it by driving *itself* against known TUI fixtures. Tests run in CI (GitHub Actions) on Linux with tmux installed.

### Test architecture

Everything is Rust — fixtures, harness, assertions.

```
tests/
├── fixtures/
│   └── src/bin/                 # Minimal TUI apps, compiled as Cargo binaries
│       ├── echo_app.rs          # Prints args, exits (lifecycle tests)
│       ├── counter_app.rs       # j/k increment/decrement, shows count (interaction tests)
│       ├── color_app.rs         # Renders colored output (snapshot --color tests)
│       ├── mouse_app.rs         # Prints click coordinates (mouse tests)
│       ├── crash_app.rs         # Exits with code 1 after 500ms (process health tests)
│       ├── slow_app.rs          # Intentionally slow render loop (perf tests)
│       └── resize_app.rs        # Prints terminal dimensions on resize (resize tests)
├── integration/
│   ├── lifecycle_test.rs        # open → status → close → status
│   ├── snapshot_test.rs         # plain, --color, --raw, --ansi, --json, --diff
│   ├── interaction_test.rs      # send, type, paste → verify via snapshot
│   ├── wait_test.rs             # --text, --stable, --text-gone, --cursor, --regex, timeout
│   ├── assert_test.rs           # --text, --no-text, --row, --color, --style, exit codes
│   ├── screenshot_test.rs       # PNG exists, correct dimensions, --html, --svg
│   ├── process_health_test.rs   # status on crashed app, exit-code, logs --stderr
│   ├── mouse_test.rs            # click → verify coordinates in snapshot
│   ├── signal_test.rs           # send SIGINT → verify graceful shutdown
│   ├── env_test.rs              # open --env TERM=dumb → verify behavior
│   ├── resize_test.rs           # resize → wait → verify new dimensions in snapshot
│   ├── multi_pane_test.rs       # open two panes → snapshot each independently
│   ├── find_test.rs             # find text, --regex, --color
│   ├── clipboard_test.rs        # write → paste → verify in snapshot
│   ├── scrollback_test.rs       # scrollback, --search
│   ├── perf_test.rs             # perf start → interact → perf stop → verify fps > 0
│   ├── latency_test.rs          # perf latency → verify mean_ms is reasonable
│   └── batch_test.rs            # batch JSON → verify combined output
├── common/
│   └── mod.rs                   # Shared test harness: session guard, cmd! macro, helpers
└── unit/
    ├── snapshot_parser_test.rs  # VT100 parsing, ANSI → spans, color extraction
    ├── key_mapping_test.rs      # key name → tmux key translation
    ├── mouse_encoding_test.rs   # row,col → SGR mouse escape sequence
    └── wait_logic_test.rs       # stability detection algorithm, timeout behavior
```

### Why Rust for everything (including fixtures)

Test fixtures are target TUI apps that agent-terminal drives. Using Rust for them:
- **Zero runtime dependencies in CI** — no Python, no bash quirks across distros
- **Compiled alongside the project** — `cargo test` builds fixtures automatically via Cargo workspace
- **Direct access to terminal APIs** — raw mode, ANSI output, signal handling, mouse events via the same `nix`/`libc` crates the main binary uses
- **Type-safe and deterministic** — no accidental Python version mismatches or shell portability bugs
- **Same toolchain** — contributors only need Rust installed, not Rust + Python

The fixtures are small crates in a Cargo workspace. `cargo test` compiles them as binaries before running integration tests. A build script or `#[cfg(test)]` ensures they only build during `cargo test`, not in release.

### Cargo workspace layout

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    ".",                         # agent-terminal main binary
    "tests/fixtures",            # test fixture binaries
]

# tests/fixtures/Cargo.toml
[package]
name = "agent-terminal-fixtures"

[[bin]]
name = "fixture-echo"
path = "src/bin/echo_app.rs"

[[bin]]
name = "fixture-counter"
path = "src/bin/counter_app.rs"

# ... etc
```

### Test fixture design

Each fixture is a minimal, deterministic TUI app. They must be:
- **Self-contained**: no dependencies beyond `libc`/`nix` for raw terminal mode
- **Deterministic**: same input → same output, no randomness or timing-dependent behavior
- **Fast**: start in <5ms (compiled Rust), respond to input in <1ms
- **Minimal**: test one thing — the counter app doesn't need colors, the color app doesn't need input

Example fixture — `counter_app.rs`:
```rust
//! Minimal TUI: j increments, k decrements, q quits. Shows count with color.
use std::io::{self, Read, Write};

fn render(count: i32) {
    let mut out = io::stdout();
    write!(out, "\x1b[2J\x1b[H").unwrap();          // clear + home
    write!(out, "Count: \x1b[1;32m{count}\x1b[0m\n").unwrap(); // green bold
    write!(out, "[j] +1  [k] -1  [q] quit\n").unwrap();
    out.flush().unwrap();
}

fn main() {
    // Set raw mode via libc
    let fd = 0; // stdin
    let mut old = unsafe { std::mem::zeroed() };
    unsafe { libc::tcgetattr(fd, &mut old) };
    let mut raw = old;
    unsafe { libc::cfmakeraw(&mut raw) };
    unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &raw) };

    let mut count = 0i32;
    render(count);

    let mut buf = [0u8; 1];
    while io::stdin().read(&mut buf).unwrap() > 0 {
        match buf[0] {
            b'j' => { count += 1; render(count); }
            b'k' => { count -= 1; render(count); }
            b'q' => break,
            _ => {}
        }
    }

    // Restore terminal
    unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &old) };
}
```

### Test harness (`tests/common/mod.rs`)

A shared module provides helpers so integration tests stay concise:

```rust
use std::process::{Command, Output};

/// RAII guard that kills the tmux session on drop — ensures cleanup even on panic.
pub struct Session {
    pub name: String,
}

impl Session {
    pub fn new() -> Self {
        let name = format!("test-{}", uuid::Uuid::new_v4().to_string()[..8].to_owned());
        Session { name }
    }

    /// Run an agent-terminal command targeting this session.
    pub fn run(&self, args: &[&str]) -> Output {
        let mut cmd_args = args.to_vec();
        cmd_args.push("--session");
        cmd_args.push(&self.name);
        Command::new(env!("CARGO_BIN_EXE_agent-terminal"))
            .args(&cmd_args)
            .output()
            .expect("failed to run agent-terminal")
    }

    /// Run and assert success.
    pub fn run_ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(out.status.success(), "command failed: {:?}\nstderr: {}",
            args, String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).to_string()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Best-effort cleanup — don't panic in drop
        let _ = Command::new(env!("CARGO_BIN_EXE_agent-terminal"))
            .args(["close", "--session", &self.name])
            .output();
    }
}

/// Path to a compiled fixture binary.
pub fn fixture(name: &str) -> String {
    let path = env!("CARGO_BIN_EXE_fixture-{name}"); // resolved at compile time per-binary
    // fallback: find in target/debug/
    format!("target/debug/fixture-{name}")
}
```

### Integration test pattern

Each integration test follows the same structure:

```rust
mod common;
use common::Session;

#[test]
fn test_send_increments_counter() {
    let s = Session::new();

    // Setup: open fixture
    s.run_ok(&["open", &fixture("counter")]);
    s.run_ok(&["wait", "--text", "Count:"]);

    // Act: send keystrokes
    s.run_ok(&["send", "j"]);
    s.run_ok(&["wait", "--text", "Count: 1"]);

    // Assert: verify state
    let out = s.run(&["assert", "--text", "Count: 1"]);
    assert!(out.status.success());

    // Cleanup: Session::drop calls `close` automatically
}
```

Key patterns:
- **`Session` RAII guard** — `Drop` impl calls `close`, so cleanup happens even on panic/assert failure
- **`run_ok` for steps, `run` for assertions** — steps should never fail (panic early); assertions need the exit code
- **`fixture()` helper** — resolves compiled fixture binary path from the Cargo workspace
- **Unique session names** per test (UUID) so tests run in parallel safely
- **wait before assert** — never assert immediately after send, always wait for render

### Running tests

```bash
cargo test                        # all tests (unit + integration)
cargo test --lib                  # unit tests only
cargo test --test lifecycle       # single integration test file
cargo test -- --test-threads=4    # parallel (each test uses its own tmux session)
```

### CI configuration (GitHub Actions)

```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Install tmux
      run: sudo apt-get install -y tmux
    - name: Start tmux server
      run: tmux start-server
    - name: Run tests
      run: cargo test -- --test-threads=4
```

tmux works headless on Linux — no display server needed. Tests run in CI exactly as they do locally.

### What each test level covers

| Level | What it tests | Speed | Needs tmux? |
|---|---|---|---|
| **Unit** | ANSI parsing, key mapping, mouse encoding, wait logic | Fast (<1s total) | No |
| **Integration** | Full CLI → tmux → fixture → assert round-trip | Medium (~30s total) | Yes |

### Test coverage goals

Every subcommand in the status table above should have at least one integration test covering the happy path and one covering the error path (e.g., `snapshot` on a dead session should return a clear error, not hang). The "Test Status" column in the status table tracks this — a feature is `[T]` only when both the implementation and its tests pass.

---

## 8. Prerequisites & Dependencies

**Required**:
- `tmux` >= 3.0 (for `capture-pane -e -p` with proper escape handling and mouse support)
- A POSIX system (macOS, Linux) — tmux doesn't run on Windows natively

**Optional**:
- `freeze` (charmbracelet) — alternative PNG screenshot renderer if preferred over built-in
- `vhs` (charmbracelet) — session recording/GIF export

**No dependency on**:
- A real TTY (works in CI / headless environments)
- Any GUI framework
- Headless Chrome (screenshots are rendered natively)
- Node.js or Python (if implemented in Rust)

---

## Open Questions

1. **Ref system for TUIs?** Could we detect interactive elements (buttons, inputs, menu items) and assign @refs like agent-browser? Heuristics for common frameworks (bubbletea, ratatui, textual) could work — e.g., `[x]` patterns, `>` highlighted lines, bracketed labels. Worth exploring post-MVP.

### Resolved

- ~~Color/style in snapshots~~ → `snapshot --color` and `snapshot --json` with spans.
- ~~vhs integration~~ → No. Extra dependency, less control over timing. tmux is sufficient.
- ~~pty-level vs tmux~~ → tmux. Session persistence and multiplexing for free. Worth the indirection.
- ~~Windows~~ → Out of scope. tmux doesn't run on Windows. WSL users get the Linux binary.
