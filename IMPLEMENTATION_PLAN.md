# Implementation Plan: djour - Terminal Journal Application in Rust

## Overview
Implement the djour specification as a production-ready Rust CLI application with clean layered architecture. Focus on implementing the spec exactly without over-engineering for hypothetical future features.

## Architecture

### Layered Design
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

**Benefits:**
- Domain logic testable without I/O
- Clear separation of concerns
- Easy to mock infrastructure for testing
- Production-ready code quality

### Project Structure
```
djour/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, error handling
│   ├── lib.rs               # Library exports
│   ├── cli/                 # CLI Layer
│   │   ├── mod.rs
│   │   ├── commands.rs      # Clap command definitions
│   │   └── output.rs        # Output formatting
│   ├── application/         # Application Layer (Use Cases)
│   │   ├── mod.rs
│   │   ├── init.rs          # Initialize journal
│   │   ├── open_note.rs     # Open/create note
│   │   ├── list_notes.rs    # List notes
│   │   ├── manage_config.rs # Config get/set
│   │   └── compile_tags.rs  # Tag compilation
│   ├── domain/              # Domain Layer (Business Logic)
│   │   ├── mod.rs
│   │   ├── journal.rs       # Journal aggregate
│   │   ├── mode.rs          # JournalMode enum
│   │   ├── time_ref.rs      # TimeReference parsing
│   │   ├── tags/
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs    # Extract tags from markdown
│   │   │   ├── query.rs     # Tag query AST (AND/OR/NOT)
│   │   │   └── compiler.rs  # Compilation logic
│   │   └── template.rs      # Template rendering
│   ├── infrastructure/      # Infrastructure Layer (I/O)
│   │   ├── mod.rs
│   │   ├── repository.rs    # File operations
│   │   ├── config.rs        # Config load/save
│   │   └── editor.rs        # Editor spawning
│   └── error.rs             # Error types
└── tests/
    ├── integration/
    └── fixtures/
```

## Dependencies (Cargo.toml)

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
chrono = "0.4"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
thiserror = "1.0"
anyhow = "1.0"
pulldown-cmark = "0.11"
walkdir = "2.5"

[dev-dependencies]
tempfile = "3.10"
assert_cmd = "2.0"
predicates = "3.1"
```

## Critical Files to Implement

### 1. `src/domain/time_ref.rs` - Time Reference System
Core domain logic for parsing time references.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TimeReference {
    Today,
    Yesterday,
    Tomorrow,
    Weekday(Weekday),
    LastWeekday(Weekday),
    NextWeekday(Weekday),
    SpecificDate(NaiveDate),
}

impl TimeReference {
    pub fn parse(input: &str) -> Result<Self, TimeRefError>;
    pub fn resolve(&self, base_date: NaiveDate) -> NaiveDate;
}
```

**Key logic:**
- Parse strings like "today", "yesterday", "friday", "last monday", "2025-01-17"
- Resolve to actual NaiveDate based on base date
- Weekday resolution: "monday" = most recent Monday (or today if Monday)
- Fully testable without I/O

### 2. `src/domain/mode.rs` - Journal Modes
Mode-specific filename generation and date resolution.

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JournalMode {
    Daily,
    Weekly,
    Monthly,
    Single,
}

impl JournalMode {
    pub fn filename_for_date(&self, date: NaiveDate) -> String;
    pub fn resolve_date(&self, time_ref: &TimeReference, base: NaiveDate) -> NaiveDate;
}
```

**Behavior:**
- `Daily`: `YYYY-MM-DD.md`
- `Weekly`: `YYYY-Www.md` (use chrono's iso_week)
- `Monthly`: `YYYY-MM.md`
- `Single`: `journal.md`

### 3. `src/domain/tags/parser.rs` - Tag Extraction
Parse markdown and extract tagged content with inheritance.

```rust
pub struct TaggedContent {
    pub tags: Vec<String>,
    pub content: String,
    pub source_file: PathBuf,
    pub date: Option<NaiveDate>,
    pub context: TagContext,
}

