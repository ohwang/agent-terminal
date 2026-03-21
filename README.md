# agent-terminal

**End-to-end testing for terminal applications.** Launch any TUI, observe the screen, send input, and assert on the result — all from the command line.

agent-terminal fills the gap that Playwright fills for web apps: automated, scriptable testing of interactive terminal interfaces. It works by running your app inside a tmux session and providing structured primitives for observation and interaction.

## Why

TUI apps are the only major category of software with zero testing infrastructure. Web apps have Playwright. Mobile has Appium. Terminal apps have nothing — until now.

Common bugs that agent-terminal catches automatically:
- **Crash on resize** — shrink the terminal and the app panics
- **Break on small terminals** — layouts that assume 80+ columns
- **Ignore NO_COLOR** — apps that blast ANSI escapes into pipes
- **Fail on TERM=dumb** — apps that crash instead of degrading gracefully

## Quick start

```bash
# Install (from source)
cargo install --path .

# Verify your environment
agent-terminal doctor

# Launch a TUI app
agent-terminal open "htop" --session demo

# See what's on screen
agent-terminal snapshot --session demo

# Interact
agent-terminal send q --session demo

# Clean up
agent-terminal close --session demo
```

## Example: testing a TUI app

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
trap 'agent-terminal close --session "$SESSION" 2>/dev/null || true' EXIT

# Launch
agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"

# Verify initial state
agent-terminal assert --text "Welcome" --session "$SESSION"

# Interact
agent-terminal send j --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"

# Verify result
agent-terminal assert --row 3 --row-text "> Option B" --session "$SESSION"

# Check colors
agent-terminal snapshot --color --session "$SESSION"
# Output:
#   1│ Welcome                    [fg:green bold]
#   2│   Option A
#   3│ > Option B                 [fg:cyan reverse]

echo "Test passed"
```

## Commands

### Lifecycle
| Command | Description |
|---|---|
| `open "<cmd>"` | Launch command in a tmux session |
| `close` | Kill the session |
| `list` | List active sessions |
| `status [--json]` | Process health (alive, PID, exit code, runtime) |

### Observation
| Command | Description |
|---|---|
| `snapshot` | Text dump with row numbers and metadata |
| `snapshot --color` | Text + style annotations (`[fg:red bold]`) |
| `snapshot --json` | Structured JSON with color spans |
| `snapshot --diff` | Diff against previous snapshot |
| `snapshot --raw` | Raw ANSI byte stream |
| `screenshot [--html]` | Render to PNG or HTML |
| `find "text"` | Search screen, return row,col |

### Interaction
| Command | Description |
|---|---|
| `send <keys...>` | Send key sequences (`j`, `Enter`, `C-c`, `Up`) |
| `type "text"` | Type literal text |
| `paste "text"` | Paste via tmux buffer |
| `resize <cols> <rows>` | Resize the terminal |
| `click <row> <col>` | Mouse click (SGR encoding) |
| `signal <SIG>` | Send Unix signal to the process |

### Waiting & assertion
| Command | Description |
|---|---|
| `wait --text "str"` | Poll until text appears |
| `wait --stable <ms>` | Poll until screen unchanged for N ms |
| `wait --regex "pattern"` | Poll until regex matches |
| `assert --text "str"` | Exit 0 if present, exit 1 with snapshot if not |
| `assert --row N --row-text "str"` | Check specific row content |

### Performance
| Command | Description |
|---|---|
| `perf start` / `perf stop` | Measure FPS over a period |
| `perf latency --key j` | Measure keystroke-to-render latency |

### Testing tools
| Command | Description |
|---|---|
| `doctor` | Validate tmux version and capabilities |
| `init` | Detect framework, generate starter test |
| `test-matrix` | Test across sizes, TERM values, color modes |
| `a11y-check` | Audit NO_COLOR, TERM=dumb, resize handling |

All commands accept `--session <name>` (default: `agent-terminal`).

## Snapshot output

**Plain text** (default):
```
[size: 80x24  cursor: 3,12  session: demo]
─────────────────────────────────────────
  1│ TODO List (1 items)
  2│ ──────────────────
  3│ > [ ] Buy groceries
  4│
  5│ [a]dd  [d]elete  [q]uit
```

**Color mode** (`--color`):
```
  1│ TODO List (1 items)            [fg:white bold]
  3│ > [ ] Buy groceries            [fg:green reverse]
  5│ [a]dd  [d]elete  [q]uit       [fg:cyan]
```

**JSON mode** (`--json`):
```json
{
  "session": "demo",
  "size": { "cols": 80, "rows": 24 },
  "cursor": { "row": 3, "col": 12 },
  "lines": [
    {
      "row": 1,
      "text": "TODO List (1 items)",
      "spans": [
        { "start": 0, "end": 19, "fg": "white", "bold": true }
      ]
    }
  ]
}
```

## Cross-terminal testing

Test your app across terminal configurations in one command:

```bash
agent-terminal test-matrix \
  --command "./my-app" \
  --sizes "80x24,120x40,40x10" \
  --terms "xterm-256color,dumb" \
  --colors "default,NO_COLOR=1" \
  --test 'wait --stable 500 && assert --text "Ready"'
```

Output:
```
                     xterm-256color  dumb
  80x24  default     ✓               ✓
  80x24  NO_COLOR    ✓               ✓
  120x40 default     ✓               ✓
  40x10  default     ✗ layout        ✗ crash

  10/12 passed, 2 failed
```

## Accessibility audit

```bash
agent-terminal a11y-check "./my-app"
```

```
  ✓ NO_COLOR: renders without ANSI color codes when NO_COLOR=1
  ✓ TERM=dumb: starts without crash
  ✗ Resize: crashes when resized to 40x10
  ✓ Contrast: no dim-on-dark text detected
```

## For AI agents

agent-terminal is designed for the autonomous agent loop:

```
snapshot → reason → act → wait → snapshot → repeat
```

The `SKILL.md` file provides Claude Code with the full command reference, failure recovery flowchart, framework-specific tips, and common mistakes to avoid. Install it as a Claude Code skill for automatic integration.

Key features for agents:
- **Process health detection** — `status --json` tells the agent if the app crashed, so it reads `logs --stderr` instead of blindly retrying
- **Structured output** — `--json` snapshots with color spans give agents precise, parseable screen state
- **Error output includes snapshots** — when a `wait` times out, the error message includes the last screen state, saving a round-trip

## Requirements

- **tmux >= 3.0** (run `agent-terminal doctor` to verify)
- macOS or Linux (tmux doesn't run on Windows natively)

## Building from source

```bash
git clone https://github.com/anthropics/agent-terminal
cd agent-terminal
cargo build --release
# Binary at target/release/agent-terminal (~7MB)
```

Run the test suite:
```bash
cargo test -- --test-threads=2
```

## License

MIT
