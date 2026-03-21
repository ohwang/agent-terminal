# Snapshot Output Formats

`agent-terminal snapshot` supports six output modes. They are mutually exclusive -- specify at most one flag.

---

## Plain Text (default)

The default format. Row-numbered, with a metadata header showing terminal size, cursor position, and session name.

```bash
agent-terminal snapshot --session myapp
```

Output:
```
[size: 80x24  cursor: 3,12  session: myapp]
-----------------------------------------
  1| File  Edit  View  Help
  2| ---------------------
  3| > item one
  4|   item two
  5|   item three
  6|
  7|
  ...
 24|
```

**When to use**: default for all general observation. Row numbers give a coordinate system for reasoning about positions.

The cursor position uses 0-indexed row and column values from tmux.

---

## Color Annotated (`--color`)

Same as plain text, but each line with non-default styling gets a bracketed annotation showing the dominant style.

```bash
agent-terminal snapshot --color --session myapp
```

Output:
```
[size: 80x24  cursor: 3,12  session: myapp]
-----------------------------------------
  1| File  Edit  View  Help              [fg:white bold]
  2| ---------------------               [fg:bright-black]
  3| > item one                          [fg:green bold reverse]
  4|   item two
  5|   Error: file not found             [fg:red]
  6|   ======== 60%                      [fg:blue]
```

Lines with default styling have no annotation. The annotation shows the dominant style on that line (the style covering the most non-whitespace characters).

**Style values**: `fg:<color>`, `bg:<color>`, `bold`, `dim`, `italic`, `underline`, `blink`, `reverse`, `strikethrough`.

**Color names**: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `bright-black` through `bright-white`, `color(N)` for 256-color palette, `rgb(R,G,B)` for true color.

**When to use**: when you need to verify colors/styles without the complexity of JSON. Good for quick checks like "is the error message red?" or "is the selected item highlighted?"

---

## Raw (`--raw`)

Exact byte stream from `tmux capture-pane -e -p` with zero post-processing. No row numbers, no metadata header, no parsing.

```bash
agent-terminal snapshot --raw --session myapp
```

Output:
```
\033[1;37mFile  Edit  View  Help\033[0m
\033[90m---------------------\033[0m
\033[1;32;7m> item one\033[0m
\033[0m  item two\033[0m
\033[31mError: file not found\033[0m
```

(Above shows escape sequences as `\033[...` for readability -- the actual output contains raw ESC bytes.)

**When to use**:
- Testing that your app emits correct escape sequences
- Piping to external tools (e.g., `agent-terminal snapshot --raw | aha > output.html`)
- Debugging cursor movement or non-SGR escape sequences that other modes discard
- Byte-level regression testing (diff raw snapshots across versions)

---

## ANSI (`--ansi`)

Middle ground: preserves raw ANSI escape sequences but adds row numbers and the metadata header.

```bash
agent-terminal snapshot --ansi --session myapp
```

Output:
```
[size: 80x24  cursor: 3,12  session: myapp]
-----------------------------------------
  1| \033[1;37mFile  Edit  View  Help\033[0m
  2| \033[90m---------------------\033[0m
  3| \033[1;32;7m> item one\033[0m
  4| \033[0m  item two\033[0m
  5| \033[31mError: file not found\033[0m
```

**When to use**: when you need the raw escape sequences for debugging but also want the spatial orientation (row numbers and size/cursor info) that the other formatted modes provide.

---

## JSON (`--json`)

Structured JSON output with full text and per-line color span data.

```bash
agent-terminal snapshot --json --session myapp
```

Output:
```json
{
  "session": "myapp",
  "size": {
    "cols": 80,
    "rows": 24
  },
  "cursor": {
    "row": 3,
    "col": 12
  },
  "lines": [
    {
      "row": 1,
      "text": "File  Edit  View  Help",
      "spans": [
        {
          "start": 0,
          "end": 22,
          "fg": "white",
          "bold": true
        }
      ]
    },
    {
      "row": 2,
      "text": "---------------------",
      "spans": [
        {
          "start": 0,
          "end": 21,
          "fg": "bright-black"
        }
      ]
    },
    {
      "row": 3,
      "text": "> item one",
      "spans": [
        {
          "start": 0,
          "end": 10,
          "fg": "green",
          "bold": true,
          "reverse": true
        }
      ]
    },
    {
      "row": 5,
      "text": "Error: file not found",
      "spans": [
        {
          "start": 0,
          "end": 6,
          "fg": "red",
          "bold": true
        },
        {
          "start": 6,
          "end": 21,
          "fg": "red"
        }
      ]
    }
  ]
}
```

### JSON Schema

Top level:

| Field | Type | Description |
|-------|------|-------------|
| `session` | string | Session name |
| `size.cols` | number | Terminal width in columns |
| `size.rows` | number | Terminal height in rows |
| `cursor.row` | number | Cursor row (0-indexed) |
| `cursor.col` | number | Cursor column (0-indexed) |
| `lines` | array | Array of Line objects |

Each Line:

| Field | Type | Description |
|-------|------|-------------|
| `row` | number | 1-indexed row number |
| `text` | string | Plain text content (ANSI stripped) |
| `spans` | array | Array of Span objects |

Each Span:

| Field | Type | Description |
|-------|------|-------------|
| `start` | number | Start character index (0-indexed, inclusive) |
| `end` | number | End character index (0-indexed, exclusive) |
| `fg` | string? | Foreground color name (omitted if default) |
| `bg` | string? | Background color name (omitted if default) |
| `bold` | bool? | Present and true if bold (omitted if false) |
| `dim` | bool? | Present and true if dim (omitted if false) |
| `italic` | bool? | Present and true if italic (omitted if false) |
| `underline` | bool? | Present and true if underline (omitted if false) |
| `blink` | bool? | Present and true if blink (omitted if false) |
| `reverse` | bool? | Present and true if reverse video (omitted if false) |
| `strikethrough` | bool? | Present and true if strikethrough (omitted if false) |

### Color Formats in JSON

| Format | Example | Description |
|--------|---------|-------------|
| Named basic | `"red"`, `"blue"` | Standard 8 ANSI colors |
| Named bright | `"bright-red"`, `"bright-cyan"` | Bright/high-intensity variants |
| 256-color | `"color(178)"` | xterm-256 palette index |
| True color | `"rgb(255,128,0)"` | 24-bit RGB |

**When to use**: programmatic processing, detailed color assertions, or when you need exact span boundaries. Most verbose but most precise.

---

## Diff (`--diff`)

Shows what changed since the last snapshot of the same session.

```bash
agent-terminal snapshot --diff --session myapp
```

Output shows lines that changed, prefixed with `+` (added/modified) and `-` (removed/modified):

```
[size: 80x24  cursor: 4,12  session: myapp  diff]
-----------------------------------------
- 3| > item one                          [selected]
+ 3|   item one
- 4|   item two
+ 4| > item two                          [selected]
```

Unchanged lines are omitted to reduce output.

**When to use**: after sending a key, to see exactly what changed. Reduces the amount of output to reason about when only one or two lines changed. Particularly useful for list navigation, form input, or any incremental UI update.

---

## Combining with `--scrollback`

The `--scrollback N` flag can be combined with the default (plain text) mode to include N lines of scrollback above the current viewport:

```bash
agent-terminal snapshot --scrollback 50 --session myapp
```

This prepends scrollback content above the visible viewport, useful for scrolling CLI apps where important output may have scrolled off screen.

Note: `--scrollback` is most useful for scrolling CLI apps (REPLs, build output). For full-screen TUI apps that use the alternate screen buffer, scrollback contains pre-TUI shell output and is generally not useful.
