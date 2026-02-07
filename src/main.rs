use chrono::NaiveDate;
use clap::Parser;
use djour::application::{
    init::InitService, manage_config::ConfigService, CompileOptions, CompileTagsService,
    ListNotesService, MigrateModeService, ModeMigrationOptions, OpenNoteService,
};
use djour::cli::{format_note_list, Cli, Commands};
use djour::domain::tags::CompilationFormat;
use djour::domain::JournalMode;
use djour::error::DjourError;
use djour::infrastructure::{FileSystemRepository, JournalRepository};
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
            InitService::execute(&path, journal_mode)
        }
        Some(Commands::Config { key, value, list }) => {
            // Discover repository
            let repo = FileSystemRepository::discover()?;
            let service = ConfigService::new(repo);

            if list {
                // List all config
                let config = service.list()?;
                println!("mode = {}", format!("{:?}", config.mode).to_lowercase());
                println!("editor = {}", config.editor);
                println!("created = {}", config.created.to_rfc3339());
                Ok(())
            } else if let Some(k) = key {
                if let Some(v) = value {
                    // Set config value
                    service.set(&k, &v)?;
                    println!("Set {} = {}", k, v);
                    Ok(())
                } else {
                    // Get config value
                    let val = service.get(&k)?;
                    println!("{}", val);
                    Ok(())
                }
            } else {
                // No key provided, show usage
                println!("Usage: djour config [--list | <key> [<value>]]");
                println!("Valid keys: mode, editor, created");
                Ok(())
            }
        }
        Some(Commands::List { from, to, limit }) => {
            // Discover repository
            let repo = FileSystemRepository::discover()?;
            let config = repo.load_config()?;

            // Parse date strings (DD-MM-YYYY format)
            let from_date = if let Some(s) = from {
                Some(NaiveDate::parse_from_str(&s, "%d-%m-%Y").map_err(|_| {
                    DjourError::Config(format!("Invalid date format: {}. Use DD-MM-YYYY", s))
                })?)
            } else {
                None
            };

            let to_date = if let Some(s) = to {
                Some(NaiveDate::parse_from_str(&s, "%d-%m-%Y").map_err(|_| {
                    DjourError::Config(format!("Invalid date format: {}. Use DD-MM-YYYY", s))
                })?)
            } else {
                None
            };

            // Execute list
            let service = ListNotesService::new(repo);
            let notes = service.execute(config.get_mode(), from_date, to_date, Some(limit))?;

            // Format and print output
            let output = format_note_list(&notes);
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
        }) => {
            // Discover repository
            let repo = FileSystemRepository::discover()?;

            // Parse date strings (DD-MM-YYYY format)
            let from_date = if let Some(s) = from {
                Some(NaiveDate::parse_from_str(&s, "%d-%m-%Y").map_err(|_| {
                    DjourError::Config(format!("Invalid date format: {}. Use DD-MM-YYYY", s))
                })?)
            } else {
                None
            };

            let to_date = if let Some(s) = to {
                Some(NaiveDate::parse_from_str(&s, "%d-%m-%Y").map_err(|_| {
                    DjourError::Config(format!("Invalid date format: {}. Use DD-MM-YYYY", s))
                })?)
            } else {
                None
            };

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
            };

            // Execute compilation
            let service = CompileTagsService::new(repo);
            let output_path = service.execute(options)?;

            println!("Compiled tags to: {}", output_path.to_string_lossy());

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
            let service = MigrateModeService::new(repo);

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

            service.execute(options)
        }
        None => {
            // Check if time_ref provided (open command)
            if let Some(time_ref) = cli.time_ref {
                // Resolve/create note and print filename
                let repo = FileSystemRepository::discover()?;
                let service = OpenNoteService::new(repo);
                let filename = service.execute(&time_ref, cli.open)?;
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
