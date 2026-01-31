# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Project Overview

**djour** is a terminal journal application written in Rust that manages markdown notes with support for multiple time-based formats (daily/weekly/monthly/single) and tag-based compilation with boolean queries.

## Essential Commands

### Building and Running
```bash
# Build debug version
cargo build

# Build release (optimized)
cargo build --release

# Run the CLI
cargo run -- <args>
# Examples:
cargo run -- init ./test-journal
cargo run -- today
cargo run -- compile work
```

### Testing
```bash
# Run all tests (unit + integration + doc tests)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test
cargo test test_time_reference_today

# Run test with output
cargo test test_name -- --nocapture

# Current test count: 199 tests (157 unit, 37 integration, 5 doc)
```

### Code Quality
```bash
# Format all code (REQUIRED before commits)
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check

# Run clippy with strict warnings
cargo clippy --all-targets --all-features -- -D warnings

# Auto-fix some clippy warnings
cargo clippy --fix
```

### Release Process
See RELEASE.md for details. Quick version:
```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
# GitHub Actions automatically builds and releases for 4 platforms
```

## Architecture

djour uses **clean layered architecture** with strict separation of concerns:

```
┌─────────────────┐
│   CLI Layer     │  Command parsing (clap), output formatting
│  src/cli/       │
└────────┬────────┘
         │
┌────────▼────────┐
│  Application    │  Use cases/orchestration (no business logic)
│ src/application/│  Services: InitService, OpenNoteService, etc.
└────────┬────────┘
         │
    ┌────┴────┐
┌───▼──┐  ┌──▼──────────┐
│Domain│  │Infrastructure│  File I/O, config, editor spawning
│src/  │  │src/infra-    │  Repository pattern for file operations
│domain│  │structure/    │
└──────┘  └─────────────┘
```

### Key Architectural Principles

1. **Domain logic is I/O-free and fully testable**
   - `src/domain/time_ref.rs` - Time reference parsing/resolution (pure logic)
   - `src/domain/tags/parser.rs` - Tag extraction from markdown AST
   - `src/domain/tags/query.rs` - Boolean query parsing/matching
   - `src/domain/mode.rs` - Mode-specific filename generation

2. **Repository pattern abstracts file operations**
   - `JournalRepository` trait in `src/infrastructure/repository.rs`
   - `FileSystemRepository` implementation
   - Discovery logic: walks up directory tree to find `.djour/`
   - **IMPORTANT**: When adding DJOUR_ROOT support, check environment variable FIRST before directory walking

3. **Application layer orchestrates use cases**
   - Each command has a dedicated service (InitService, OpenNoteService, CompileService, etc.)
   - Services coordinate between domain logic and infrastructure
   - No business logic in application layer

4. **Error handling with exit codes**
   - `DjourError` enum in `src/error.rs` with `thiserror`
   - Each error variant maps to specific exit code (0=success, 2=not djour dir, 3=invalid time ref, 4=tag not found)
   - `display_with_suggestions()` method provides helpful error messages with examples

## Critical Implementation Details

### Time Reference System (`src/domain/time_ref.rs`)
- Parse strings like "today", "friday", "last monday", "2025-01-17"
- **Resolution logic**: "monday" = most recent Monday (or today if today is Monday)
- All resolution based on a `base_date` parameter for testability
- Mode-specific behavior handled in `JournalMode::resolve_date()`

### Tag System (`src/domain/tags/`)
Two-level tagging with inheritance:

**Section-level tags** (in headings):
```markdown
## Work Notes #work #project-alpha
Content here inherits both tags.

### Subsection #urgent
Inherits #work and #project-alpha, adds #urgent
```

**Paragraph-level tags** (end of paragraph):
```markdown
Quick idea about the garden. #garden #ideas
This paragraph has two tags, no inheritance.
```

**Implementation**:
- Use `pulldown-cmark` to parse markdown into events
- Track section hierarchy as a stack
- Tags accumulate down the hierarchy
- Parser in `parser.rs`, query system in `query.rs`, compilation in `compiler.rs`

### Boolean Tag Queries (`src/domain/tags/query.rs`)
Parse and evaluate: "work AND urgent", "work OR personal", "work NOT meeting"

**Query AST**:
```rust
enum TagQuery {
    Single(String),           // "work"
    And(Box<Query>, Box<Query>),  // "work AND urgent"
    Or(Box<Query>, Box<Query>),   // "work OR personal"
    Not(Box<Query>),          // "NOT meeting"
}
```

**IMPORTANT**: Implement `Display` trait instead of inherent `to_string()` method (clippy warning).

### Configuration (`src/infrastructure/config.rs`)
Stored in `.djour/config.toml`:
```toml
mode = "daily"
editor = "vim"
created = "2025-01-17T10:30:00Z"
```

