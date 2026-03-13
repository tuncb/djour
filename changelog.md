# Changelog

## [v0.5.1]

- `compile` now groups output into two levels: section 1 uses the note's default generated header,
  based on the active mode/template; if no generated header is available, it falls back to a date or
  filename label, and section 2 comes from the nearest level-2 heading in the source note.

## [v0.4.2]

- File-producing commands now print full output paths.
- Journal discovery now checks the current folder first.

## [v0.4.1]

- Added the `folder` command to print the journal root path and support opening it in the configured editor.

## [v0.4.0]

- Added `tags` to list all tags found in notes.
- Added `--recursive` support for note traversal commands.
- Added `retag` to rename tags across notes.
- Removed the `--yes` option from the mode migration command.
- Refactored command implementation away from struct-based handlers toward free functions.

## [v0.3.1]

- Improved raw byte copying behavior.
- Improved link path handling during compilation.
- Removed parser-level post-processing in the tag compiler.
- Removed the old compatibility backend.

## [v0.3.0]

- `compile` now includes hashtags in generated output.
- Added raw section compilation support.
- Preserved markdown links and similar inline constructs during compilation.
- Default compilation output folder changed to `.compilations`.
- Tightened list item formatting for paragraph-based compilations.
- Removed the `created` field from the configuration file without backward compatibility.

## [v0.2.1]

- Release packaging was reduced to shipping only the executable artifact.

## [v0.2.0]

- Added synthetic end-to-end test coverage.
- Opening notes now requires the `--open` flag explicitly.
- `compile --open` opens the compiled file; without `--open`, the command prints the output filename.
- Simplified the README.
- Tests no longer fail because of environment-variable mismatches or unexpected extra output lines.

## [v0.1.6]

- Improved date range text in compiled weekly and monthly markdown output.

## [v0.1.5]

- Corrected `last <weekday>` and `next <weekday>` resolution so they choose previous and next occurrences consistently.
- Updated repository automation and contributor instructions.

## [v0.1.4]

- Added a note style conversion command.
- Fixed code fence handling for triple-backtick blocks.

## [v0.1.3]

- Fixed extra empty lines after compiled sections.
- Isolated tests from leaked environment variables such as `DJOUR_ROOT`.

## [v0.1.2]

- Updated weekly template dates.
- Updated the weekly filename format.
- Improved tag parsing in list items, sections, and lists.
- Added repository agent instructions.

## [v0.1.1]

- Coverage reporting no longer depends on a third-party service.

## [v0.1.0]

- Initial public release of `djour`.
- Established the core terminal journal workflow and tagged markdown compilation support.
- Fixed hashtag compilation in the first release line.
