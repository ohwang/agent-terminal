# Session Management Guide

How agent-terminal uses tmux sessions, and strategies for isolation, parallelism, and cleanup.

---

## Default Session

When `--session` is omitted, all commands use the default session name `agent-terminal`.

```bash
# These are equivalent:
agent-terminal open "./my-app"
agent-terminal open "./my-app" --session agent-terminal
```

The default session is convenient for single-test scenarios. For anything involving multiple concurrent tests, use named sessions.

---

## Named Sessions

Give each test its own session name to avoid collisions.

```bash
agent-terminal open "./my-app" --session test-navigation
agent-terminal open "./my-app" --session test-resize
agent-terminal open "./my-app" --session test-colors
```

### Naming Conventions

**By test purpose:**

```bash
--session test-nav
--session test-input
--session test-perf
```

**By PID (for bash scripts running in parallel):**

```bash
--session "test-$$"       # e.g., test-48231
```

**By timestamp:**

```bash
--session "test-$(date +%s)"
```

**By CI job:**

```bash
--session "ci-${CI_JOB_ID:-local}"
```

---

## Multi-Pane Support

A single session can contain multiple named panes. This is useful for testing client/server architectures or multi-process setups.

```bash
# Start a server in one pane
agent-terminal open "./server" --session mytest --pane server

# Start a client in another pane
agent-terminal open "./client" --session mytest --pane client
```

Observation and interaction commands accept `--pane` to target a specific pane:

```bash
# Snapshot the server pane
agent-terminal snapshot --session mytest --pane server

# Type into the client pane
agent-terminal type "connect localhost:8080" --session mytest --pane client

# Check status of both
agent-terminal status --session mytest --pane server
agent-terminal status --session mytest --pane client
```

When `--pane` is omitted, commands target the first (default) pane.

### Closing Multi-Pane Sessions

`close` kills the entire session, including all panes:

```bash
agent-terminal close --session mytest   # kills both server and client panes
```

---

## Session Cleanup

### Trap Pattern (Bash Scripts)

Always use a trap to ensure cleanup on exit, error, or interrupt:

```bash
#!/usr/bin/env bash
set -euo pipefail

SESSION="test-$$"

cleanup() {
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

agent-terminal open "./my-app" --session "$SESSION"
# ... run test ...
agent-terminal close --session "$SESSION"
trap - EXIT
```

The `2>/dev/null || true` ensures cleanup never fails -- if the session is already gone, the error is silently ignored.

### Manual Cleanup

List and kill stale sessions:

```bash
# See what's running
agent-terminal list

# Kill a specific session
agent-terminal close --session stale-test

# Kill all agent-terminal sessions (bash one-liner)
agent-terminal list | while read -r name _rest; do
    agent-terminal close --session "$name"
done
```

### Cleanup Before Test

If a previous run crashed without cleanup:

```bash
# Ensure clean slate
agent-terminal close --session "$SESSION" 2>/dev/null || true
agent-terminal open "./my-app" --session "$SESSION"
```

---

## CI/CD Isolation

### GitHub Actions

```yaml
jobs:
  tui-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install tmux
        run: sudo apt-get install -y tmux

      - name: Build
        run: cargo build --release

      - name: TUI Tests
        run: |
          export PATH="$PWD/target/release:$PATH"
          bash tests/tui-test.sh
```

Key points:
- tmux works headlessly -- no display server needed.
- Each job gets a fresh VM, so session names do not collide across jobs.
- Within a single job, use unique session names if running multiple tests.

### Parallel Test Jobs

When running tests in parallel (e.g., matrix strategy), session names are already isolated because each job runs in a separate VM/container. Within a single job, use unique names:

```bash
# Run tests in parallel within one job
for test_file in tests/tui-*.sh; do
    bash "$test_file" &
done
wait
```

Each test script should use `SESSION="test-$$"` to get a unique session name per process.

### Docker

tmux works inside Docker containers. The container provides natural isolation:

```dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y tmux
COPY agent-terminal /usr/local/bin/
COPY my-app /usr/local/bin/
COPY tests/ /tests/
CMD ["bash", "/tests/run-all.sh"]
```

---

## Session Lifecycle Diagram

```
open          wait --stable     snapshot/send/wait     close
  |               |                    |                 |
  v               v                    v                 v
[create] --> [stabilize] --> [observe/interact loop] --> [kill]
  tmux           app                  test              tmux
  session        renders              logic             session
  starts         first                runs              destroyed
                 frame
```

---

## Troubleshooting

### "Session already exists"

A previous run did not clean up. Fix:

```bash
agent-terminal close --session <name>
```

### "No such session"

The session was never created or was already closed. Check:

```bash
agent-terminal list
```

### "Permission denied" on tmux socket

This can happen in CI environments with strict `/tmp` permissions. Verify with:

```bash
agent-terminal doctor
```

### Sessions accumulating in development

During development, sessions may pile up. Periodic cleanup:

```bash
# List all sessions
agent-terminal list

# Close all
agent-terminal list | awk '{print $1}' | xargs -I{} agent-terminal close --session {}
```