**Editor priority** (in `get_editor()` method):
1. `$EDITOR` environment variable
2. `$VISUAL` environment variable
3. `config.editor` from config file
4. System default (notepad on Windows, nano on Unix)

**Mode override** (in `get_mode()` method):
1. `$DJOUR_MODE` environment variable (if valid)
2. `config.mode` from config file

### Journal Modes (`src/domain/mode.rs`)
Each mode has specific filename patterns:
- `Daily`: `YYYY-MM-DD.md` (e.g., `2025-01-17.md`)
- `Weekly`: `YYYY-Www.md` using ISO week (e.g., `2025-W03.md`) - use `chrono::Datelike::iso_week()`
- `Monthly`: `YYYY-MM.md` (e.g., `2025-01.md`)
- `Single`: `journal.md` (always the same file)

## Common Development Patterns

### Adding a new command
1. Add variant to `Commands` enum in `src/cli/commands.rs`
2. Create service in `src/application/your_service.rs`
3. Add match arm in `src/main.rs` to dispatch to service
4. Write integration test in `tests/your_command_tests.rs`
5. Add `#![allow(deprecated)]` at top of test file (assert_cmd warnings)

### Adding a new domain type
1. Add type in appropriate `src/domain/` module
2. Keep I/O-free - use `&str`, `Path`, primitive types only
3. Write extensive unit tests in same file (`#[cfg(test)] mod tests`)
4. Make it `pub` in parent `mod.rs`

### Working with dates
- Use `chrono::NaiveDate` for dates without timezone
- Format dates: `date.format("%d-%m-%Y")` for display, `%Y-%m-%d` for ISO
- Parse dates: `NaiveDate::parse_from_str("15-01-2025", "%d-%m-%Y")`
- ISO week: `date.iso_week().week()` returns u32

### Error handling
- Return `Result<T>` from all fallible operations using project's `Result` type alias
- Use `?` operator to propagate errors
- Wrap errors: `std::io::Error` auto-converts via `#[from]` in thiserror
- Config errors: `DjourError::Config(format!("..."))`

## Testing Patterns

### Unit tests (domain layer)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = "test";

        // Act
        let result = parse(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration tests (CLI)
```rust
#![allow(deprecated)]  // REQUIRED at top of test file

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_workflow() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();
}
```

## Important Files

### Core Domain Logic (I/O-free, highly tested)
- `src/domain/time_ref.rs` - Time reference parsing and date resolution
- `src/domain/mode.rs` - Journal mode filename generation
- `src/domain/tags/query.rs` - Boolean query AST and matching
- `src/domain/tags/parser.rs` - Markdown tag extraction with inheritance
- `src/domain/tags/compiler.rs` - Tag compilation and output formatting

### Infrastructure (I/O operations)
- `src/infrastructure/repository.rs` - File system operations, `.djour/` discovery
- `src/infrastructure/config.rs` - TOML config load/save
- `src/infrastructure/editor.rs` - Editor process spawning

### Application Services (orchestration)
- `src/application/init.rs` - Initialize journal directory
- `src/application/open_note.rs` - Create/open note in editor
- `src/application/compile_tags.rs` - Tag compilation workflow
- `src/application/list_notes.rs` - List notes with filtering
- `src/application/manage_config.rs` - Config get/set operations

### Entry Points
- `src/main.rs` - CLI dispatch, error handling, exit codes
- `src/cli/commands.rs` - Clap command definitions

## Known Issues and Gotchas

1. **assert_cmd deprecation**: Test files need `#![allow(deprecated)]` at the top
2. **Display vs to_string**: Implement `Display` trait instead of inherent `to_string()` method
3. **Option::is_none_or**: Use `is_none_or(|x| ...)` instead of `map_or(true, |x| ...)`
4. **Environment variable testing**: Clean up env vars with `std::env::remove_var()` in tests to avoid race conditions
5. **Date format consistency**: Use DD-MM-YYYY for user-facing commands, YYYY-MM-DD for internal/ISO

## Specification and Planning

- **djour-spec.md** - Complete specification defining all features and behavior
- **IMPLEMENTATION_PLAN.md** - 7-phase implementation plan with detailed tasks
- **README.md** - User-facing documentation with examples

Current status: **Phase 7 complete** (all core features implemented, 199 tests passing, CI/CD configured).

## Dependencies

Key external crates:
- `clap` - CLI parsing with derive macros
- `chrono` - Date/time handling, ISO weeks
- `pulldown-cmark` - Markdown parsing for tag extraction
- `toml` - Config file serialization
- `thiserror` - Error type derivation
- `walkdir` - Directory traversal for note discovery

Dev dependencies:
- `tempfile` - Temporary directories for tests
- `assert_cmd` - CLI testing
- `predicates` - Assertion helpers
