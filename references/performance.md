# Performance Testing

## Start/Stop Mode (recommended)

```bash
agent-terminal open "./my-app" --session perf
agent-terminal wait --stable 500 --session perf

# Start recording frames
agent-terminal perf start --session perf

# Perform rapid interactions (no --wait-stable here -- want to stress-test)
agent-terminal send "j" --session perf
agent-terminal send "j" --session perf
agent-terminal send "j" --session perf
agent-terminal send "G" --session perf

# Stop and get metrics
agent-terminal perf stop --json --session perf
# Returns: { "fps": 24.5, "frame_count": 12, "p95_frame_ms": 88, ... }

# Measure input latency separately
agent-terminal perf latency --key "j" --samples 5 --json --session perf
# Returns: { "mean_ms": 18, "p95_ms": 38, ... }

agent-terminal close --session perf
```

## Quick One-Shot Measurement

```bash
agent-terminal perf fps --duration 3000 --session perf    # passive observation
agent-terminal perf fps --during 'send "j" && send "k"' --session perf  # during actions
```

## Interpreting Results

- **FPS**: 0 = frozen, 1-5 = sluggish, 10-30 = normal
- **Latency**: <16ms = excellent, 16-50ms = good, 50-100ms = noticeable, >100ms = sluggish
