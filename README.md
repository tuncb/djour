# djour - Terminal Journal Application

> A lightweight, flexible command-line journal application written in Rust

## Features

- 📅 **Multiple journal modes** - Daily, weekly, monthly, or single continuous file
- 🏷️ **Powerful tag system** - Boolean queries with AND, OR, NOT operators
- 📝 **Custom templates** - Personalize your note structure
- 🔍 **Smart time references** - Use natural language like "today", "last friday"
- 📊 **Tag compilation** - Extract and compile tagged content across notes
- ⚡ **Fast and lightweight** - Built with Rust for performance
- 🎨 **Editor agnostic** - Works with your favorite text editor
- 🌍 **Cross-platform** - Windows, macOS, and Linux support

## Quick Start

```bash
# Initialize a journal in the current directory
djour init

# Open today's note
djour today

# Open yesterday's note
djour yesterday

# List recent notes
djour list

# Compile all work-related entries
djour compile work
```

## Installation

### From Source

```bash
git clone https://github.com/yourusername/djour.git
cd djour
cargo build --release
cargo install --path .
```

The binary will be installed to `~/.cargo/bin/djour` (make sure this is in your PATH).

### Requirements

- Rust 1.70 or later
- A text editor (configured via `EDITOR` environment variable or djour config)

## Usage

### Initialize a Journal

Create a new journal in a directory:

```bash
# Initialize in current directory (default: daily mode)
djour init

# Initialize with a specific mode
djour init --mode weekly

# Initialize in a specific directory
djour init ~/my-journal --mode monthly
```

**Journal Modes:**
- `daily` - One file per day (`YYYY-MM-DD.md`)
- `weekly` - One file per ISO week (`YYYY-Www-YYYY-MM-DD.md`, week start date included)
- `monthly` - One file per month (`YYYY-MM.md`)
- `single` - All entries in one file (`journal.md`)

### Opening Notes

djour supports natural language time references:

```bash
# Simple references
djour today
djour yesterday
djour tomorrow

# Weekdays (most recent occurrence)
djour monday
djour friday

# Relative weekdays (previous/next occurrence)
djour last monday
djour next wednesday

# Specific dates (YYYY-MM-DD format)
djour 2025-01-17
```

**How it works:**
- If the note doesn't exist, djour creates it from a template
- Then opens it in your configured editor
- Your changes are automatically saved when you close the editor

**Mode-specific behavior:**

In `weekly` mode, all day references resolve to that week's file:
```bash
djour today       # Opens 2025-W03-2025-01-13.md (current week)
djour monday      # Opens 2025-W03-2025-01-13.md (week containing that Monday)
```

In `monthly` mode, all day references resolve to that month's file:
```bash
djour today       # Opens 2025-01.md
djour 2025-01-10  # Opens 2025-01.md
```

In `single` mode, all references open the same file:
```bash
djour today       # Opens journal.md
djour yesterday   # Opens journal.md (appends dated section)
```

### Tagging

Add tags to organize and find content across your notes.

#### Section-Level Tags

Tags in headings apply to all content until the next heading of equal or higher level:

```markdown
## Meeting Notes #work #project-alpha

Discussed the timeline for Q1 deliverables.
Action items assigned to the team.

### Follow-up Tasks

These tasks inherit #work and #project-alpha from the parent section.

## Personal Thoughts #personal

This section only has the #personal tag.
```

#### Paragraph-Level Tags

Tags at the end of a paragraph apply only to that paragraph:

```markdown
Had a great idea for the garden layout today. #garden #ideas

Unrelated thought about dinner plans.

Need to remember to call the dentist. #health #todo
```

**Tag Rules:**
- Tags start with `#` followed by alphanumeric characters, hyphens, or underscores
- Tags are case-insensitive (`#Work` = `#work`)
- Multiple tags can be applied: `#work #urgent #project-x`
- Section tags cascade to child sections
- Paragraph tags do not cascade

### Listing Notes

View your existing notes with filtering options:

```bash
# List recent notes (default: 10)
djour list

# List notes in a date range
djour list --from 01-01-2025 --to 31-01-2025

# Limit the number of results
djour list --limit 5
```

**Note:** Dates use DD-MM-YYYY format for the list command.

