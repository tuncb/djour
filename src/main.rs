use clap::Parser;
use djour::application::{init::InitService, manage_config::ConfigService, OpenNoteService};
use djour::cli::{Cli, Commands};
use djour::domain::JournalMode;
use djour::error::DjourError;
use djour::infrastructure::FileSystemRepository;
use std::str::FromStr;

fn main() {
    let cli = Cli::parse();

    let result = run(cli);

    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
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
        None => {
            // Check if time_ref provided (open command)
            if let Some(time_ref) = cli.time_ref {
                // Open note
                let repo = FileSystemRepository::discover()?;
                let service = OpenNoteService::new(repo);
                service.execute(&time_ref)?;
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
