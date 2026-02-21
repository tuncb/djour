//! CLI layer - Command-line interface

pub mod commands;
pub mod output;

pub use commands::{Cli, Commands};
pub use output::{format_note_list, format_tag_list};
