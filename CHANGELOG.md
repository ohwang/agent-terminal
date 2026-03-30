# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-03-30

### Added

#### Session lifecycle
- `open` command to launch apps in tmux sessions with env vars, custom size, `--shell` keep-alive, and `--no-stderr` mode
- `close` command to kill sessions
- `list` command to show active sessions
- `status` command with `--json` output (alive, PID, exit code, runtime)
- `exit-code` command to retrieve process exit code
- `logs` command to capture stderr/stdout from the managed process
- `doctor` command to validate tmux version and capabilities
- `init` command to detect framework and generate a starter test script

#### Observation
- `snapshot` with plain text, `--color` (style annotations), `--raw`, `--ansi`, `--json` (structured with color spans), and `--diff` modes
- `snapshot --scrollback` to include scrollback history
- `snapshot --window` to composite all panes in a multi-pane layout
- `screenshot` rendering to PNG (with real text via ab_glyph) and `--html`
- `screenshot --annotate` to overlay a row/col grid
- `screenshot --window` for multi-pane capture
- `scrollback` command to read and search tmux scrollback buffer
- `find` command to locate text on screen by literal or regex, with `--color` filtering

#### Interaction
- `send` command for key sequences (e.g., `j`, `Enter`, `C-c`, `Up`) with `--wait-stable`
- `type` command for literal text input with `--enter` and `--wait-stable` flags
- `paste` command via tmux paste buffer
- `resize` command to change terminal dimensions
- `click` command with SGR mouse encoding, `--right`, and `--double` click
- `drag` command for mouse drag between positions
- `scroll-wheel` command for scroll events
- `signal` command to send Unix signals to the managed process
- `clipboard` command for read/write/paste operations
- Named `--pane` support across all interaction and observation commands

#### Waiting and assertions
- `wait` with `--text`, `--text-gone`, `--stable`, `--cursor`, `--regex`, and `--exit` conditions
- `wait` with configurable `--timeout` and `--interval`
- `assert` with `--text`, `--no-text`, `--row`/`--row-text`, `--cursor-row`, `--color`/`--color-style`, and `--style`/`--style-check`
- Error output includes the last snapshot on wait timeout or assertion failure

#### Performance measurement
- `perf start`/`perf stop` for frame recording with JSON output
- `perf fps` to measure FPS with `--during` (run command while measuring) and `--duration` (passive observation)
- `perf latency` to measure keystroke-to-render latency with configurable samples

#### Cross-terminal testing
- `test-matrix` to test across terminal sizes, TERM values, and color modes in a single run

#### Accessibility
- `a11y-check` to audit NO_COLOR compliance, TERM=dumb handling, resize resilience, and contrast

#### Recording and replay
- `record start`/`record stop` to capture session recordings with configurable FPS and group/label metadata
- `record list` to enumerate recordings with `--json` output
- `record view` for AI-readable chronological playback of recorded sessions
- `web` command to launch an embedded web viewer for recorded sessions
- Action logging: interaction commands are automatically logged into active recordings

#### Live monitoring
- `watch` command for a live dashboard of all active sessions with `--filter` prefix matching

#### Developer experience
- Claude Code skill integration via `SKILL.md`
- Starter test templates for basic, curses, and REPL apps
- Default terminal size optimized for LLM vision (112x30)