pub enum TagContext {
    Section { heading: String, level: usize },
    Paragraph,
}

pub struct TagParser;

impl TagParser {
    pub fn extract_from_markdown(content: &str, source: &Path) -> Vec<TaggedContent>;
}
```

**Strategy:**
1. Use pulldown-cmark to parse markdown into events
2. Track section hierarchy (stack of current sections)
3. Extract tags from headings and paragraph endings
4. Implement tag inheritance: child sections inherit parent tags
5. Return tagged content blocks

### 4. `src/domain/tags/query.rs` - Tag Query System
Parse and evaluate boolean tag queries.

```rust
#[derive(Debug, Clone)]
pub enum TagQuery {
    Single(String),
    And(Box<TagQuery>, Box<TagQuery>),
    Or(Box<TagQuery>, Box<TagQuery>),
    Not(Box<TagQuery>),
}

impl TagQuery {
    pub fn parse(query: &str) -> Result<Self, TagQueryError>;
    pub fn matches(&self, tags: &[String]) -> bool;
}
```

**Parser logic:**
- "work" → Single("work")
- "work AND urgent" → And(Single("work"), Single("urgent"))
- "work OR personal" → Or(...)
- "work NOT meeting" → And(Single("work"), Not(Single("meeting")))

### 5. `src/infrastructure/repository.rs` - File Operations
Abstract file system operations behind a trait.

```rust
pub trait JournalRepository {
    fn load_config(&self) -> Result<Config>;
    fn save_config(&self, config: &Config) -> Result<()>;
    fn note_exists(&self, path: &Path) -> bool;
    fn read_note(&self, path: &Path) -> Result<String>;
    fn write_note(&self, path: &Path, content: &str) -> Result<()>;
    fn list_note_files(&self, mode: JournalMode) -> Result<Vec<PathBuf>>;
}

pub struct FileSystemRepository {
    root: PathBuf,
}
```

**Key operations:**
- Discover journal root (walk up directory tree for `.djour/`)
- TOML config read/write
- Safe file creation (atomic writes)
- Directory traversal with walkdir

### 6. `src/application/open_note.rs` - Open Note Use Case
Orchestrate opening a note (primary user workflow).

```rust
pub struct OpenNoteService {
    repository: Box<dyn JournalRepository>,
    journal: Journal,
    config: Config,
}

impl OpenNoteService {
    pub fn execute(&self, time_ref_str: &str) -> Result<()> {
        // 1. Parse time reference
        // 2. Resolve to filename based on mode
        // 3. Create file with template if needed
        // 4. Launch editor
    }
}
```

### 7. `src/application/compile_tags.rs` - Compilation Use Case
Compile tagged content across notes.

```rust
pub struct CompileService {
    repository: Box<dyn JournalRepository>,
    config: Config,
}

pub struct CompilationOptions {
    pub query: TagQuery,
    pub format: CompilationFormat,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub include_context: bool,
}

pub enum CompilationFormat {
    Chronological,
    Grouped,
}
```

**Process:**
1. List all note files (with optional date filter)
2. Parse each file and extract tagged content
3. Filter by tag query
4. Sort/group by format
5. Generate markdown output
6. Write to compilations/ directory

## Implementation Phases

### Phase 1: Foundation (Week 1)
**Goal:** Project setup and core domain types

Tasks:
1. Initialize Cargo project with dependencies
2. Set up module structure (cli/, domain/, application/, infrastructure/)
3. Implement `JournalMode` enum with filename generation
4. Implement `TimeReference` parsing and resolution
5. Write unit tests for time reference logic
6. Implement error types with exit codes

**Deliverable:** Time reference system fully working and tested

**Files:**
- `Cargo.toml`
- `src/lib.rs`, `src/main.rs`
- `src/domain/mode.rs`
- `src/domain/time_ref.rs`
- `src/error.rs`

### Phase 2: Configuration & Init (DETAILED PLAN)
**Goal:** Initialize journals and manage configuration

**Status:** Phase 1 Complete ✓ (JournalMode, TimeReference, Error types all implemented with 29+ tests)

---

## Step-by-Step Implementation Plan

### Step 1: Config Infrastructure (`src/infrastructure/config.rs`)

**Current State:** Type definition exists, no implementation

**Implement:**
```rust
impl Config {
    /// Load config from .djour/config.toml in the given directory
    pub fn load_from_dir(path: &Path) -> Result<Self>;

