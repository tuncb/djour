use chrono::NaiveDate;
use clap::Parser;
use djour::application::{
    compile_tags, get_config, init, list_config, list_notes, list_tags, migrate_mode, open_note,
    retag_notes, set_config, CompileOptions, ModeMigrationOptions, RetagOptions,
};
use djour::cli::{format_note_list, format_tag_list, Cli, Commands};
use djour::domain::tags::CompilationFormat;
use djour::domain::JournalMode;
use djour::error::DjourError;
use djour::infrastructure::{EditorSession, FileSystemRepository, JournalRepository};
use std::str::FromStr;

fn main() {
    let cli = Cli::parse();

    let result = run(cli);

    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e.display_with_suggestions());
            std::process::exit(e.exit_code());
        }
    }
}

fn run(cli: Cli) -> Result<(), DjourError> {
    match cli.command {
        Some(Commands::Init { path, mode }) => {
            // Parse mode string to enum
            let journal_mode = JournalMode::from_str(&mode).map_err(DjourError::Config)?;

            // Execute init
            init(&path, journal_mode)
        }
        Some(Commands::Config { key, value, list }) => {
            // Discover repository
            let repo = FileSystemRepository::discover()?;

            if list {
                // List all config
                let config = list_config(&repo)?;
                println!("mode = {}", format!("{:?}", config.mode).to_lowercase());
                println!("editor = {}", config.editor);
                Ok(())
            } else if let Some(k) = key {
                if let Some(v) = value {
                    // Set config value
                    set_config(&repo, &k, &v)?;
                    println!("Set {} = {}", k, v);
                    Ok(())
                } else {
                    // Get config value
                    let val = get_config(&repo, &k)?;
                    println!("{}", val);
                    Ok(())
                }
            } else {
                // No key provided, show usage
                println!("Usage: djour config [--list | <key> [<value>]]");
                println!("Valid keys: mode, editor");
                Ok(())
            }
        }
        Some(Commands::List {
            from,
            to,
            limit,
            recursive,
        }) => {
            // Discover repository
            let repo = FileSystemRepository::discover()?;
            let config = repo.load_config()?;

            let from_date = parse_cli_date(from)?;
            let to_date = parse_cli_date(to)?;

            // Execute list
            let notes = list_notes(
                &repo,
                config.get_mode(),
                from_date,
                to_date,
                Some(limit),
                recursive,
            )?;

            // Format and print output
            let output = format_note_list(&notes);
            print!("{}", output);

            Ok(())
        }
        Some(Commands::Tags {
            from,
            to,
            recursive,
        }) => {
            let repo = FileSystemRepository::discover()?;
            let from_date = parse_cli_date(from)?;
            let to_date = parse_cli_date(to)?;

            let tags = list_tags(&repo, from_date, to_date, recursive)?;
            let output = format_tag_list(&tags);
            print!("{}", output);

            Ok(())
        }
        Some(Commands::Compile {
            query,
            output,
            from,
            to,
            format,
            include_context,
            open,
            recursive,
        }) => {
            // Discover repository
            let repo = FileSystemRepository::discover()?;

            let from_date = parse_cli_date(from)?;
            let to_date = parse_cli_date(to)?;

            // Parse format string
            let compilation_format = match format.to_lowercase().as_str() {
                "chronological" => CompilationFormat::Chronological,
                "grouped" => CompilationFormat::Grouped,
                _ => {
                    return Err(DjourError::Config(format!(
                        "Invalid format: {}. Use 'chronological' or 'grouped'",
                        format
                    )))
                }
            };

            // Create compile options
            let options = CompileOptions {
                query,
                output,
                from: from_date,
                to: to_date,
                format: compilation_format,
                include_context,
                recursive,
            };

            // Execute compilation
            let output_path = compile_tags(&repo, options)?;

            if open {
                let config = repo.load_config()?;
                let editor = EditorSession::new(config.get_editor());
                editor.open(&output_path)?;
            } else {
                let printable = output_path
                    .strip_prefix(repo.root())
                    .unwrap_or(&output_path)
                    .to_string_lossy();
                println!("{}", printable);
            }

            Ok(())
        }
        Some(Commands::Mode {
            to,
            from,
            dry_run,
            yes,
            archive_dir,
        }) => {
            let repo = FileSystemRepository::discover()?;
            eprintln!(
                "Warning: mode migration is non-recursive; --recursive is omitted for this command."
            );

            let to_mode = JournalMode::from_str(&to).map_err(DjourError::Config)?;
            let from_mode = match from {
                Some(s) => Some(JournalMode::from_str(&s).map_err(DjourError::Config)?),
                None => None,
            };

            let options = ModeMigrationOptions {
                to_mode,
                from_mode,
                dry_run,
                yes,
                archive_dir,
            };

            migrate_mode(&repo, options)
        }
        Some(Commands::Retag {
            from_tag,
            to_tag,
            from,
            to,
            recursive,
            dry_run,
        }) => {
            let repo = FileSystemRepository::discover()?;
            let from_date = parse_cli_date(from)?;
            let to_date = parse_cli_date(to)?;

            let options = RetagOptions {
                from_tag,
                to_tag,
                from: from_date,
                to: to_date,
                recursive,
                dry_run,
            };

            let report = retag_notes(&repo, options)?;
            if report.dry_run {
                println!(
                    "Dry run: {} file(s) would be updated with {} replacement(s).",
                    report.changed_files, report.total_replacements
                );
            } else {
                println!(
                    "Updated {} file(s) with {} replacement(s).",
                    report.changed_files, report.total_replacements
                );
            }

            for change in report.changes {
                println!("{} ({})", change.filename, change.replacements);
            }

            Ok(())
        }
        None => {
            // Check if time_ref provided (open command)
            if let Some(time_ref) = cli.time_ref {
                // Resolve/create note and print filename
                let repo = FileSystemRepository::discover()?;
                let filename = open_note(&repo, &time_ref, cli.open)?;
                println!("{}", filename);
                Ok(())
            } else {
                // No command and no time_ref, show help
                println!("djour - Terminal journal/notes application");
                println!("Use --help for usage information");
                Ok(())
            }
        }
    }
}

fn parse_cli_date(value: Option<String>) -> Result<Option<NaiveDate>, DjourError> {
    value
        .map(|s| {
            NaiveDate::parse_from_str(&s, "%d-%m-%Y").map_err(|_| {
                DjourError::Config(format!("Invalid date format: {}. Use DD-MM-YYYY", s))
            })
        })
        .transpose()
}
