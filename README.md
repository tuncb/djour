# djour

Terminal journal and notes application written in Rust.

## Main Features

- Multiple journal modes: `daily`, `weekly`, `monthly`, `single`
- Time references: `today`, `yesterday`, `tomorrow`, weekdays, `last <weekday>`, `next <weekday>`, specific dates
- Tag-aware markdown notes with boolean tag queries
- Compile tagged content into markdown reports
- Works with your preferred editor and plain files on disk

## Installation

### From source

```bash
git clone https://github.com/yourusername/djour.git
cd djour
cargo install --path .
```

## Quick Start

```bash
# Initialize journal in current directory (default mode: daily)
djour init

# Open today's note in your editor
djour --open today

# List recent notes
djour list

# Compile all content tagged #work
djour compile work
```

## How Tags Work

Tags are written directly in markdown as `#tag_name`.

### Tag syntax

- Start with `#`
- Allowed characters after `#`: letters, numbers, `_`, `-`
- Case-insensitive (`#Work` and `#work` are treated the same)

### Section-level tags

Tags in a heading apply to content under that heading until the next heading of the same or higher level.

```markdown
## Sprint Planning #work #sprint

Define scope and tasks.

### Backend

This subsection inherits #work and #sprint.
```

### Paragraph-level tags

Tags at the end of a paragraph apply only to that paragraph.

```markdown
Prepare release checklist. #work #ops

Buy groceries after work.
```

### Tag queries (`compile`)

`djour compile <QUERY>` supports:

- `AND`
- `OR`
- `NOT`
- Parentheses for grouping

Examples:

```bash
djour compile "work AND urgent"
djour compile "work OR personal"
djour compile "work NOT meeting"
djour compile "(work AND sprint) OR (personal AND learning)"
```

## Executable Arguments

### Global usage

```bash
djour [OPTIONS] [TIME_REF]
djour <COMMAND>
```

### Global arguments and options

- `[TIME_REF]`: time reference for note selection
- `--open`: open selected note in configured editor (requires `TIME_REF`)
- `-h, --help`: print help
- `-V, --version`: print version

Accepted `TIME_REF` forms:

- `today`, `yesterday`, `tomorrow`
- `monday` ... `sunday`
- `last monday`, `next friday`
- Date in `DD-MM-YYYY` format, for example `17-01-2025`

### `init`

Initialize a new journal.

```bash
djour init [PATH] [--mode <MODE>]
```

- `[PATH]`: target directory (default: `.`)
- `-m, --mode <MODE>`: `daily|weekly|monthly|single` (default: `daily`)

### `config`

View or modify config.

```bash
djour config [OPTIONS] [KEY] [VALUE]
```

- `[KEY]`: config key to read/write
- `[VALUE]`: value to set
- `-l, --list`: list all config values

Examples:

```bash
djour config --list
djour config mode
djour config mode weekly
djour config editor "code -w"
```

### `list`

List notes.

```bash
djour list [--from <DATE>] [--to <DATE>] [--limit <N>]
```

- `--from <DATE>`: start date inclusive (`DD-MM-YYYY`)
- `--to <DATE>`: end date inclusive (`DD-MM-YYYY`)
- `--limit <N>`: max entries to show (default: `10`)

### `compile`

Compile tagged content.

```bash
djour compile <QUERY> [OPTIONS]
```

- `<QUERY>`: tag query expression
- `-o, --output <PATH>`: output file (default: `compilations/<tag>.md`)
- `--from <DATE>`: start date filter (`DD-MM-YYYY`)
- `--to <DATE>`: end date filter (`DD-MM-YYYY`)
- `--format <FORMAT>`: `chronological|grouped` (default: `chronological`)
- `--include-context`: include parent section headings
- `--open`: open compiled output in editor

### `mode`

Migrate journal mode (`daily <-> weekly`).

```bash
djour mode <MODE> [OPTIONS]
```

- `<MODE>`: target mode (`daily` or `weekly`)
- `--from <MODE>`: override detected current mode (`daily` or `weekly`)
- `--dry-run`: show migration plan only
- `--yes`: apply migration (required unless `--dry-run`)
- `--archive-dir <PATH>`: archive folder relative to journal root

## Configuration Keys

- `mode`: journal mode
- `editor`: editor command
- `created`: creation timestamp (read-only)

## Environment Variables

| Variable | Purpose |
|---|---|
| `DJOUR_ROOT` | Default journal directory |
| `DJOUR_MODE` | Override configured journal mode |
| `EDITOR` | Preferred editor |
| `VISUAL` | Fallback editor |

Editor selection order:

1. `EDITOR`
2. `VISUAL`
3. `.djour/config.toml` `editor`
4. System default (`notepad` on Windows, `nano` on Unix)

## Development

```bash
cargo build
cargo test
cargo fmt
cargo clippy --all-targets --all-features
```
