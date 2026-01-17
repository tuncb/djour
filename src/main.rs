use clap::Parser;
use djour::cli::Cli;

fn main() {
    let _cli = Cli::parse();

    // CLI command dispatch will be implemented in Phase 2
    println!("djour - Terminal journal application");
    println!("Commands will be available in Phase 2");

    std::process::exit(0);
}