    /// Save config to .djour/config.toml in the given directory
    pub fn save_to_dir(&self, path: &Path) -> Result<()>;

    /// Get the editor command, checking environment variables first
    pub fn get_editor(&self) -> String;

    /// Create a new config with default values
    pub fn new(mode: JournalMode) -> Self;

    /// Detect default editor from environment or system
    fn detect_default_editor() -> String;
}
```

**Implementation Details:**
- `load_from_dir()`: Read `.djour/config.toml`, deserialize with `toml::from_str`
- `save_to_dir()`: Serialize with `toml::to_string_pretty`, write to `.djour/config.toml`
- `get_editor()`: Check `$EDITOR` env var → `$VISUAL` → config.editor → system default
- `detect_default_editor()`: Return "notepad" on Windows, "nano" on Unix
- `new()`: Create config with `created: Utc::now()`
- Handle errors and wrap in `DjourError::Config`

**Tests:**
- Load valid config file
- Load invalid TOML (should error)
- Save and reload config (round-trip)
- Editor detection with different env vars
- Missing config file (should error with NotDjourDirectory)

---

### Step 2: Repository Infrastructure (`src/infrastructure/repository.rs`)

**Current State:** Empty trait, bare struct

**Implement JournalRepository Trait:**
```rust
pub trait JournalRepository {
    /// Get the root directory of this repository
    fn root(&self) -> &Path;

    /// Load configuration from .djour/config.toml
    fn load_config(&self) -> Result<Config>;

    /// Save configuration to .djour/config.toml
    fn save_config(&self, config: &Config) -> Result<()>;

    /// Check if .djour directory exists
    fn is_initialized(&self) -> bool;

    /// Create .djour directory structure
    fn initialize(&self) -> Result<()>;
}
```

**Implement FileSystemRepository:**
```rust
impl FileSystemRepository {
    pub fn new(root: PathBuf) -> Self;

    /// Discover journal root by walking up directory tree
    pub fn discover() -> Result<Self>;

    /// Discover from a specific starting directory
    pub fn discover_from(start: &Path) -> Result<Self>;

    /// Check if a path contains a .djour directory
    fn has_djour_dir(path: &Path) -> bool;
}

impl JournalRepository for FileSystemRepository {
    // Implement all trait methods
}
```

**Implementation Details:**
- `discover()`: Start from `std::env::current_dir()`, walk up using `parent()`
- `discover_from()`: Walk up from given path until finding `.djour/` or reaching root
- `is_initialized()`: Check if `{root}/.djour/` exists
- `initialize()`: Create `.djour/` directory using `fs::create_dir()`
- `load_config()`: Delegate to `Config::load_from_dir()`
- `save_config()`: Delegate to `Config::save_to_dir()`
- Stop walking at filesystem root, return `NotDjourDirectory` error if not found
- Use `fs::canonicalize()` to get absolute paths

**Tests:**
- Discover journal root (with tempdir containing .djour)
- Discover fails when no .djour found
- Initialize creates .djour directory
- is_initialized checks correctly

---

### Step 3: Init Use Case (`src/application/init.rs`)

**Current State:** Empty placeholder

**Implement:**
```rust
pub struct InitService;

