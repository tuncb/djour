//! CLI command definitions

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "djour")]
#[command(about = "Terminal journal/notes application", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Time reference (e.g., today, yesterday, last monday, 2025-01-17)
    #[arg(value_name = "TIME_REF")]
    pub time_ref: Option<String>,

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
}
