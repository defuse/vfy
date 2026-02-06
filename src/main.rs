mod cli;
mod compare;
mod stats;

use std::process;
use std::sync::Arc;

use clap::Parser;

use cli::{Cli, Config};
use stats::Stats;

fn main() {
    // Replace the default panic hook to handle broken pipes cleanly.
    // Rust ignores SIGPIPE, so writing to a broken pipe (e.g. piping to
    // `head` or `grep`) causes println! to panic. Catch that and exit
    // with a visible message instead of a traceback.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        if msg.contains("Broken pipe") {
            eprintln!("Broken pipe: output was truncated");
            process::exit(141); // 128 + SIGPIPE(13)
        }
        default_hook(info);
    }));

    // Print the command-line we were run with
    let cmd: Vec<String> = std::env::args()
        .map(|a| {
            if a.contains(|c: char| c.is_whitespace() || "\"'\\$`!#&|;(){}[]<>?*~".contains(c)) {
                format!("'{}'", a.replace('\'', "'\\''"))
            } else {
                a
            }
        })
        .collect();
    println!("CMD: {}", cmd.join(" "));

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
        stats_ctrlc.eprint_summary();
        eprintln!("WARNING: EXITING BEFORE VERIFICATION WAS COMPLETE!");
        process::exit(130);
    })
    .expect("Error setting Ctrl-C handler");

    compare::compare_dirs(&config, &stats);

    stats.print_summary();

    if stats.has_differences_or_weirdness() {
        process::exit(1);
    }
}
