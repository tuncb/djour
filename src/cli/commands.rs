//! CLI command definitions (to be implemented in Phase 2)

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "djour")]
#[command(about = "Terminal journal/notes application", long_about = None)]
pub struct Cli {
    // Commands will be added in Phase 2
}