impl InitService {
    pub fn execute(path: &Path, mode: JournalMode) -> Result<()> {
        // 1. Resolve to absolute path
        // 2. Check if already initialized (error if .djour exists)
        // 3. Create directory if it doesn't exist
        // 4. Create .djour subdirectory
        // 5. Create default config
        // 6. Save config to .djour/config.toml
        // 7. Print success message
    }
}
```

**Implementation Details:**
- Use `fs::canonicalize()` to get absolute path
- Check if `{path}/.djour` already exists → error "Already initialized"
- Create `{path}` directory with `fs::create_dir_all()` if missing
- Create `{path}/.djour` directory with `fs::create_dir()`
- Create `Config::new(mode)` with current timestamp
- Save config using `Config::save_to_dir()`
- Return success or wrapped errors

**Tests:**
- Init new journal in empty directory
- Init in existing directory
- Init fails if already initialized
- Config file created with correct content
- Specified mode is saved correctly

---

### Step 4: Manage Config Use Case (`src/application/manage_config.rs`)

**Current State:** Empty placeholder

**Implement:**
```rust
pub struct ConfigService {
    repository: FileSystemRepository,
}

impl ConfigService {
    pub fn new(repository: FileSystemRepository) -> Self;

    /// Get a single config value
    pub fn get(&self, key: &str) -> Result<String>;

    /// Set a config value
    pub fn set(&self, key: &str, value: &str) -> Result<()>;

    /// List all config values
    pub fn list(&self) -> Result<Config>;
}
```

**Implementation Details:**
- `get()`: Load config, match key ("mode", "editor", "created"), return value as string
- `set()`: Load config, match key, update field, validate, save config
  - "mode": Parse string to JournalMode enum (daily/weekly/monthly/single)
  - "editor": Update editor field directly
  - "created": Reject with error (read-only field)
- `list()`: Load and return entire config
- Validation: Ensure mode is valid enum value
- Return error for unknown keys

**Tests:**
- Get mode, editor, created
- Set mode to valid value
- Set editor to custom command
- Set invalid mode (should error)
- Set unknown key (should error)
- List returns all config

---

### Step 5: CLI Commands (`src/cli/commands.rs`)

**Current State:** Empty Cli struct

**Implement:**
```rust
#[derive(Parser, Debug)]
#[command(name = "djour")]
#[command(about = "Terminal journal/notes application")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new journal
    Init {
        /// Directory to initialize (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Journal mode
        #[arg(short, long, default_value = "daily")]
        mode: String,
    },

    /// View or modify configuration
    Config {
        /// Config key to get or set
        key: Option<String>,

        /// Value to set (if provided, sets the key)
        value: Option<String>,

        /// List all configuration
        #[arg(short, long)]
        list: bool,
    },
}
```

**Implementation Details:**
- Use clap derive macros for clean syntax
- `init` defaults to current directory "."
- `init --mode` accepts string, validate to JournalMode enum in handler
- `config` with no args shows usage
- `config key` gets value
- `config key value` sets value
- `config --list` shows all config
- Return proper error messages for invalid inputs

---

### Step 6: Main Entry Point (`src/main.rs`)

**Update to dispatch commands:**
```rust
use clap::Parser;
use djour::cli::{Cli, Commands};
use djour::application::{init, manage_config};
use djour::infrastructure::FileSystemRepository;
use djour::domain::JournalMode;
use std::str::FromStr;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Init { path, mode }) => {
            // Parse mode string to enum
            let journal_mode = JournalMode::from_str(&mode)
                .map_err(|_| format!("Invalid mode: {}", mode))?;

            // Execute init
            init::InitService::execute(&path, journal_mode)
        }
        Some(Commands::Config { key, value, list }) => {
            // Discover repository
            let repo = FileSystemRepository::discover()?;
            let service = manage_config::ConfigService::new(repo);

            if list {
                let config = service.list()?;
                println!("{:#?}", config);
            } else if let Some(k) = key {
                if let Some(v) = value {
                    service.set(&k, &v)?;
                    println!("Set {} = {}", k, v);
                } else {
                    let val = service.get(&k)?;
                    println!("{}", val);
                }
            } else {
                println!("Usage: djour config [--list | <key> [<value>]]");
            }
            Ok(())
        }
        None => {
            println!("djour - Terminal journal application");
            println!("Use --help for usage information");
            Ok(())
        }
    };

    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(e.exit_code());
        }
    }
}
```

**Implementation Details:**
- Parse CLI with clap
- Match on command enum
- Handle each command with appropriate service
- Convert errors to exit codes
- Print helpful error messages to stderr
- Implement `FromStr` for `JournalMode` to parse mode strings

---

### Step 7: JournalMode FromStr Implementation

**Add to `src/domain/mode.rs`:**
```rust
use std::str::FromStr;

