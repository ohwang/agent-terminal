# Recording Sessions

Use recording to capture before/after sessions for review. Actions are auto-logged with timestamps. Frames are deduplicated (only screen changes recorded).

## Before/After Pattern

```bash
# 1. Open and start recording
agent-terminal open "./my-app" --session app
agent-terminal wait --stable 500 --session app
agent-terminal record start --session app --group "fix-42" --label "before"

# 2. Interact to demonstrate behavior
agent-terminal send "j" --session app --wait-stable 200

# 3. Stop recording
agent-terminal record stop --session app
# Output: Recording dir: ~/.agent-terminal/recordings/fix-42/...
agent-terminal close --session app

# ... make code changes ...

# 4. Record the "after" state (same steps with --label "after")
```

## Reviewing Recordings

```bash
# View key frames (before/after each action + final frame)
agent-terminal record view --dir <path from record stop>

# View all frames interleaved chronologically
agent-terminal record view --dir <path> --all-frames

# Structured JSON output
agent-terminal record view --dir <path> --json

# List all recordings
agent-terminal record list

# Web viewer for visual replay
agent-terminal web --port 8080
```

## Recording Files

Each recording directory contains:
- `recording.cast` -- asciinema v2 format (visual replay with colors)
- `frames.jsonl` -- plain-text snapshots per frame (AI-readable)
- `actions.jsonl` -- agent-terminal commands logged during recording
- `meta.json` -- session, group, label, duration, frame count

## Options

- `--group` ties related recordings together (e.g., all recordings for one bug fix)
- `--label` distinguishes within a group (e.g., "before", "after")
- `--fps N` sets capture rate (default: 10)
- `--dir path` saves to a custom directory instead of `~/.agent-terminal/recordings/`
