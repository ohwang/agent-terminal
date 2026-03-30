# agent-terminal

[![CI](https://github.com/ohwang/agent-terminal/actions/workflows/ci.yml/badge.svg)](https://github.com/ohwang/agent-terminal/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/agent-terminal.svg)](https://crates.io/crates/agent-terminal)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Playwright for terminal apps. Launch any TUI in a tmux session, observe the screen, send input, assert on results.

```bash
agent-terminal open "htop" --session demo
agent-terminal snapshot --session demo        # text dump with row numbers
agent-terminal send q --session demo          # send keystrokes
agent-terminal close --session demo
```

## Commands

### Lifecycle
| Command | Purpose |
|---|---|
| `open "<cmd>"` | Launch in tmux session |
| `close` | Kill session |
| `list` | List active sessions |
| `status [--json]` | PID, alive/dead, exit code, runtime. JSON includes `last_stderr` when dead |

### Observe
| Command | Purpose |
|---|---|
| `snapshot` | Plain text with row numbers |
| `snapshot --color` | Text + style annotations (`[fg:red bold]`) |
| `snapshot --json` | Structured JSON with color spans (includes pane layout for multi-pane sessions) |
| `snapshot --diff` | Diff against previous snapshot |
| `screenshot [--html]` | PNG or HTML rendering |
| `find "text" [--json]` | Search screen, return row,col. `--json` returns `{matches: [{row, col, text, style?}]}` |

### Interact
| Command | Purpose |
|---|---|
| `send <keys...>` | Key sequences (`j`, `Enter`, `C-c`, `Up`). Chain: `--wait-stable`, `--capture`, `--screenshot` |
| `type "text"` | Literal text input. Chain: `--enter`, `--wait-stable`, `--capture`, `--screenshot` |
| `paste "text"` | Via tmux buffer |
| `resize <cols> <rows>` | Change terminal size |
| `click <row> <col>` | Mouse click (SGR encoding) |
| `signal <SIG>` | Unix signal to process |

### Wait & Assert
| Command | Purpose |
|---|---|
| `wait --text "str"` | Poll until text appears. Chain: `--capture`, `--screenshot` |
| `wait --text-any "a" "b"` | Poll until any of the texts appears (OR semantics) |
| `wait --stable <ms>` | Poll until screen stops changing |
| `wait --regex "pat"` | Poll until regex matches |
| `assert --text "str"` | Exit 0 if present, 1 with snapshot if not |
| `assert --row N --row-text "str"` | Check specific row |

### Test
| Command | Purpose |
|---|---|
| `test-matrix` | Sweep sizes × TERM values × color modes |
| `a11y-check` | Audit NO_COLOR, TERM=dumb, resize handling |
| `perf start/stop` | Measure FPS |
| `perf latency` | Keystroke-to-render latency |
| `doctor` | Validate tmux version |
| `init` | Generate starter test script |

All commands take `--session <name>` (default: `agent-terminal`).

## Example test

```bash
#!/usr/bin/env bash
set -euo pipefail
SESSION="test-$$"
trap 'agent-terminal close --session "$SESSION" 2>/dev/null || true' EXIT

agent-terminal open "./my-app" --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal assert --text "Welcome" --session "$SESSION"
agent-terminal send j --session "$SESSION"
agent-terminal wait --stable 200 --session "$SESSION"
agent-terminal assert --row 3 --row-text "> Option B" --session "$SESSION"
```

## Cross-terminal matrix

```bash
agent-terminal test-matrix \
  --command "./my-app" \
  --sizes "80x24,120x40,40x10" \
  --terms "xterm-256color,dumb" \
  --colors "default,NO_COLOR=1" \
  --test 'wait --stable 500 && assert --text "Ready"'
```

```
                     xterm-256color  dumb
  80x24  default     ✓               ✓
  80x24  NO_COLOR    ✓               ✓
  120x40 default     ✓               ✓
  40x10  default     ✗ layout        ✗ crash
```

## AI agent integration

Designed for the agent loop: `snapshot → reason → act → wait → repeat`.

Install `SKILL.md` as a Claude Code skill for full command reference, failure recovery, and framework-specific tips. Key features:
- `status --json` detects crashes and includes `last_stderr` so agents don't need separate calls
- `--json` snapshots give parseable screen state with color spans and pane layout
- `find --json` returns structured matches with row, col, text, and style
- `wait --text-any` handles branching flows (success OR error) in one call
- `wait --json` returns structured timeout errors with diagnostics (elapsed, change count)
- Failed `wait` includes the last screen state in the error, saving a round-trip

## Install

Requires **tmux >= 3.0** (`agent-terminal doctor` to verify). macOS or Linux.

```bash
cargo install --path .
# or
cargo build --release  # binary at target/release/agent-terminal
```

## License

MIT
