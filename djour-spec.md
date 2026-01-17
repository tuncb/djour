# djour - Terminal Journal/Notes Application

A command-line note-taking application that manages markdown diary entries with support for multiple time-based formats and tag-based compilation.

## Overview

djour organizes markdown notes within a designated folder, handling file creation and organization while delegating editing to the user's preferred editor. Notes can be organized by day, week, month, or as a single continuous file.

## Core Concepts

### Journal Modes

| Mode | File Pattern | Description |
|------|--------------|-------------|
| `daily` | `YYYY-MM-DD.md` | One file per day |
| `weekly` | `YYYY-Www.md` | One file per ISO week (e.g., `2025-W03.md`) |
| `monthly` | `YYYY-MM.md` | One file per month |
| `single` | `journal.md` | All entries in one file |

### Directory Structure

```
<journal-root>/
├── .djour/
│   └── config.toml
├── 2025-01-15.md      # daily mode
├── 2025-01-16.md
├── 2025-W03.md        # weekly mode
├── 2025-01.md         # monthly mode
├── journal.md         # single mode
└── compilations/      # generated tag compilations
    └── work.md
```

---

## Commands

### Initialization

```bash
djour init [path] [--mode <mode>]
```

Initializes a directory for djour. Creates `.djour/config.toml`.

| Argument | Description |
|----------|-------------|
| `path` | Directory to initialize (default: `.`) |
| `--mode` | Journal mode: `daily`, `weekly`, `monthly`, `single` (default: `daily`) |

**Examples:**
```bash
djour init .                    # Initialize current directory with daily mode
djour init ~/notes --mode weekly
djour init ./work-journal --mode single
```

**Config file (`.djour/config.toml`):**
```toml
mode = "daily"
editor = "$EDITOR"  # Falls back to vim, nano, or notepad
created = "2025-01-17T10:30:00Z"
```

---

### Opening Notes

```bash
djour <timeref>
```

Opens the appropriate note file for the given time reference. Creates the file if it doesn't exist, then opens it in the configured editor.

#### Time References

| Reference | Description | Example File (daily mode) |
|-----------|-------------|---------------------------|
| `today` | Current day | `2025-01-17.md` |
| `yesterday` | Previous day | `2025-01-16.md` |
| `tomorrow` | Next day | `2025-01-18.md` |
| `monday` ... `sunday` | Most recent occurrence (or today if matching) | `2025-01-13.md` |
| `last monday` ... `last sunday` | Previous week's day | `2025-01-06.md` |
| `next monday` ... `next sunday` | Next week's day | `2025-01-20.md` |
| `YYYY-MM-DD` | Specific date | `2025-01-15.md` |
| `now` | Current time period (alias for `today`) | `2025-01-17.md` |

**Mode-specific behavior:**

In `weekly` mode, all day references resolve to that week's file:
```bash
djour today       # Opens 2025-W03.md (current week)
djour monday      # Opens 2025-W03.md (week containing that Monday)
djour 2025-01-10  # Opens 2025-W02.md
```

In `monthly` mode, all day references resolve to that month's file:
```bash
djour today       # Opens 2025-01.md
djour 2025-01-10  # Opens 2025-01.md
```

In `single` mode, all references open the same file:
```bash
djour today       # Opens journal.md
djour yesterday   # Opens journal.md
```

**Examples:**
```bash
djour today
djour yesterday
djour friday
djour last wednesday
djour next monday
djour 2025-01-10
```

---

### Listing Notes

```bash
djour list [--from <date>] [--to <date>] [--limit <n>]
```

Lists existing note files.

| Option | Description |
|--------|-------------|
| `--from` | Start date (inclusive) |
| `--to` | End date (inclusive) |
| `--limit` | Maximum entries to show (default: 10) |

**Examples:**
```bash
djour list
djour list --from 2025-01-01 --to 2025-01-15
djour list --limit 5
```

---

### Configuration

```bash
djour config [key] [value]
djour config --list
```

