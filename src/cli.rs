use clap::Parser;
use std::path::{Component, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "vfy",
    about = "Verify backup integrity by comparing directory trees",
    arg_required_else_help = true,
    after_help = "\
WARNING: Output behavior is currently NOT STABLE between releases.
WARNING: This release has only been tested on Linux.

Verbosity levels:
  (default)  Show differences only. For missing/extra directories, only the
             top-level directory is listed; children are counted but not shown.
  -v         Add DEBUG lines showing each directory comparison.
  -vv        Add DEBUG lines for file comparisons. Show all individual entries
             inside missing/extra directories. With --all, show BLAKE3 hashes.

Output prefixes (grep-friendly):
  MISSING-FILE:                  File in original missing from backup
  MISSING-DIR:                   Directory in original missing from backup
  MISSING-SYMLINK:               Symlink in original missing from backup
  MISSING-SPECIAL:               Special file in original missing from backup
  MISSING-ERROR:                 Something (that errored) in original missing from backup
  EXTRA-FILE:                    File in backup not in original
  EXTRA-DIR:                     Directory in backup not in original
  EXTRA-SYMLINK:                 Symlink in backup not in original
  EXTRA-SPECIAL:                 Extra special file in backup not in original
  EXTRA-ERROR:                   Extra something (that errored) in backup not in original
  DIFFERENT-FILE [reason]:       File differs (reason: first mismatch of SIZE, SAMPLE, HASH)
  FILE-DIR-MISMATCH:             One side is a file, the other is a directory
  DIFFERENT-SYMLINK-TARGET:      Both sides are symlinks but point to different targets
  DIFFERENT-SYMLINK-STATUS:      One side is a symlink, the other is not
  SPECIAL-FILE:                  Entry is a device, FIFO, socket, etc.
  SYMLINK-SKIPPED:               Symlink skipped (use --follow to compare resolved content)
  DANGLING-SYMLINK:              Symlink target does not exist (with --follow)
  DIFFERENT-FS:                  Different filesystem skipped (--one-filesystem)
  SKIP:                          Entry skipped via --ignore
  ERROR:                         I/O or permission error
  DEBUG:                         Verbose logging (-v dirs, -vv files and hashes)
  SUMMARY:                       Final counts (not guaranteed to add up to 100%)

Symlink handling with --follow:
  When both sides are symlinks with different targets:
    - Reports DIFFERENT-SYMLINK-TARGET as a warning
    - Continues comparing resolved contents (may find similarities)

  When one side is a symlink and the other is a regular file/directory:
    - Reports DIFFERENT-SYMLINK-STATUS as structural mismatch
    - Reports original as MISSING-*, backup symlink as EXTRA-*
    - Does NOT compare contents (structural failure means no backup exists)

  Rationale: A symlink replacing a directory is a structural failure--the backup
  tree doesn't contain the actual data. Two symlinks with different targets is
  a metadata difference--the resolved data may still be equivalent."
)]

pub struct Cli {
    /// Original directory
    pub original: PathBuf,

    /// Backup directory
    pub backup: PathBuf,

    /// Verbose output (-v for dirs, -vv for files, hashes with --all, see below)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Number of random samples to compare per file
    #[arg(short, long, default_value_t = 0)]
    pub samples: u32,

    /// Full BLAKE3 hash comparison
    #[arg(short, long)]
    pub all: bool,

    /// Compare symlinked-to contents (symlink target paths are always compared, even without --follow)
    #[arg(short, long)]
    pub follow: bool,

    /// Stay on one filesystem (only supported on Unix-like OSes)
    #[cfg(unix)]
    #[arg(short = 'o', long)]
    pub one_filesystem: bool,

    /// Directories to ignore (can be specified multiple times). Must exist in original or backup.
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
    pub ignore: Vec<PathBuf>,
    /// Device ID of the original root directory (for --one-filesystem). Set to enforce staying on the same filesystem.
    #[cfg(unix)]
    pub original_device: Option<u64>,
    /// Device ID of the backup root directory (for --one-filesystem). Set to enforce staying on the same filesystem.
    #[cfg(unix)]
    pub backup_device: Option<u64>,
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
            2 => Verbosity::Files,
            n => return Err(format!("-v can be specified at most twice, but was specified {} times", n)),
        };

        // Validate --ignore paths: must exist and be within original or backup tree.
        // We must NOT resolve symlinks in the path — compare_recursive builds paths
        // by joining readdir names, so symlink directory names stay unresolved.
        // Strategy: make the path absolute (prepend cwd if relative), normalize
        // . and .. components, verify it exists, and check it's within the tree.
        let mut ignore = Vec::new();
        for p in &cli.ignore {
            let abs = if p.is_absolute() {
                p.clone()
            } else {
                std::env::current_dir()
                    .map_err(|e| format!("Cannot get current directory: {}", e))?
                    .join(p)
            };
            let resolved = normalize_path(&abs);
            // Verify the entry itself exists (the symlink or file, not its target)
            if resolved.symlink_metadata().is_err() {
                return Err(format!(
                    "Ignore path {:?} does not exist or cannot be resolved: No such file or directory",
                    p
                ));
            }
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

        // Get device IDs for --one-filesystem check
        #[cfg(unix)]
        let (original_device, backup_device) = if cli.one_filesystem {
            use std::os::unix::fs::MetadataExt;
            let orig_dev = std::fs::metadata(&original)
                .map_err(|e| format!("Cannot stat original directory {:?}: {}", original, e))?
                .dev();
            let backup_dev = std::fs::metadata(&backup)
                .map_err(|e| format!("Cannot stat backup directory {:?}: {}", backup, e))?
                .dev();
            (Some(orig_dev), Some(backup_dev))
        } else {
            (None, None)
        };

        Ok(Config {
            original,
            backup,
            verbosity,
            samples: cli.samples,
            all: cli.all,
            follow: cli.follow,
            ignore,
            #[cfg(unix)]
            original_device,
            #[cfg(unix)]
            backup_device,
        })
    }
}

fn normalize_path(path: &PathBuf) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {} // skip "."
            Component::ParentDir => {
                result.pop(); // go up for ".."
            }
            other => result.push(other),
        }
    }
    result
}
