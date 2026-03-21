# Snapshot Format Reference

The `snapshot` command supports six output formats. This document describes each format with examples.

---

## Plain Text (default)

The default output format. Includes a metadata header, separator line, and numbered rows.

**Usage:**

```bash
agent-terminal snapshot --session test1
```

**Output structure:**

```
[size: 80x24  cursor: 3,1  session: test1]
-----------------------------------------
  1| Welcome to my-app
  2|
  3| > Option A
  4|   Option B
  5|   Option C
  6|
  7| [q]uit  [Enter] select
  8|
  9|
 10|
 ...
 24|
```

**Header fields:**

| Field | Description |
|-------|-------------|
| `size` | Terminal dimensions as `COLSxROWS` |
| `cursor` | Current cursor position as `row,col` (1-indexed) |
| `session` | Session name |

**Row format:** Right-justified line number, pipe separator, then the row content. Empty rows below content are included up to the terminal height.

---

## Color (`--color`)

Annotated format that includes style information for each row. Style annotations appear in square brackets before the text they apply to.

**Usage:**

```bash
agent-terminal snapshot --color --session test1
```

**Output structure:**

```
[size: 80x24  cursor: 3,1  session: test1]
-----------------------------------------
  1| [bold]Welcome to my-app[/bold]
  2|
  3| [fg:green bold]> Option A[/fg:green bold]
  4|   Option B
  5|   Option C
  6|
  7| [dim][q]uit  [Enter] select[/dim]
```

**Style annotations:**

| Annotation | Meaning |
|------------|---------|
| `[bold]` | Bold text |
| `[dim]` | Dimmed text |
| `[italic]` | Italic text |
| `[underline]` | Underlined text |
| `[fg:COLOR]` | Foreground color (e.g., `fg:red`, `fg:green`, `fg:#ff0000`) |
| `[bg:COLOR]` | Background color |
| `[reverse]` | Reversed fg/bg |

Annotations nest: `[fg:red bold]Error[/fg:red bold]`.

---

## Raw (`--raw`)

Direct tmux capture-pane output with no formatting, headers, or line numbers. Useful for piping to other tools.

**Usage:**

```bash
agent-terminal snapshot --raw --session test1
```

**Output structure:**

```
Welcome to my-app

> Option A
  Option B
  Option C

[q]uit  [Enter] select



```

No header. No line numbers. No separator. Trailing spaces on each row are preserved. Rows extend to the full terminal width.

---

## ANSI (`--ansi`)

Raw ANSI escape sequences with row numbers. Preserves the original terminal escape codes for exact color reproduction.

**Usage:**

```bash
agent-terminal snapshot --ansi --session test1
```

**Output structure:**

```
  1| \e[1mWelcome to my-app\e[0m
  2|
  3| \e[32;1m> Option A\e[0m
  4|   Option B
  5|   Option C
  6|
  7| \e[2m[q]uit  [Enter] select\e[0m
```

The escape sequences are actual ANSI codes (shown here as `\e[...]` for readability). When printed to a terminal that supports ANSI, colors and styles render correctly.

---

## JSON (`--json`)

Full structured output with metadata, rows, cursor position, and style spans. Best for programmatic processing.

**Usage:**

```bash
agent-terminal snapshot --json --session test1
```

**Output structure:**

```json
{
  "session": "test1",
  "size": {
    "cols": 80,
    "rows": 24
  },
  "cursor": {
    "row": 3,
    "col": 1,
    "visible": true
  },
  "rows": [
    {
      "row": 1,
      "text": "Welcome to my-app",
      "spans": [
        {
          "start": 0,
          "end": 18,
          "style": {
            "bold": true,
            "fg": null,
            "bg": null
          }
        }
      ]
    },
    {
      "row": 2,
      "text": "",
      "spans": []
    },
    {
      "row": 3,
      "text": "> Option A",
      "spans": [
        {
          "start": 0,
          "end": 10,
          "style": {
            "bold": true,
            "fg": "green",
            "bg": null
          }
        }
      ]
    }
  ],
  "title": "my-app",
  "timestamp": "2026-03-21T10:30:00Z"
}
```

**Top-level fields:**

| Field | Type | Description |
|-------|------|-------------|
| `session` | string | Session name |
| `size` | object | `cols` and `rows` as integers |
| `cursor` | object | `row`, `col` (1-indexed), `visible` (boolean) |
| `rows` | array | Array of row objects |
| `title` | string | Terminal title (if set by the app) |
| `timestamp` | string | ISO 8601 capture time |

**Row object fields:**

| Field | Type | Description |
|-------|------|-------------|
| `row` | integer | Row number (1-indexed) |
| `text` | string | Plain text content of the row |
| `spans` | array | Array of style span objects |

**Span object fields:**

| Field | Type | Description |
|-------|------|-------------|
| `start` | integer | Start column (0-indexed into text) |
| `end` | integer | End column (exclusive) |
| `style` | object | Style properties |

**Style object fields:**

| Field | Type | Description |
|-------|------|-------------|
| `bold` | bool | Bold |
| `dim` | bool | Dim |
| `italic` | bool | Italic |
| `underline` | bool | Underline |
| `reverse` | bool | Reversed colors |
| `fg` | string or null | Foreground color name or hex |
| `bg` | string or null | Background color name or hex |

---

## Diff (`--diff`)

Shows only rows that changed since the last snapshot for the same session. Unchanged rows are omitted. Changed rows are marked with `~`, new content with `+`, removed content with `-`.

**Usage:**

```bash
agent-terminal snapshot --diff --session test1
```

**Output structure (after moving cursor from Option A to Option B):**

```
[size: 80x24  cursor: 4,1  session: test1  diff: 2 rows changed]
-----------------------------------------
  3|~  Option A
  4|~> Option B
```

**Markers:**

| Marker | Meaning |
|--------|---------|
| `~` | Row content changed |
| `+` | New row (previously empty) |
| `-` | Row cleared (previously had content) |
| _(no marker)_ | Row unchanged (omitted from output) |

If no previous snapshot exists for the session, the diff output is identical to the full plain-text snapshot with all rows marked `+`.

---

## Scrollback Integration

The `--scrollback N` flag can be combined with any format (except `--raw`) to include N lines from the scrollback buffer above the current viewport.

**Usage:**

```bash
agent-terminal snapshot --scrollback 10 --session test1
```

**Output structure:**

```
[size: 80x24  cursor: 1,0  session: test1  scrollback: 10]
-----------------------------------------
 -10| previous output line 1
  -9| previous output line 2
  ...
  -1| previous output line 10
  ---- viewport ----
   1| current visible line 1
   2| current visible line 2
  ...
  24|
```

Scrollback lines are numbered with negative indices. A separator marks the boundary between scrollback and the visible viewport.