View or modify configuration.

**Examples:**
```bash
djour config mode              # Show current mode
djour config mode weekly       # Change to weekly mode
djour config editor "code -w"  # Set VS Code as editor
djour config --list            # Show all config
```

---

## Tagging System

djour supports hashtags for organizing and compiling related content across notes.

### Tag Syntax

Tags can be applied at two levels:

#### Section-Level Tags

Tags in headings apply to all content until the next heading of equal or higher level:

```markdown
## Meeting Notes #work #project-alpha

Discussed the timeline for Q1 deliverables.
Action items assigned to the team.

### Follow-up Tasks

These tasks inherit #work and #project-alpha from the parent section.

## Personal Thoughts #personal

This section only has #personal tag.
```

#### Paragraph-Level Tags

Tags at the end of a paragraph apply only to that paragraph:

```markdown
Had a great idea for the garden layout today. #garden #ideas

Unrelated thought about dinner plans.

Need to remember to call the dentist. #health #todo
```

### Tag Rules

- Tags start with `#` followed by alphanumeric characters, hyphens, or underscores
- Tags are case-insensitive (`#Work` = `#work`)
- Multiple tags can be applied: `#work #urgent #project-x`
- Section tags cascade to child sections
- Paragraph tags do not cascade

---

## Compilation

```bash
djour compile <tag> [options]
```

Compiles all content matching a tag into a new markdown file.

| Option | Description |
|--------|-------------|
| `--output`, `-o` | Output file path (default: `compilations/<tag>.md`) |
| `--from` | Start date filter |
| `--to` | End date filter |
| `--format` | Output format: `grouped`, `chronological` (default: `chronological`) |
| `--include-context` | Include parent section headings for context |

**Examples:**
```bash
djour compile work
djour compile project-alpha --from 2025-01-01 --output ./reports/alpha.md
djour compile ideas --format grouped
```

### Compilation Output Formats

#### Chronological (default)

Entries appear in date order:

```markdown
# Compilation: #work

## 2025-01-15

### Meeting Notes #work #project-alpha

Discussed the timeline...

## 2025-01-16

### Sprint Planning #work

Assigned tasks for the week...
```

#### Grouped

Entries grouped by source file:

```markdown
# Compilation: #work

## From: 2025-01-15.md

### Meeting Notes #work #project-alpha

Discussed the timeline...

## From: 2025-01-16.md

### Sprint Planning #work

Assigned tasks for the week...
```

### Multi-Tag Compilation

```bash
djour compile "work AND urgent"      # Both tags required
djour compile "work OR personal"     # Either tag
djour compile "work NOT meeting"     # Exclude tag
```

---

## File Templates

When creating new files, djour uses templates based on mode:

### Daily Template

```markdown
# 2025-01-17

## Morning


## Afternoon


## Evening

```

### Weekly Template

```markdown
# Week 3, 2025

## Monday


## Tuesday


## Wednesday


## Thursday


## Friday


## Weekend

```

### Monthly Template

```markdown
# January 2025

## Week 1


## Week 2


## Week 3


## Week 4

```

### Single Mode

Appends a dated section:

```markdown
---

# 2025-01-17

```

Templates can be customized via `.djour/templates/`:
- `daily.md`
- `weekly.md`
- `monthly.md`
- `entry.md` (for single mode appends)

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `DJOUR_ROOT` | Default journal directory |
| `EDITOR` | Preferred text editor |
| `DJOUR_MODE` | Override default mode |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Not a djour directory |
| 3 | Invalid date/time reference |
| 4 | Tag not found (for compile) |

---

## Examples

### Typical Daily Workflow

```bash
# Morning: start the day's notes
djour today

# Reference yesterday's notes
djour yesterday

# Quick weekly review
djour compile work --from "last monday" --to "last friday"
```

### Project Documentation

```bash
# Initialize project journal
djour init ./project-notes --mode single

# Add entries
djour today  # Opens journal.md, appends new dated section

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