### Compilation

Compile all content matching a tag query into a single markdown file:

```bash
# Compile a single tag
djour compile work

# Compile with date filters
djour compile project-alpha --from 01-01-2025 --to 31-01-2025

# Custom output location
djour compile ideas --output ./reports/ideas.md

# Grouped format (by source file)
djour compile work --format grouped

# Include parent section context
djour compile work --include-context
```

**Boolean Tag Queries:**

```bash
# Both tags required
djour compile "work AND urgent"

# Either tag
djour compile "work OR personal"

# Exclude a tag
djour compile "work NOT meeting"

# Complex queries
djour compile "(work AND urgent) OR (personal AND important)"
```

**Compilation Formats:**

**Chronological (default)** - Entries in date order:
```markdown
# Compilation: #work

## 2025-01-15

### Meeting Notes #work #project-alpha
Discussed the timeline...

## 2025-01-16

### Sprint Planning #work
Assigned tasks for the week...
```

**Grouped** - Entries grouped by source file:
```markdown
# Compilation: #work

## From: 2025-01-15.md

### Meeting Notes #work #project-alpha
Discussed the timeline...

## From: 2025-01-16.md

### Sprint Planning #work
Assigned tasks for the week...
```

### Configuration

View or modify journal settings:

```bash
# Show current mode
djour config mode

# Change to weekly mode
djour config mode weekly

# Migrate from daily to weekly (converts existing notes, archives originals)
djour mode weekly --yes

# Preview what would change without modifying files
djour mode weekly --dry-run

# Set editor (overrides EDITOR environment variable)
djour config editor "code -w"

# List all configuration
djour config --list
```

**Configuration Keys:**
- `mode` - Journal mode (daily, weekly, monthly, single)
- `editor` - Text editor command
- `created` - Journal creation timestamp (read-only)

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `DJOUR_ROOT` | Default journal directory (overrides discovery) | `export DJOUR_ROOT=~/journal` |
| `DJOUR_MODE` | Override configured journal mode | `export DJOUR_MODE=weekly` |
| `EDITOR` | Primary text editor | `export EDITOR=vim` |
| `VISUAL` | Alternative text editor (fallback) | `export VISUAL=nano` |

**Editor Priority:**
1. `EDITOR` environment variable
2. `VISUAL` environment variable
3. Configured editor in `.djour/config.toml`
4. System default (`notepad` on Windows, `nano` on Unix)

## Custom Templates

Customize the structure of new notes by creating templates in `.djour/templates/`:

```bash
# Template files
.djour/templates/
├── daily.md      # Template for daily notes
├── weekly.md     # Template for weekly notes
├── monthly.md    # Template for monthly notes
└── entry.md      # Template for single mode entries
```

**Template Variables:**
- `{DATE}` - Full date (e.g., "January 17, 2025")
- `{ISO_DATE}` - ISO format (e.g., "2025-01-17")
- `{YEAR}` - Year (e.g., "2025")
- `{MONTH}` - Month name (e.g., "January")
- `{WEEK_NUMBER}` - ISO week number, zero-padded (e.g., "03")
- `{DAY_NAME}` - Day name (e.g., "Friday")

**Example custom daily template** (`.djour/templates/daily.md`):
```markdown
# {DATE}

## Goals
-

## Work Log

## Notes

## Reflections
```

If no custom template is found, djour falls back to built-in templates.

Note: mode migration (`djour mode ...`) currently supports only `daily <-> weekly` and requires
built-in templates (it will refuse to run if `.djour/templates/daily.md` or `.djour/templates/weekly.md`
exists).

## Examples

### Personal Journal Workflow

```bash
# Initialize personal journal
djour init ~/journal --mode daily

# Morning: start the day's notes
djour today

# Reference yesterday's notes
djour yesterday

# Weekly review
djour compile personal --from 13-01-2025 --to 19-01-2025
```

### Project Documentation

```bash
# Initialize project journal
djour init ./project-notes --mode single

# Add entries (appends new dated section)
djour today  # Opens journal.md

# Compile all design decisions
djour compile design-decision --output ./docs/decisions.md
```

### Research Notes

