# Matrix Testing

Test your app across terminal sizes, TERM values, and color modes in one command:

```bash
agent-terminal test-matrix \
  --command "./my-app" \
  --sizes "80x24,120x40,40x10" \
  --terms "xterm-256color,dumb" \
  --colors "default,NO_COLOR=1" \
  --test "agent-terminal assert --text 'Welcome' --session {session}; agent-terminal status --session {session} --json"
```

This runs 12 combinations (3 sizes x 2 terms x 2 colors) and reports:

```
COMBINATION                              RESULT
------------------------------------------------------------
80x24+xterm-256color+default             pass
80x24+xterm-256color+NO_COLOR=1          pass
80x24+dumb+default                       FAIL: Process crashed during startup
40x10+xterm-256color+default             FAIL: text "Welcome" not found
...

10/12 passed, 2 failed
Failure snapshots saved to: ./agent-terminal-matrix/
```

Use `{session}` in test commands -- it gets replaced with the per-combination session name.

**When to use**: after you have a working app and want to verify it handles edge cases (terminal sizes, color modes, TERM values). Not for initial development.
