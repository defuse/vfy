use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "backup-verify", about = "Verify backup integrity by comparing directory trees")]
pub struct Cli {
    /// Original directory
    pub original: PathBuf,

    /// Backup directory
    pub backup: PathBuf,

    /// Verbose output (-v for dirs, -vv for files)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Number of random samples to compare per file
    #[arg(short, long, default_value_t = 0)]
    pub samples: u32,

    /// Full BLAKE3 hash comparison
    #[arg(long)]
    pub all: bool,

    /// Follow symlinks into directories
    #[arg(long)]
    pub follow: bool,

    /// Stay on one filesystem
    #[arg(long)]
    pub one_filesystem: bool,

    /// Directories to ignore (can be specified multiple times)
    #[arg(short, long)]
    pub ignore: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Dirs,
    Files,
}

pub struct Config {
    pub original: PathBuf,
    pub backup: PathBuf,
    pub verbosity: Verbosity,
    pub samples: u32,
    pub all: bool,
    pub follow: bool,
    pub one_filesystem: bool,
    pub ignore: Vec<PathBuf>,
}

impl Config {
    pub fn from_cli(cli: Cli) -> Result<Self, String> {
        let original = cli.original.canonicalize().map_err(|e| {
            format!("Cannot resolve original directory {:?}: {}", cli.original, e)
        })?;
        let backup = cli.backup.canonicalize().map_err(|e| {
            format!("Cannot resolve backup directory {:?}: {}", cli.backup, e)
        })?;

        let verbosity = match cli.verbose {
            0 => Verbosity::Quiet,
            1 => Verbosity::Dirs,
            _ => Verbosity::Files,
        };

        // Validate --ignore paths: must exist and be within original or backup tree
        let mut ignore = Vec::new();
        for p in &cli.ignore {
            let resolved = p.canonicalize().map_err(|e| {
                format!("Ignore path {:?} does not exist or cannot be resolved: {}", p, e)
            })?;
            if !resolved.starts_with(&original) && !resolved.starts_with(&backup) {
                return Err(format!(
                    "Ignore path {:?} is not within the original ({:?}) or backup ({:?}) directory",
                    resolved, original, backup
                ));
            }
            ignore.push(resolved);
        }

        #[cfg(not(unix))]
        if cli.one_filesystem {
            eprintln!("Warning: --one-filesystem is not supported on this platform and will be ignored");
        }

        Ok(Config {
            original,
            backup,
            verbosity,
            samples: cli.samples,
            all: cli.all,
            follow: cli.follow,
            one_filesystem: cli.one_filesystem,
            ignore,
        })
    }
}
