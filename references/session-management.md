# Session Management

agent-terminal uses tmux sessions as the runtime container for terminal applications. Every command targets a session by name.

---

## Default Session

If `--session` is omitted, the default name `agent-terminal` is used:

```bash
agent-terminal open "htop"           # creates session "agent-terminal"
agent-terminal snapshot              # reads from session "agent-terminal"
agent-terminal close                 # kills session "agent-terminal"
```

This is convenient for quick one-off usage, but for anything beyond a single test, use named sessions.

---

## Named Sessions

Use `--session <name>` to create isolated sessions:

```bash
agent-terminal open "./my-app" --session mytest
agent-terminal snapshot --session mytest
agent-terminal send "q" --session mytest
agent-terminal close --session mytest
```

Session names must be valid tmux session names (alphanumeric, hyphens, underscores, dots).

### Listing Sessions

```bash
agent-terminal list
```

Shows all active tmux sessions (not just agent-terminal ones). Sessions with names starting with `agent-terminal` are tagged.

---

## Multi-Pane Sessions

A single session can have multiple panes for testing apps that need companion processes (server + client, watcher + editor, etc.).

### Creating Panes

The first `open` creates the session. Subsequent `open` calls with `--pane` split the existing session:

```bash
# Create session with the server
agent-terminal open "npm run dev" --session myapp --pane server

# Add a test runner in a second pane
agent-terminal open "npm run test:watch" --session myapp --pane tests
```

The `--pane` flag on the first open is optional (the session gets created regardless). On subsequent opens, `--pane` creates a new horizontal split within the existing session.

### Targeting Panes

Use `--pane` on observation and interaction commands to target a specific pane:

```bash
agent-terminal snapshot --session myapp --pane server
agent-terminal snapshot --session myapp --pane tests
agent-terminal send "q" --session myapp --pane server
agent-terminal status --session myapp --pane server
```

Without `--pane`, commands target the default (first) pane.

### Multi-Pane Example

```bash
# Start a web server and a TUI client
agent-terminal open "python -m http.server 8080" --session demo --pane server
agent-terminal wait --stable 500 --session demo
agent-terminal open "./tui-client http://localhost:8080" --session demo --pane client

# Wait for client to connect
agent-terminal wait --text "Connected" --session demo --pane client

# Interact with the client
agent-terminal send "Enter" --session demo --pane client
agent-terminal wait --stable 300 --session demo --pane client
agent-terminal snapshot --session demo --pane client

# Check server logs
agent-terminal snapshot --session demo --pane server

# Clean up (closes all panes)
agent-terminal close --session demo
```

---

## Parallel Test Isolation

When running tests in parallel (CI, or multiple test files), each test must use a unique session name to avoid collisions.

### Strategy 1: PID-based names (bash)

```bash
SESSION="test-$$"
agent-terminal open "./my-app" --session "$SESSION"
# ... test ...
agent-terminal close --session "$SESSION"
```

`$$` is the shell PID -- unique per bash process.

### Strategy 2: Descriptive names

```bash
agent-terminal open "./my-app" --session "test-resize"
agent-terminal open "./my-app" --session "test-input"
agent-terminal open "./my-app" --session "test-colors"
```

### Strategy 3: UUID-based names

```bash
SESSION="test-$(uuidgen | head -c 8)"
agent-terminal open "./my-app" --session "$SESSION"
```

### What Happens on Collision

If you try to `open` a session that already exists, agent-terminal returns an error:

```
ERROR: Session 'test' already exists. Close it first or use a different name.
```

This is by design -- silent reuse could mask bugs.

---

## Cleanup

### Manual Cleanup

```bash
agent-terminal close --session mytest
```

This kills the tmux session and removes temp files (`/tmp/agent-terminal-<session>-stderr`, `/tmp/agent-terminal-<session>-exit`).

### Automatic Cleanup with `trap` (bash)

Always use `trap` in test scripts to clean up on failure:

```bash
#!/usr/bin/env bash
set -euo pipefail

SESSION="test-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
# ... tests ...
agent-terminal close --session "$SESSION"
trap - EXIT   # clear the trap on success
```

The `trap cleanup EXIT` runs even if the script fails with `set -e` or is interrupted with Ctrl+C.

### Cleaning Up All Sessions

If sessions leak (e.g., a test crashed without cleanup), you can list and close them:

```bash
# List all sessions
agent-terminal list

# Close specific leaked sessions
agent-terminal close --session at-matrix-3
agent-terminal close --session test-12345

# Nuclear option: kill all tmux sessions (careful!)
tmux kill-server
```

### Temp File Locations

agent-terminal creates temp files scoped to each session:

| File | Purpose |
|------|---------|
| `/tmp/agent-terminal-<session>-stderr` | Captured stderr from the process |
| `/tmp/agent-terminal-<session>-exit` | Exit code of the process |

These are cleaned up by `agent-terminal close`. If sessions are killed externally (e.g., `tmux kill-session`), these files may remain. They are harmless and will be overwritten on next use.

Performance recording also creates temp files:

| File | Purpose |
|------|---------|
| `/tmp/agent-terminal-perf/<session>-pid` | PID of the perf poller process |
| `/tmp/agent-terminal-perf/<session>-frames.jsonl` | Frame change data |
| `/tmp/agent-terminal-perf/<session>-poller.sh` | The poller script |

These are cleaned up by `agent-terminal perf stop`.

---

## Session Lifecycle Diagram

```
open --session s1
  |
  v
[tmux session "s1" created, command running]
  |
  |-- snapshot, send, type, wait, assert, find, click, ...
  |   (all target --session s1)
  |
  |-- open --session s1 --pane p2
  |   (splits session, creates second pane)
  |
  |-- status --session s1
  |   (check if process is alive)
  |
  v
close --session s1
  |
  v
[session killed, temp files removed]
```

---

## Tips

- Use descriptive session names in test scripts for debugging: `test-resize-small`, `test-nocolor`, `test-vim-bindings`.
- The `test-matrix` command handles session naming automatically -- each combination gets `at-matrix-N`.
- Session names are visible in `tmux list-sessions` if you need to debug manually.
- If Claude is running multiple tests, it should use unique session names for each and can run them sequentially without conflicts.
