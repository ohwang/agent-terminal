# CLAUDE.md

## Project overview

agent-terminal is a Rust CLI and agent skill for autonomous TUI testing via tmux. It provides structured observation (snapshot, screenshot), interaction (send, type, click), and assertion primitives that enable AI agents and developers to test terminal applications end-to-end.

## Build & run

```bash
cargo build                          # debug build
cargo build --release                # release build (~7MB binary)
cargo test -- --test-threads=2       # run all tests (requires tmux >= 3.0)
cargo test --test lifecycle          # run a single test file
```

Binary: `target/debug/agent-terminal` (or `target/release/agent-terminal`)

## Project structure

```
src/
  main.rs        # CLI entrypoint (clap), dispatches to modules
  ansi.rs        # shared ANSI parser: Style, Span, Line, parse_ansi, style matching
  session.rs     # tmux session lifecycle (open/close/list/status/doctor/init/test-matrix/a11y-check)
  snapshot.rs    # capture-pane + output formatting (plain/color/raw/ansi/json/diff)
  interact.rs    # send-keys, type, paste, resize, mouse, signals, clipboard
  wait.rs        # poll-based wait, assert, find
  annotate.rs    # screenshot rendering (PNG via image crate, HTML)
  perf.rs        # FPS measurement (start/stop + inline) and latency probes

tests/
  common/mod.rs  # test harness (Session RAII guard, fixture helpers)
  *_test.rs      # 15 integration test files (91 tests total)
  fixtures/      # 7 minimal TUI apps used as test targets (separate Cargo workspace member)

references/      # detailed docs (commands, snapshot formats, session mgmt, patterns)
templates/       # starter bash test scripts (basic, curses, REPL)
SKILL.md         # Claude Code skill definition — the main AI prompt
```

## Architecture notes

- **tmux is the runtime dependency.** All commands ultimately call `tmux` via `std::process::Command`. No PTY handling in-process.
- **Session isolation.** Each session uses temp files at `/tmp/agent-terminal-<session>-{stderr,exit}` for process health tracking.
- **ANSI parsing is shared.** The `ansi.rs` module contains the canonical ANSI parser (Style, Span, parse_ansi, parse_ansi_line) used by `snapshot.rs`, `wait.rs`, and `watch.rs`. The `annotate.rs` module still has its own renderer-specific parser that resolves to RGB tuples.
- **Screenshot rendering is basic.** The PNG renderer uses a simple block-character approach rather than real font rasterization via `ab_glyph`. The HTML renderer is the higher-quality path. `ab_glyph` is a dependency but not fully wired up for glyph rendering yet.
- **Perf start/stop** spawns a shell script as a background process that polls capture-pane. The PID is tracked in `/tmp/agent-terminal-perf/`.

## Key patterns

- All public functions return `Result<(), String>` — errors print to stderr and exit 1.
- Error output includes the last snapshot when possible (see `rich_error()` in session.rs, error formatting in wait.rs).
- Integration tests use unique session names (`test-<pid>-<counter>`) for parallel safety.
- Test fixtures are compiled Rust binaries in a workspace member (`tests/fixtures/`).

## Testing

Tests require tmux >= 3.0 installed. Run `agent-terminal doctor` to verify.

Tests create and destroy tmux sessions. The `Session` RAII guard in `tests/common/mod.rs` ensures cleanup even on panic. Use `--test-threads=2` or higher for reasonable parallelism without overwhelming tmux.

Fixture binaries are built automatically as part of the workspace. They live in `tests/fixtures/src/bin/` and are referenced by name (e.g., `fixture-counter`).

## Common tasks

- **Add a new subcommand**: Add variant to `Commands` enum in `main.rs`, implement in the appropriate module, add integration test.
- **Add a test fixture**: Create `tests/fixtures/src/bin/foo_app.rs`, add `[[bin]]` entry to `tests/fixtures/Cargo.toml`.
- **Fix ANSI parsing**: The canonical parser is in `ansi.rs` (`parse_ansi`, `parse_ansi_line`). The renderer-specific parser in `annotate.rs` resolves to RGB tuples and is separate by design.