```bash
# Weekly research journal
djour init ~/research --mode weekly

# This week's notes
djour today

# Compile all literature references
djour compile literature --format grouped
```

### Work Log with Tags

```markdown
# 2025-01-17

## Sprint Planning #work #sprint

### Tasks Assigned #work #sprint #development
- Implement authentication system
- Write unit tests
- Code review for PR #123

## Team Meeting #work #meeting
Discussed Q1 roadmap. Key priorities:
1. Performance improvements
2. New feature launches

## Personal Learning #personal #learning
Started reading "Designing Data-Intensive Applications" #books
```

```bash
# Compile all work items
djour compile work --output work-log.md

# Compile only sprint-related work (not meetings)
djour compile "work AND sprint NOT meeting"
```

## Directory Structure

```
<journal-root>/
├── .djour/
│   ├── config.toml         # Configuration file
│   └── templates/          # Optional custom templates
│       ├── daily.md
│       ├── weekly.md
│       ├── monthly.md
│       └── entry.md
├── 2025-01-15.md          # Daily notes
├── 2025-01-16.md
├── 2025-W03.md            # Weekly notes
├── 2025-01.md             # Monthly notes
├── journal.md             # Single mode journal
└── compilations/          # Generated tag compilations
    ├── work.md
    └── personal.md
```

## Tips & Best Practices

### Consistent Tagging
- Use consistent tag names across notes
- Create a tag taxonomy early (e.g., `#work`, `#personal`, `#learning`)
- Combine tags for better organization (`#work #meeting`, `#personal #goals`)

### Weekly Reviews
```bash
# Compile last week's work
djour compile work --from 06-01-2025 --to 12-01-2025 --output weekly-review.md
```

### Quick Capture
```bash
# Set up an alias for quick access
alias jj='djour today'
```

### Backup Strategy
Your journal is just markdown files - use git, Dropbox, or any backup solution:
```bash
cd ~/journal
git init
git add .
git commit -m "Journal backup"
git push
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Linting and Formatting

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features

# Fix clippy warnings automatically
cargo clippy --fix
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

## Architecture

djour follows a clean layered architecture:

```
┌─────────────────┐
│   CLI Layer     │  Command parsing, output formatting
└────────┬────────┘
         │
┌────────▼────────┐
│  Application    │  Use cases, orchestration
└────────┬────────┘
         │
    ┌────┴────┐
┌───▼──┐  ┌──▼──────────┐
│Domain│  │Infrastructure│  File I/O, config, editor
└──────┘  └─────────────┘
```

**Key Design Principles:**
- Domain logic is testable without I/O
- Repository pattern abstracts file operations
- Clear separation of concerns
- Production-ready error handling

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Not a djour directory |
| 3 | Invalid date/time reference |
| 4 | Tag not found (for compile) |

## Troubleshooting

### "Not a djour directory" error

**Solution:**
```bash
# Run djour init in your journal directory
djour init

# Or set DJOUR_ROOT to point to an existing journal
export DJOUR_ROOT=~/journal
```

### Editor doesn't open

**Solution:**
```bash
# Set your editor
export EDITOR=vim

# Or configure it
djour config editor "code -w"

# On Windows, use full path if needed
djour config editor "C:\Program Files\Notepad++\notepad++.exe"
```

### Invalid time reference

**Solution:**
Valid formats:
- Simple: `today`, `yesterday`, `tomorrow`
- Weekdays: `monday`, `last friday`, `next wednesday`
- Dates: `YYYY-MM-DD` (e.g., `2025-01-17`)

### Tags not found in compilation

**Solution:**
- Check tag spelling (tags are case-insensitive)
- Ensure tags start with `#` in your notes
- Use `djour list` to verify notes exist
- Try a broader query: `"work OR personal"`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [clap](https://github.com/clap-rs/clap) for CLI parsing
- Uses [chrono](https://github.com/chronotope/chrono) for date handling
- Markdown parsing with [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark)
- Error handling with [thiserror](https://github.com/dtolnay/thiserror)

## Links

- [GitHub Repository](https://github.com/yourusername/djour)
- [Issue Tracker](https://github.com/yourusername/djour/issues)
- [Changelog](CHANGELOG.md)

---

**Made with ❤️ and Rust**