impl FromStr for JournalMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(JournalMode::Daily),
            "weekly" => Ok(JournalMode::Weekly),
            "monthly" => Ok(JournalMode::Monthly),
            "single" => Ok(JournalMode::Single),
            _ => Err(format!("Invalid mode: {}", s)),
        }
    }
}
```

**Test:**
- Parse "daily", "weekly", "monthly", "single"
- Parse uppercase variants "DAILY", "Weekly"
- Parse invalid mode returns error

---

### Step 8: Integration Tests

**Create `tests/integration/init_tests.rs`:**
```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_init_creates_config() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Check .djour directory exists
    assert!(temp.path().join(".djour").exists());

    // Check config.toml exists
    let config_path = temp.path().join(".djour/config.toml");
    assert!(config_path.exists());

    // Check config content
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("mode = \"daily\""));
}

#[test]
fn test_init_with_weekly_mode() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("weekly")
        .assert()
        .success();

    let config_path = temp.path().join(".djour/config.toml");
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("mode = \"weekly\""));
}

#[test]
fn test_init_already_initialized_fails() {
    let temp = TempDir::new().unwrap();

    // First init succeeds
    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Second init fails
    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .failure();
}

#[test]
fn test_config_get_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize
    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Get mode
    Command::cargo_bin("djour").unwrap()
        .current_dir(temp.path())
        .arg("config")
        .arg("mode")
        .assert()
        .success()
        .stdout(predicate::str::contains("daily"));
}

#[test]
fn test_config_set_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize
    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Set mode to weekly
    Command::cargo_bin("djour").unwrap()
        .current_dir(temp.path())
        .arg("config")
        .arg("mode")
        .arg("weekly")
        .assert()
        .success();

    // Verify it was set
    Command::cargo_bin("djour").unwrap()
        .current_dir(temp.path())
        .arg("config")
        .arg("mode")
        .assert()
        .success()
        .stdout(predicate::str::contains("weekly"));
}

