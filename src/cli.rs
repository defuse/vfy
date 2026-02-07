use clap::Parser;
use std::path::{Component, Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "vfy",
    about = "Verify backup integrity by comparing directory trees. By default, only compares file sizes.",
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
  SKIP:                          Entry skipped via --ignore or error/FS/type mismatch between sides
  ERROR:                         I/O or permission error
  DEBUG:                         Verbose logging (-v dirs, -vv files and hashes)
  SUMMARY:                       Final counts (not guaranteed to add up to 100%)

Symlink handling with --follow:
  When both sides are symlinks with different targets:
    - Reports DIFFERENT-SYMLINK-TARGET as a warning
    - Continues comparing resolved contents (may find similarities)

  When one side is a symlink and the other is a regular file/directory:
    - Reports DIFFERENT-SYMLINK-STATUS as structural mismatch
    - Reports original as MISSING-*, backup symlink as EXTRA-* (or vice-versa)
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

    /// Ignore one directory or file. Must exist. Ignoring one side also ignores the other.
    #[arg(short, long, verbatim_doc_comment)] // verbatim so it doesn't strip the period!
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
        //
        // We canonicalize the original and backup roots because:
        //
        //   - If the user compares to a/ (real dir) to b/ (symlink), we don't
        //     want that to be immediately reported as a difference.
        //   - For same-filesystem detection, we need to know where the root
        //     really is, i.e. that b/ symlink could be on a different device
        //     than its actual contents.
        //   - We can warn the user when they are comparing a directory to
        //     itself in a way that "sees through" different ways of arriving
        //     at the same actual path through symlinks.
        //   - Log messages contain the canonical paths of everything being
        //     compared, helping the user notice if what they are comparing is
        //     not what they expected due to symlink resolution.
        //
        // The ignore paths we provide to compare() must match what
        // compare_recursive walks, which is:
        //
        //      the canonicalized root / readdir entry names
        //
        // This means we do have to canonicalize *part* of the ignore path.
        // For example, if the user does...
        //
        //      original:   /a/this_is_a_symlink/original
        //          where /a/this_is_a_symlink -> a/actual_original
        //      ignore: /a/this_is_a_symlink/original/x
        //
        //  ...then the original path gets canonicalized to /a/actual_original
        //  but then an ignore path of /a/this_is_a_symlink/original/x will not
        //  match anything.
        //
        //  But we cannot canonicalize the *entire* --ignore path, because "x"
        //  could be a symlink to /other, so /a/this_is_a_symlink/original/x
        //  would get canonicalized to /other and again, nothing would be
        //  ignored.
        //
        // So, we:
        //
        //  1. Make the roots absolute and *normalize*.
        //  2. Make the ignore paths absolute and *normalize* them.
        //  3. Make sure each ignore path has a prefix which is one of the roots
        //     as typed or their canonicalizations.
        //  4. Strip off that prefix, and suffix what's left to the canonicalized root.
        //
        // This has the property that users must type the ignore path "the same
        // way" as they type the root paths OR the same way as the canonical
        // path. In other words, they cannot ignore something by providing a
        // path inside one of the roots that gets there a different way through
        // symlinks. If the user did that, the code below will report that the
        // ignore path is not within one of the roots.
        //
        // Also, if a user ignores original/x and x is a symlink to original/y,
        // then with --follow only original/x gets ignored, not original/y.
        //
        // Ignored paths are ALWAYS relative to the cwd, not relative to the
        // roots as is common in other backup tools. There is some potential for
        // confusion: if the user assumes the ignore paths are relative to the
        // root, and their cwd is deep inside a root, a path fragment collision
        // could lead to the wrong thing being ignored. But this seems unlikely.
        // If the cwd is one of the roots and the user assumes the path is
        // relative to the roots, then it "accidentally" works like the user
        // expects.
        //

        // Utility for making paths absolute and normalized (without canonicalizing).
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Cannot get current directory: {}", e))?;
        let make_absolute_and_normalized = |p: &Path| -> PathBuf {
            if p.is_absolute() { normalize_path(p) } else { normalize_path(&cwd.join(p)) }
        };

        let orig_as_typed = make_absolute_and_normalized(&cli.original);
        let backup_as_typed = make_absolute_and_normalized(&cli.backup);

        let mut ignore = Vec::new();

        for p in &cli.ignore {
            let normed = make_absolute_and_normalized(p);

            // Verify the ignored path exists (if a symlink, the symlink itself, not its target)
            if normed.symlink_metadata().is_err() {
                return Err(format!(
                    "Ignore path {:?} does not exist or cannot be resolved: No such file or directory",
                    p
                ));
            }

            // We don't check if ignored symlinks' targets exist, since the user
            // may intentionally be ignoring a dangling symlink.

            // Extract the within-tree suffix by stripping the root prefix.
            // Try the as-typed roots first (handles symlinks above the root),
            // then canonical roots (handles user typing the resolved path).
            // Rejoin suffix onto canonical root so it matches walked paths.

            // There are four allowed cases:
            // 1. The absolute ignore path starts with the user-specified original path.
            let stored = if let Ok(suffix) = normed.strip_prefix(&orig_as_typed) {
                original.join(suffix)
            // 2. The absolute ignore path starts with the canonicalized original path.
            } else if let Ok(suffix) = normed.strip_prefix(&original) {
                original.join(suffix)
            // 3. The absolute ignore path starts with the user-specified backup path.
            } else if let Ok(suffix) = normed.strip_prefix(&backup_as_typed) {
                backup.join(suffix)
            // 4. The absolute ignore path starts with the canonicalized backup path.
            } else if let Ok(suffix) = normed.strip_prefix(&backup) {
                backup.join(suffix)
            } else {
                return Err(format!(
                    "Ignore path {:?} is not within the original ({:?}) or backup ({:?}) directory",
                    normed, original, backup
                ));
            };
            ignore.push(stored);
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

fn normalize_path(path: &Path) -> PathBuf {
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
