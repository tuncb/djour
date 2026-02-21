//! CLI command definitions

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "djour")]
#[command(about = "Terminal journal/notes application", long_about = None)]
#[command(version)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    /// Time reference (e.g., today, yesterday, last monday, 17-01-2025)
    #[arg(value_name = "TIME_REF")]
    pub time_ref: Option<String>,

    /// Open the selected note in configured editor
    #[arg(long, requires = "time_ref")]
    pub open: bool,

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

        /// Journal mode (daily, weekly, monthly, single)
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

    /// List existing notes
    List {
        /// Start date (inclusive, format: DD-MM-YYYY)
        #[arg(long)]
        from: Option<String>,

        /// End date (inclusive, format: DD-MM-YYYY)
        #[arg(long)]
        to: Option<String>,

        /// Maximum number of entries to show
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Search notes recursively (excluding directories that start with '.')
        #[arg(long)]
        recursive: bool,
    },

    /// Compile tagged content
    Compile {
        /// Tag query (e.g., "work", "work AND urgent", "work OR personal")
        query: String,

        /// Output file path (default: .compilations/<tag>.md)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Start date filter (format: DD-MM-YYYY)
        #[arg(long)]
        from: Option<String>,

        /// End date filter (format: DD-MM-YYYY)
        #[arg(long)]
        to: Option<String>,

        /// Output format: chronological, grouped
        #[arg(long, default_value = "chronological")]
        format: String,

        /// Include parent section headings for context
        #[arg(long)]
        include_context: bool,

        /// Open compiled file in configured editor
        #[arg(long)]
        open: bool,

        /// Search notes recursively (excluding directories that start with '.')
        #[arg(long)]
        recursive: bool,
    },

    /// List all tags used in notes
    Tags {
        /// Start date filter (inclusive, format: DD-MM-YYYY)
        #[arg(long)]
        from: Option<String>,

        /// End date filter (inclusive, format: DD-MM-YYYY)
        #[arg(long)]
        to: Option<String>,

        /// Search notes recursively (excluding directories that start with '.')
        #[arg(long)]
        recursive: bool,
    },

    /// Convert one tag to another across notes
    Retag {
        /// Source tag name (with or without leading #)
        from_tag: String,

        /// Destination tag name (with or without leading #)
        to_tag: String,

        /// Start date filter (inclusive, format: DD-MM-YYYY)
        #[arg(long)]
        from: Option<String>,

        /// End date filter (inclusive, format: DD-MM-YYYY)
        #[arg(long)]
        to: Option<String>,

        /// Search notes recursively (excluding directories that start with '.')
        #[arg(long)]
        recursive: bool,

        /// Show planned changes without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Change journal mode and migrate existing notes (daily <-> weekly)
    Mode {
        /// Target mode (daily or weekly)
        #[arg(value_name = "MODE")]
        to: String,

        /// Override detected current mode (daily or weekly)
        #[arg(long)]
        from: Option<String>,

        /// Show the migration plan without making changes
        #[arg(long)]
        dry_run: bool,

        /// Apply the migration (required unless --dry-run)
        #[arg(long)]
        yes: bool,

        /// Archive directory (relative to journal root)
        #[arg(long)]
        archive_dir: Option<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;

    #[test]
    fn parses_time_ref_without_open_flag() {
        let cli = Cli::try_parse_from(["djour", "today"]).unwrap();
        assert_eq!(cli.time_ref.as_deref(), Some("today"));
        assert!(!cli.open);
    }

    #[test]
    fn parses_open_flag_with_time_ref() {
        let cli = Cli::try_parse_from(["djour", "--open", "today"]).unwrap();
        assert_eq!(cli.time_ref.as_deref(), Some("today"));
        assert!(cli.open);
    }

    #[test]
    fn rejects_open_without_time_ref() {
        let result = Cli::try_parse_from(["djour", "--open"]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_open_with_subcommand() {
        let result = Cli::try_parse_from(["djour", "list", "--open"]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_compile_open_flag() {
        let cli = Cli::try_parse_from(["djour", "compile", "work", "--open"]).unwrap();
        match cli.command {
            Some(super::Commands::Compile {
                open, recursive, ..
            }) => {
                assert!(open);
                assert!(!recursive);
            }
            _ => panic!("Expected compile command"),
        }
    }

    #[test]
    fn parses_compile_recursive_flag() {
        let cli = Cli::try_parse_from(["djour", "compile", "work", "--recursive"]).unwrap();
        match cli.command {
            Some(super::Commands::Compile { recursive, .. }) => assert!(recursive),
            _ => panic!("Expected compile command"),
        }
    }

    #[test]
    fn parses_tags_command() {
        let cli = Cli::try_parse_from(["djour", "tags"]).unwrap();
        match cli.command {
            Some(super::Commands::Tags {
                from,
                to,
                recursive,
            }) => {
                assert!(from.is_none());
                assert!(to.is_none());
                assert!(!recursive);
            }
            _ => panic!("Expected tags command"),
        }
    }

    #[test]
    fn parses_tags_command_with_date_filters() {
        let cli = Cli::try_parse_from([
            "djour",
            "tags",
            "--from",
            "01-01-2025",
            "--to",
            "31-01-2025",
        ])
        .unwrap();

        match cli.command {
            Some(super::Commands::Tags {
                from,
                to,
                recursive,
            }) => {
                assert_eq!(from.as_deref(), Some("01-01-2025"));
                assert_eq!(to.as_deref(), Some("31-01-2025"));
                assert!(!recursive);
            }
            _ => panic!("Expected tags command"),
        }
    }

    #[test]
    fn parses_tags_recursive_flag() {
        let cli = Cli::try_parse_from(["djour", "tags", "--recursive"]).unwrap();
        match cli.command {
            Some(super::Commands::Tags { recursive, .. }) => {
                assert!(recursive);
            }
            _ => panic!("Expected tags command"),
        }
    }

    #[test]
    fn parses_list_recursive_flag() {
        let cli = Cli::try_parse_from(["djour", "list", "--recursive"]).unwrap();
        match cli.command {
            Some(super::Commands::List { recursive, .. }) => {
                assert!(recursive);
            }
            _ => panic!("Expected list command"),
        }
    }

    #[test]
    fn parses_retag_command_defaults() {
        let cli = Cli::try_parse_from(["djour", "retag", "work", "focus"]).unwrap();
        match cli.command {
            Some(super::Commands::Retag {
                from_tag,
                to_tag,
                from,
                to,
                recursive,
                dry_run,
            }) => {
                assert_eq!(from_tag, "work");
                assert_eq!(to_tag, "focus");
                assert!(from.is_none());
                assert!(to.is_none());
                assert!(!recursive);
                assert!(!dry_run);
            }
            _ => panic!("Expected retag command"),
        }
    }

    #[test]
    fn parses_retag_command_with_options() {
        let cli = Cli::try_parse_from([
            "djour",
            "retag",
            "#work",
            "project",
            "--from",
            "01-01-2025",
            "--to",
            "31-01-2025",
            "--recursive",
            "--dry-run",
        ])
        .unwrap();

        match cli.command {
            Some(super::Commands::Retag {
                from_tag,
                to_tag,
                from,
                to,
                recursive,
                dry_run,
            }) => {
                assert_eq!(from_tag, "#work");
                assert_eq!(to_tag, "project");
                assert_eq!(from.as_deref(), Some("01-01-2025"));
                assert_eq!(to.as_deref(), Some("31-01-2025"));
                assert!(recursive);
                assert!(dry_run);
            }
            _ => panic!("Expected retag command"),
        }
    }
}