#[test]
fn test_config_list() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    Command::cargo_bin("djour").unwrap()
        .current_dir(temp.path())
        .arg("config")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("mode"))
        .stdout(predicate::str::contains("editor"));
}
```

---

## Implementation Order

1. **JournalMode FromStr** (5 minutes) - Simple addition to existing file
2. **Config Infrastructure** (30 minutes) - load/save/get_editor/new methods
3. **Repository Infrastructure** (45 minutes) - Trait methods + FileSystemRepository impl
4. **Init Use Case** (20 minutes) - InitService implementation
5. **Manage Config Use Case** (30 minutes) - ConfigService get/set/list
6. **CLI Commands** (30 minutes) - Clap command definitions
7. **Main Entry Point** (30 minutes) - Command dispatch logic
8. **Integration Tests** (45 minutes) - 6 comprehensive tests
9. **Manual Testing** (30 minutes) - Verify with actual CLI usage

**Total Estimated Time:** ~4 hours

---

## Files to Modify/Create

**Modify:**
- `src/domain/mode.rs` - Add FromStr impl
- `src/infrastructure/config.rs` - Full implementation
- `src/infrastructure/repository.rs` - Full trait and impl
- `src/application/init.rs` - InitService
- `src/application/manage_config.rs` - ConfigService
- `src/cli/commands.rs` - Command definitions
- `src/main.rs` - Command dispatch

**Create:**
- `tests/integration/mod.rs` - Integration test module
- `tests/integration/init_tests.rs` - Init and config tests

---

## Verification Checklist

After implementation, verify:

1. **Init Command:**
   ```bash
   djour init ./test-journal
   # Should create .djour/config.toml with mode=daily

   djour init ./weekly-journal --mode weekly
   # Should create config with mode=weekly
   ```

2. **Config Command:**
   ```bash
   cd test-journal
   djour config mode
   # Should print: daily

   djour config mode weekly
   # Should update config

   djour config --list
   # Should show all config values

   djour config editor "code -w"
   # Should update editor
   ```

3. **Error Cases:**
   ```bash
   djour init ./test-journal
   # Second init should fail

   djour config mode invalid
   # Should error: Invalid mode

   cd /tmp
   djour config mode
   # Should error: Not a djour directory
   ```

4. **Tests:**
   ```bash
   cargo test
   # All unit tests pass (29 from Phase 1)

   cargo test --test init_tests
   # All 6 integration tests pass
   ```

**Deliverable:** Can initialize journals and manage configuration via CLI

### Phase 3: Open Notes (Week 3)
**Goal:** Create and open notes in editor

Tasks:
1. Implement built-in templates as constants
2. Implement template variable substitution
3. Implement editor detection and spawning
4. Complete `FileSystemRepository` (note operations)
5. Implement `OpenNoteService` use case
6. Build `open` command (default command, accepts time ref)
7. Write integration tests for open workflow

**Deliverable:** Full note creation and editing workflow

**Files:**
- `src/domain/template.rs`
- `src/infrastructure/editor.rs`
- `src/application/open_note.rs`
- Update `src/cli/commands.rs`

### Phase 4: List Notes (Week 4)
**Goal:** List and filter existing notes

Tasks:
1. Implement note file discovery
2. Implement date parsing from filenames (inverse of mode.filename_for_date)
3. Implement date range filtering
4. Implement `ListNotesService` use case
5. Build `list` command with options (--from, --to, --limit)
6. Format output nicely
7. Write tests for listing and filtering

**Deliverable:** Can list notes with filters

**Files:**
- `src/application/list_notes.rs`
- Update `src/infrastructure/repository.rs`
- Update `src/cli/commands.rs`, `src/cli/output.rs`

### Phase 5: Tag Parsing (Week 5)
**Goal:** Extract tags from markdown with inheritance

Tasks:
1. Implement markdown parsing with pulldown-cmark
2. Implement section hierarchy tracking
3. Implement tag extraction (heading and paragraph level)
4. Implement tag inheritance logic (children inherit parent tags)
5. Write extensive unit tests for tag extraction
6. Handle edge cases (nested sections, malformed tags)

**Deliverable:** Robust tag extraction system

**Files:**
- `src/domain/tags/parser.rs`
- `src/domain/tags/mod.rs`

### Phase 6: Tag Queries & Compilation (Week 6)
**Goal:** Compile tagged content with boolean queries

Tasks:
1. Implement tag query parser (AND, OR, NOT)
2. Implement query matching logic
3. Implement compilation formats (chronological, grouped)
4. Implement `CompileService` use case
5. Build `compile` command with all options
6. Generate markdown output
7. Write integration tests for compilation
8. Test multi-tag queries

**Deliverable:** Full tag compilation system

**Files:**
- `src/domain/tags/query.rs`
- `src/domain/tags/compiler.rs`
- `src/application/compile_tags.rs`
- Update `src/cli/commands.rs`

### Phase 7: Polish & Testing (Week 7)
**Goal:** Production-ready quality

Tasks:
1. Add custom template support (.djour/templates/)
2. Improve error messages with helpful context
3. Add comprehensive integration tests
4. Test all commands end-to-end
5. Add environment variable support (DJOUR_ROOT, EDITOR)
6. Write README with examples
7. Set up CI (cargo fmt, clippy, test)
8. Handle edge cases and error paths

**Deliverable:** Production-ready djour

## Error Handling Strategy

```rust
#[derive(Debug, thiserror::Error)]
pub enum DjourError {
    #[error("Not a djour directory: {0}")]
    NotDjourDirectory(PathBuf),

