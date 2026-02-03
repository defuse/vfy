mod cli;
mod compare;
mod stats;

use std::io::Write;
use std::process;
use std::sync::Arc;

use clap::Parser;

use cli::{Cli, Config};
use stats::Stats;

fn main() {
    let cli = Cli::parse();

    let config = match Config::from_cli(cli) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(2);
        }
    };

    if !config.original.is_dir() {
        eprintln!("Error: {:?} is not a directory", config.original);
        process::exit(2);
    }

    if !config.backup.is_dir() {
        eprintln!("Error: {:?} is not a directory", config.backup);
        process::exit(2);
    }

    if config.original == config.backup {
        eprintln!("Warning: original and backup are the same directory");
    }

    let stats = Arc::new(Stats::new());
    let stats_ctrlc = Arc::clone(&stats);

    ctrlc::set_handler(move || {
        eprintln!("\nInterrupted!");
        stats_ctrlc.print_summary();
        if let Err(e) = std::io::stdout().flush() {
            eprintln!("Warning: failed to flush stdout: {}", e);
        }
        process::exit(130);
    })
    .expect("Error setting Ctrl-C handler");

    compare::compare_dirs(&config, &stats);

    stats.print_summary();

    if stats.has_differences() {
        process::exit(1);
    }
}
