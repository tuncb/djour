# Mode Migration (Daily <-> Weekly) Plan

This document describes the implementation plan for a new `djour` command that changes the journal
mode **and migrates existing notes on disk** between `daily` and `weekly`.

## Goals (Scope Locked)

- Support **daily <-> weekly** only (no monthly/single in v1).
- Works only with **built-in templates**.
- If any custom template exists in `.djour/templates/` (daily.md or weekly.md), **error + abort**.
- **Strict validation** of file structure:
  - If section headings do not match the expected built-in headings, **error + abort**.
  - For weekly -> daily, if any non-whitespace exists outside the expected header + weekday sections,
    **error + abort**.
- Always **archive originals** (and back up modified targets).

Non-goals (v1):
- Migrating monthly/single.
- Trying to infer structure from custom templates.
- Best-effort migrations; this is strict by design.

## User-Facing Command (CLI Contract)

Add a new subcommand (keep `djour config mode` as config-only):

```bash
djour mode <daily|weekly> [--dry-run] [--archive-dir <path>] [--from <daily|weekly>]
```

Behavior:
- Default: print a migration plan and apply the migration.
- `--dry-run`: print the plan only (no writes).
- `--archive-dir`: optional override for the archive directory.
- `--from`: optional override for the detected current storage mode:
  - Default is `config.mode` from `.djour/config.toml`.
  - Ignore `DJOUR_MODE` env override for migration (it is a runtime override, not storage format).

## Archive + Safety Strategy

- Create an archive directory:
  - Default: `.djour/archive/mode-migration-<timestamp>/`
- Before modifying any existing target file, **copy** it to the archive.
- After successfully writing targets, **move** source files into the archive.
- Update `.djour/config.toml` mode **last** (also back up the prior config into archive).
- Use **atomic writes** for modified/created targets (write temp file, then rename).
- Add **idempotency markers** around injected blocks so rerunning after partial failure does not
  duplicate content.

## Migration: Daily -> Weekly (Inject)

### Preflight (No Writes)

- Refuse if `.djour/templates/daily.md` or `.djour/templates/weekly.md` exists.
- List notes using the current mode and parse dates from filenames.
- Group daily notes by their target weekly filename (computed via `JournalMode::Weekly`).
- For each weekly target:
  - If the weekly file exists:
    - Validate it has the expected built-in weekly structure for that week:
      - Expected weekly header line
      - 7 weekday `##` headings (Monday..Sunday), each exactly once, in order
    - Legacy weekly filenames (`YYYY-Www.md`) are ignored by migration.

### Transform (Strings)

- For each daily note:
  - Validate daily file header matches the built-in daily template for that date.
  - Extract the "body" after the header (content to inject).
- Ensure the weekly file exists:
  - If missing, create it from built-in weekly template for that week.
- Inject the daily body into the correct weekday section:
  - Place it under `## <Weekday>` heading corresponding to the day being migrated.
  - Surround injected content with idempotency markers like:
    - `<!-- djour:migrated-from=YYYY-MM-DD.md:start -->`
    - `<!-- djour:migrated-from=YYYY-MM-DD.md:end -->`

### Execute (Writes)

- Create archive dir.
- Back up any existing weekly targets that will be modified.
- Write updated weekly targets (atomic write).
- Move daily source files into the archive.
- Update config.mode to `weekly` last.

## Migration: Weekly -> Daily (Split)

### Preflight (No Writes)

- Refuse if `.djour/templates/daily.md` or `.djour/templates/weekly.md` exists.
- List weekly notes and parse week start date from filename.
- For each weekly note, validate strictly:
  - First non-empty line matches expected built-in weekly header for that week.
  - The 7 weekday headings exist exactly once, in order.
  - Only whitespace is allowed between header and Monday heading.
  - Only whitespace is allowed after Sunday section.
- Compute the 7 target daily filenames (week start + 0..6 days).
- If any target daily filename already exists, **abort** (v1 policy: no merging).

### Transform (Strings)

- Extract each weekday section body.
- Remove only migration marker lines (keep user content).
- For each day with non-empty body:
  - Create `YYYY-MM-DD.md` from built-in daily template for that date.
  - Append extracted weekday body.

### Execute (Writes)

- Create archive dir.
- Write all daily targets (atomic write).
- Move weekly source files into the archive.
- Update config.mode to `daily` last.

## Code Changes (Where)

- `src/cli/commands.rs`
  - Add `Commands::Mode { ... }`.
- `src/main.rs`
  - Dispatch to a new service.
- `src/application/migrate_mode.rs` (new)
  - Orchestrate: discover -> preflight plan -> execute -> config update.
- `src/domain/mode_migration.rs` (new)
  - Pure string/date logic:
    - Built-in template validators for daily/weekly
    - Weekly section slicer (split by weekday headings)
    - Injector for daily body into weekday section
    - Marker helpers
- `src/infrastructure/repository.rs`
  - Add filesystem helpers:
    - `copy_note(from, to)`
    - `move_note(from, to)`
    - `write_note_atomic(path, content)`
  - Unit tests for these ops.

## Known Problems / Risks

- Many existing journals will not exactly match built-in headings (strict mode will abort).
- Legacy weekly filename format (`YYYY-Www.md`) is ignored by migration (not converted).
- Windows file locking can fail moves/renames mid-migration:
  - Atomic writes + archive backups reduce corruption risk.
  - Idempotency markers reduce duplication risk on rerun.
- Tag parsing "inheritance bleed" can occur if concatenating notes naively:
  - This plan avoids concatenation and instead injects content inside the correct weekday section.

## Test Plan

Add `tests/mode_migration_tests.rs` (integration):
- Daily -> weekly happy path:
  - Weekly file created/updated and contains injected content under correct weekday section.
  - Daily sources moved to archive.
  - Config updated to weekly.
- Weekly -> daily happy path:
  - Daily files created from weekday sections.
  - Weekly sources moved to archive.
  - Config updated to daily.
- Refuses to run when `.djour/templates/weekly.md` exists.
- Aborts on missing/mismatched weekday headings.
- Aborts if any target daily file already exists (weekly -> daily).