    #[error("Invalid time reference: {0}")]
    InvalidTimeReference(String),

    #[error("Tag not found: {0}")]
    TagNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl DjourError {
    pub fn exit_code(&self) -> i32 {
        match self {
            DjourError::NotDjourDirectory(_) => 2,
            DjourError::InvalidTimeReference(_) => 3,
            DjourError::TagNotFound(_) => 4,
            _ => 1,
        }
    }
}
```

Use `thiserror` for domain errors, `anyhow` for application layer.

## Testing Strategy

### Unit Tests
- Time reference parsing and resolution
- Tag extraction from markdown
- Tag query parsing and matching
- Mode filename generation
- Template variable substitution

### Integration Tests
```rust
// tests/integration/workflow_test.rs
#[test]
fn test_full_workflow() {
    let temp = TempDir::new().unwrap();

    // Init journal
    Command::cargo_bin("djour").unwrap()
        .arg("init")
        .arg(temp.path())
        .assert().success();

    // Create note (would open editor, skip in test)
    // Compile tags
    // Verify output
}
```

### Test Fixtures
Create sample journals in `tests/fixtures/` with pre-made notes for compilation testing.

## Key Design Decisions

### 1. Repository Pattern
- Abstract all file I/O behind `JournalRepository` trait
- Easy to mock for testing
- Could swap file system for database later (though not needed)

### 2. Time Reference as Domain Type
- Parse once into structured enum
- Resolution logic separate from parsing
- Fully testable without dates (use fixed base_date in tests)

### 3. Tag Inheritance via Section Stack
- Track current section hierarchy as stack
- When entering new section, push to stack with tags
- When leaving section (heading at same/higher level), pop stack
- Current tags = union of all tags in stack

### 4. Two-Phase Tag Processing
- Phase 1: Parse markdown structure (pulldown-cmark)
- Phase 2: Extract content with tags based on structure
- Cleaner separation of concerns

### 5. Template System
- Built-in templates as constants
- Custom templates override from `.djour/templates/`
- Simple variable substitution (no complex templating engine)

## Verification Plan

Test the implementation end-to-end:

1. **Initialize a journal**
   ```bash
   djour init ~/test-journal --mode daily
   cd ~/test-journal
   ```

2. **Open notes**
   ```bash
   djour today
   djour yesterday
   djour last friday
   ```

3. **Add tagged content** to notes manually:
   ```markdown
   ## Meeting Notes #work #project-alpha

   Discussed Q1 timeline.

   ## Ideas #personal

   Garden layout thoughts. #garden #ideas
   ```

4. **List notes**
   ```bash
   djour list
   djour list --from 2025-01-01 --to 2025-01-31
   ```

5. **Compile tags**
   ```bash
   djour compile work
   djour compile "work AND project-alpha"
   djour compile ideas --format grouped
   ```

6. **Config management**
   ```bash
   djour config mode weekly
   djour config editor "code -w"
   djour config --list
   ```

7. **Test different modes**
   - Create weekly journal, verify week file naming
   - Create monthly journal, verify month file naming
   - Create single journal, verify append behavior

## Non-Goals (Keep it Simple)

- ❌ Plugin system (not in spec)
- ❌ Database backend (files are fine)
- ❌ TUI/interactive mode (CLI only)
- ❌ Export to non-markdown formats (not in spec)
- ❌ Encryption (not in spec)
- ❌ Sync between machines (not in spec)

Focus on implementing the specification exactly as written with production-quality code.

## Summary

This plan provides:
- ✅ Clean layered architecture for maintainability
- ✅ Strong separation of concerns
- ✅ Highly testable design
- ✅ Production-ready error handling
- ✅ Pragmatic 7-week implementation timeline
- ✅ No over-engineering - just implement the spec

The architecture supports the spec's requirements while remaining simple and focused.
