use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
use rand::Rng;

use crate::cli::{Config, Verbosity};
use crate::stats::{DiffReasons, Stats};

/// Result of loading metadata for a path.
#[derive(Debug, Clone)]
enum Meta {
    Error(String),
    Dangling,
    Special(fs::Metadata),
    File(fs::Metadata),
    Dir(fs::Metadata, Vec<OsString>),
    Symlink(fs::Metadata),
}

/// When reporting differences via report(), controls whether we output with
/// EXTRA- or MISSING- prefixes. Also used to influence behavior in certain
/// cases.
#[derive(Clone, Copy)]
enum Direction {
    Missing,
    Extra,
}

impl Meta {
    fn is_error_or_dangling(&self) -> bool {
        matches!(self, Meta::Error(_) | Meta::Dangling)
    }

    fn is_file_dir_or_symlink(&self) -> bool {
        matches!(self, Meta::File(_) | Meta::Dir(_, _) | Meta::Symlink(_))
    }
}

impl Direction {
    fn prefix(self, kind: &Meta) -> &'static str {
        match (self, kind) {
            (Direction::Missing, Meta::File(_)) => "MISSING-FILE",
            (Direction::Missing, Meta::Dir(_, _)) => "MISSING-DIR",
            (Direction::Missing, Meta::Symlink(_)) => "MISSING-SYMLINK",
            // This is unreachable because dangling symlinks are reported as
            // missing in the call to report() without follow=true, i.e. before
            // we ever try to resolve them.
            (Direction::Missing, Meta::Dangling) => "MISSING-SYMLINK",
            (Direction::Missing, Meta::Special(_)) => "MISSING-SPECIAL",
            (Direction::Missing, Meta::Error(_)) => "MISSING-ERROR",
            (Direction::Extra, Meta::File(_)) => "EXTRA-FILE",
            (Direction::Extra, Meta::Dir(_, _)) => "EXTRA-DIR",
            (Direction::Extra, Meta::Symlink(_)) => "EXTRA-SYMLINK",
            // This is unreachable because dangling symlinks are reported as
            // missing in the call to report() without follow=true, i.e. before
            // we ever try to resolve them.
            (Direction::Extra, Meta::Dangling) => "EXTRA-SYMLINK",
            (Direction::Extra, Meta::Special(_)) => "EXTRA-SPECIAL",
            (Direction::Extra, Meta::Error(_)) => "EXTRA-ERROR",
        }
    }

    fn inc_missing_or_extra_count(self, stats: &Stats) {
        match self {
            Direction::Missing => stats.inc_missing(),
            Direction::Extra => stats.inc_extras(),
        }
    }

    fn inc_items(self, stats: &Stats) {
        match self {
            Direction::Missing => stats.inc_original_items(),
            Direction::Extra => stats.inc_backup_items(),
        }
    }
}

// -- Metadata loading ---------------------------------------------------------

/// Load metadata for a path.
///
/// `follow=false`: uses symlink_metadata (default), symlinks returned as Meta::Symlink
/// `follow=true`: uses metadata (follows symlinks), returns Dangling if target doesn't exist
///
/// For directories (in either mode), also reads directory entries.
/// A directory that stats OK but can't be read returns Error.
fn load_meta(path: &Path, follow: bool) -> Meta {
    let meta = if follow {
        match fs::metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Meta::Dangling,
            Err(e) => {
                return Meta::Error(format!("Cannot read metadata for [{}]: {}", path.display(), e));
            }
        }
    } else {
        match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                return Meta::Error(format!("Cannot read metadata for [{}]: {}", path.display(), e));
            }
        }
    };

    let ft = meta.file_type();

    // With follow=false, detect symlinks before anything else
    assert!(!(follow & ft.is_symlink()));
    if !follow && ft.is_symlink() {
        return Meta::Symlink(meta);
    }

    if ft.is_dir() {
        match read_dir_entries(path) {
            Ok(mut entries) => {
                // Deterministic enumeration order.
                entries.sort();
                Meta::Dir(meta, entries)
            }
            Err(e) => {
                Meta::Error(format!("Cannot read directory [{}]: {}", path.display(), e))
            }
        }
    } else if ft.is_file() {
        Meta::File(meta)
    } else {
        // Block devices, char devices, FIFOs, sockets, etc.
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            if ft.is_block_device()
                || ft.is_char_device()
                || ft.is_fifo()
                || ft.is_socket()
            {
                Meta::Special(meta)
            } else {
                Meta::Error(format!("Unknown file type at [{}]", path.display()))
            }
        }
        #[cfg(not(unix))]
        {
            // For now, return an error so misbehavior on unsupported non-Unix
            // OSes is more obvious to the user.
            Meta::Error(format!("Unknown file type at [{}]", path.display()))
            // TODO: Meta::Special(meta), once we understand non-Unix behavior #29.
        }
    }
}

fn read_dir_entries(dir: &Path) -> Result<Vec<OsString>, std::io::Error> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        entries.push(entry.file_name());
    }
    Ok(entries)
}

// -- Entry point --------------------------------------------------------------

/// Compare directories according to the provided Config.
/// Populates stats with counts and prints log messages to stdout.
pub fn compare_dirs(config: &Config, stats: &Stats) {
    compare(&config.original, &config.backup, false, config, stats);
}

// -- Comparison ---------------------------------------------------------------

/// Compare two paths at the same relative position in both trees.
///
/// orig, backup, and ignore paths in the Config MUST be absolute paths for
/// ignore comparisons to work properly.
///
/// Pre: Neither path counted as an item yet. Both sides expected to exist.
/// Post: Both counted. Fully categorized. Descendants processed.
fn compare(
    orig: &Path,
    backup: &Path,
    follow: bool,
    config: &Config,
    stats: &Stats,
) {
    // Check ignore first so we don't encounter errors, different fs, or special files that the user ignored.
    if config.ignore.iter().any(|ig| ig == orig || ig == backup) {
        // One side may not exist, but be conservative and tell the user we're skipping both sides.
        println!("SKIP: [{}]", orig.display());
        stats.inc_skipped();
        println!("SKIP: [{}]", backup.display());
        stats.inc_skipped();
        return;
    }

    let meta_orig = load_meta(orig, follow);
    let meta_back = load_meta(backup, follow);

    // Check one-filesystem before any processing
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Some(root_dev) = config.original_device {
            match &meta_orig {
                Meta::Dir(m, _) | Meta::File(m) | Meta::Symlink(m) | Meta::Special(m) => {
                    if m.dev() != root_dev {
                        stats.inc_original_items();
                        stats.inc_backup_items(); // not strictly correct when the other side doesn't exist
                        println!("DIFFERENT-FS: [{}]", orig.display());
                        stats.inc_skipped();
                        // When original side is on a different FS, let the user know we skipped reporting the backup side.
                        println!("SKIP: [{}]", backup.display());
                        stats.inc_skipped();
                        return;
                    }
                }
                // No check for Dangling (already checked with follow=false one level up the stack) or Error
                Meta::Dangling | Meta::Error(_) => {}
            }
        }

        if let Some(root_dev) = config.backup_device {
            match &meta_back {
                Meta::Dir(m, _) | Meta::File(m) | Meta::Symlink(m) | Meta::Special(m) => {
                    if m.dev() != root_dev {
                        stats.inc_original_items(); // not strictly correct when the other side doesn't exist
                        stats.inc_backup_items();
                        println!("DIFFERENT-FS: [{}]", backup.display());
                        stats.inc_skipped();
                        // When backup is on a different FS, let the user know we skipped reporting the original side.
                        println!("SKIP: [{}]", orig.display());
                        stats.inc_skipped();
                        return;
                    }
                }
                Meta::Dangling | Meta::Error(_) => {}
            }
        }
    }

    // --- Errors / Dangling ---
    if meta_orig.is_error_or_dangling() {
        stats.inc_original_items();
        match &meta_orig {
            Meta::Error(msg) => {
                println!("ERROR: {}", msg);
                stats.inc_errors();
            }
            Meta::Dangling => {
                println!("DANGLING-SYMLINK: [{}]", orig.display());
                stats.inc_errors();
            }
            _ => unreachable!(),
        }
    }

    if meta_back.is_error_or_dangling() {
        stats.inc_backup_items();
        match &meta_back {
            Meta::Error(msg) => {
                println!("ERROR: {}", msg);
                stats.inc_errors();
            }
            Meta::Dangling => {
                println!("DANGLING-SYMLINK: [{}]", backup.display());
                stats.inc_errors();
            }
            _ => unreachable!(),
        }
    }

    // We can return early if BOTH are error/dangling, since that means both
    // sides have been reported above. Otherwise, when only one side is
    // error/dangling, we fall through to the difference reporting code below.
    if meta_orig.is_error_or_dangling() && meta_back.is_error_or_dangling() {
        return;
    }

    // --- Special files ---
    if matches!(meta_orig, Meta::Special(_)) {
        stats.inc_original_items();
        println!("SPECIAL-FILE: [{}]", orig.display());
        stats.inc_special_files();
    }

    if matches!(meta_back, Meta::Special(_)) {
        stats.inc_backup_items();
        println!("SPECIAL-FILE: [{}]", backup.display());
        stats.inc_special_files();
    }

    // Same pattern as above: we can exit early if we've already reported both sides.
    if matches!(meta_orig, Meta::Special(_)) && matches!(meta_back, Meta::Special(_)) {
        return;
    }

    // Same type on both sides: compare by type / recurse into directories
    match (&meta_orig, &meta_back) {
        (Meta::File(om), Meta::File(bm)) => {
            compare_files(orig, backup, om, bm, config, stats);
            return;
        }
        (Meta::Dir(_, _), Meta::Dir(_, _)) => {
            compare_directories(orig, backup, &meta_orig, &meta_back, config, stats);
            return;
        }
        (Meta::Symlink(_), Meta::Symlink(_)) => {
            compare_symlinks(orig, backup, config, stats);
            return;
        }
        _ => {}
    }

    // Control arrives here whenever there is some sort of mismatch between the
    // two sides. Either there is an explicit type mismatch, or one side errored.

    // We still need to report type mismatches for the other cases.

    match (&meta_orig, &meta_back) {
        // If any side is Error/Dangling/Special, that side has already been reported above.
        // Other side will be reported below.
        (_, Meta::Error(_) | Meta::Dangling | Meta::Special(_)) => {},
        (Meta::Error(_) | Meta::Dangling | Meta::Special(_), _) => {},
        // Symlink vs (File | Dir)
        // (File | Dir) vs Symlink
        (Meta::Symlink(_), Meta::File(_) | Meta::Dir(_, _)) |
        (Meta::File(_) | Meta::Dir(_, _), Meta::Symlink(_)) => {
            println!(
                "DIFFERENT-SYMLINK-STATUS: [{}] (symlink mismatch)",
                orig.display()
            );
            stats.inc_different();
        },
        // File vs Dir
        (Meta::File(_), Meta::Dir(_, _)) => {
            println!("FILE-DIR-MISMATCH: [{}] (file vs dir)", orig.display());
            stats.inc_different();
        },
        // Dir vs File
        (Meta::Dir(_, _), Meta::File(_)) => {
            println!("FILE-DIR-MISMATCH: [{}] (dir vs file)", orig.display());
            stats.inc_different();
        }
        // Same type handled above
        (Meta::File(_), Meta::File(_)) |
        (Meta::Dir(_, _), Meta::Dir(_, _)) |
        (Meta::Symlink(_), Meta::Symlink(_)) => unreachable!("Same-type cases handled above"),
    }

    // Control falls through when we should report differences.

    if meta_orig.is_file_dir_or_symlink() {
        // If we can't positively verify the backup exists, conservatively report missing.
        // We pass false so that we get a MISSING-SYMLINK *and* MISSING-FILE for resolving symlinks.
        report(orig, Direction::Missing, false, true, config, stats);
    }

    if meta_back.is_file_dir_or_symlink() {
        match meta_orig {
            // Error means we can't verify, don't suggest deletion of potentially
            // valid backup by calling it "EXTRA".
            Meta::Error(_) => {
                // Let the user know we are skipping the other side, though.
                println!("SKIP: [{}]", backup.display());
            },
            _ => {
                // TODO: See #24, reporting backup files as "extra" when the
                // backup has real files but the original contains some other
                // type may not be what we want to do, i.e. EXTRA might indicate
                // to the user they can safely delete files, when that's not
                // necesarily true.
                report(backup, Direction::Extra, false, true, config, stats);
            }
        }
    }
}

/// Compare two files.
///
/// Pre: Both are files. Neither counted. Metadata loaded.
/// Post: Both counted. Content compared. Log messages shown.
fn compare_files(
    orig: &Path,
    backup: &Path,
    orig_meta: &fs::Metadata,
    backup_meta: &fs::Metadata,
    config: &Config,
    stats: &Stats,
) {
    if config.verbosity >= Verbosity::Files {
        println!(
            "DEBUG: Comparing file [{}] to [{}]",
            orig.display(),
            backup.display()
        );
    }

    match compare_file_content(orig, backup, orig_meta, backup_meta, config, stats) {
        FileCompareResult::Different(r) => {
            stats.inc_original_items();
            stats.inc_backup_items();

            println!("DIFFERENT-FILE [{}]: [{}]", r, orig.display());
            stats.inc_different();
        }
        FileCompareResult::Same => {
            stats.inc_original_items();
            stats.inc_backup_items();

            stats.inc_similarities();
        }
        FileCompareResult::OrigError => {
            stats.inc_original_items();
            stats.inc_backup_items();

            // Can't read original - but don't report backup as "extra"
            // because it might convince the user it can safely be deleted.
            // Indicate that the error caused us to skip something.
            println!("SKIP: [{}]", backup.display());
            stats.inc_skipped();
        }
        FileCompareResult::BackupError => {
            stats.inc_backup_items();
            // report() assumes its input hasn't been counted (inc_original_items()) yet.
            // Can't read backup, be conservative and report original as missing
            report(orig, Direction::Missing, false, true, config, stats);
        }
        FileCompareResult::BothError => {
            // Both failed, errors already reported and counted
            stats.inc_original_items();
            stats.inc_backup_items();
        }
    }
}

/// Compare two directories.
///
/// Pre: Both are dirs. Entries pre-loaded. Neither counted.
/// Post: Both counted. All children processed. Logs output.
fn compare_directories(
    orig: &Path,
    backup: &Path,
    orig_meta: &Meta,
    backup_meta: &Meta,
    config: &Config,
    stats: &Stats,
) {
    if config.verbosity >= Verbosity::Dirs {
        println!("DEBUG: Comparing [{}] to [{}]", orig.display(), backup.display());
    }

    let orig_entries = match orig_meta {
        Meta::Dir(_, e) => e,
        _ => unreachable!()
    };

    let backup_entries = match backup_meta {
        Meta::Dir(_, e) => e,
        _ => unreachable!()
    };

    stats.inc_original_items();
    stats.inc_backup_items();
    // Both directories being present counts as a similarity, even if their contents differ
    stats.inc_similarities();

    let mut backup_set: HashSet<&OsString> = backup_entries.into_iter().collect();

    for name in orig_entries {
        let orig_path = orig.join(name);
        let backup_path = backup.join(name);

        let in_backup = backup_set.remove(name);
        if in_backup {
            compare(&orig_path, &backup_path, false, config, stats);
        } else {
            report(&orig_path, Direction::Missing, false, true, config, stats);
        }
    }

    // Remaining in backup_set are extras
    let mut extras: Vec<&OsString> = backup_set.into_iter().collect();
    extras.sort();

    for name in &extras {
        let backup_path = backup.join(name);
        report(&backup_path, Direction::Extra, false, true, config, stats);
    }
}

/// Compare two symlinks.
///
/// Pre: Both are symlinks. Neither counted.
/// Post: Both counted. Targets compared. If --follow, resolved content
/// compared via compare. Logs output.
fn compare_symlinks(
    orig: &Path,
    backup: &Path,
    config: &Config,
    stats: &Stats,
) {
    let orig_target = match fs::read_link(orig) {
        Ok(t) => t,
        Err(e) => {
            // Can't read original symlink - but don't report backup as "extra"
            // because it might be a valid backup. Safe/conservative behavior.
            stats.inc_original_items();
            stats.inc_backup_items();
            println!(
                "ERROR: Cannot read symlink target for [{}]: {}",
                orig.display(),
                e
            );
            stats.inc_errors();

            // But do report the backup side as skipped.
            println!("SKIP: [{}]", backup.display());
            stats.inc_skipped();

            return;
        }
    };
    let backup_target = match fs::read_link(backup) {
        Ok(t) => t,
        Err(e) => {
            stats.inc_backup_items();
            // report() does the inc_original_items()
            println!(
                "ERROR: Cannot read symlink target for [{}]: {}",
                backup.display(),
                e
            );
            stats.inc_errors();

            // Conservatively report the original side as missing, since we
            // can't compare due to the error.
            report(orig, Direction::Missing, false, true, config, stats);

            return;
        }
    };

    stats.inc_original_items();
    stats.inc_backup_items();

    let targets_differ = orig_target != backup_target;
    if targets_differ {
        println!(
            "DIFFERENT-SYMLINK-TARGET: [{}] (targets differ: {:?} vs {:?})",
            orig.display(),
            orig_target,
            backup_target
        );
        stats.inc_different();
    } else {
        stats.inc_similarities();
    }

    if config.follow {
        // --follow: compare resolved content as additional items.
        // Symlinks are already counted above. The resolved content is
        // counted separately by compare (via its helpers or report).
        compare(orig, backup, true, config, stats);
    } else {
        println!(
            "SYMLINK-SKIPPED: [{}] (use --follow to compare resolved content)",
            orig.display()
        );
        stats.inc_skipped();
    }
}

// -- report -------------------------------------------------------------------

/// Report a path and all descendants as missing or extra.
///
/// Pre: Entry has NOT been counted as an item. Callers other than report() itself must set follow=false.
/// Post: Entry counted. Classified. Descendants processed. Logs output.
fn report(
    path: &Path,
    direction: Direction,
    follow: bool,
    // Gets set to true for the top-level call, and false for recursive calls unless verbosity level is >= Files.
    print: bool,
    config: &Config,
    stats: &Stats,
) {
    // Check ignore first (before any I/O)
    // TODO (#35): --ignore paths should automatically apply to the other side
    // (i.e. to the backup, when ignoring a folder in original), but this is not
    // implemented yet.
    if config.ignore.iter().any(|ig| ig == path) {
        println!("SKIP: [{}]", path.display());
        stats.inc_skipped();
        return;
    }

    let meta = load_meta(path, follow);

    // Check --one-filesystem before processing entries on different filesystems.
    // This handles both mount points (follow=false) and resolved symlinks (follow=true).
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let root_dev = match direction {
            Direction::Missing => config.original_device,
            Direction::Extra => config.backup_device,
        };

        // Will be None when --one-filesystem is not enabled.
        if let Some(root_dev) = root_dev {
            let entry_dev = match &meta {
                Meta::Dir(m, _) | Meta::File(m) | Meta::Symlink(m) | Meta::Special(m) => {
                    Some(m.dev())
                }
                // We only see dangling with follow=true, and only report()
                // calls report() with follow=true, meaning the report() one
                // stack level up already did the FS check.
                Meta::Dangling | Meta::Error(_) => None,
            };

            if let Some(dev) = entry_dev {
                // Item is on a different device.
                if dev != root_dev {
                    println!("DIFFERENT-FS: [{}]", path.display());
                    stats.inc_skipped();

                    // We are in report() because the current item is missing from the other side.
                    // Even though it's on a different filesystem, we should still report it as missing/extra.
                    // For follow=true, the symlink itself was already reported by report() one stack level up.
                    if !follow {
                        direction.inc_items(stats);
                        if print {
                            println!("{}: [{}]", direction.prefix(&meta), path.display());
                        }
                        direction.inc_missing_or_extra_count(stats);
                    }

                    return;
                }
            }
        }
    }

    // We're not skipping it due to --ignore or DIFFERENT-FS, so count it.
    direction.inc_items(stats);

    match &meta {
        Meta::Error(msg) => {
            println!("ERROR: Error reporting: {}", msg);
            stats.inc_errors();
        }
        Meta::Dangling => {
            println!("DANGLING-SYMLINK: [{}]", path.display());
            stats.inc_errors();
        }
        Meta::Special(_) => {
            println!("SPECIAL-FILE: [{}]", path.display());
            stats.inc_special_files();
        },
        _ => {}
    }

    // If we have a dangling symlink, this means follow=true, and it's
    // already been reported as MISSING-SYMLINK by the report() one stack
    // level up. Only recursive calls to report() set follow=true.
    // Everything else gets reported as MISSING/EXTRA here.
    if !matches!(meta, Meta::Dangling) {
        if print {
            println!("{}: [{}]", direction.prefix(&meta), path.display());
        }
        direction.inc_missing_or_extra_count(stats);
    }

    // Recursive calls get print=true only if the verbosity level asks for it.
    // Note: With just one -v, we only report the top-level EXTRA-/MISSING-
    // directory, not subdirectories.
    let print_children = config.verbosity >= Verbosity::Files;

    match &meta {
        Meta::Dir(_, entries) => {
            for name in entries {
                report(&path.join(name), direction, false, print_children, config, stats);
            }
        }
        Meta::Symlink(_) if config.follow => {
            report(path, direction, true, print_children, config, stats);
        }
        // All of these leaf types have already been reported above.
        Meta::File(_) | Meta::Symlink(_) | Meta::Special(_) | Meta::Dangling | Meta::Error(_) => {}
    }
}

// -- File content comparison --------------------------------------------------

enum FileCompareResult {
    Same,
    Different(DiffReasons),
    /// Original file failed to read. Error has already been reported and counted.
    OrigError,
    /// Backup file failed to read. Error has already been reported and counted.
    BackupError,
    /// Both files failed to read. Error has already been reported and counted.
    BothError,
}

/// Compare two files by content. 
///
/// Reports errors for each side independently so that an error on one side
/// doesn't hide an error on the other side.
fn compare_file_content(
    orig: &Path,
    backup: &Path,
    orig_meta: &fs::Metadata,
    backup_meta: &fs::Metadata,
    config: &Config,
    stats: &Stats,
) -> FileCompareResult {
    let mut reasons = DiffReasons::default();

    let orig_size = orig_meta.len();
    let backup_size = backup_meta.len();

    if orig_size != backup_size {
        reasons.size = true;
    }

    // Sample check — only if sizes match and samples > 0
    if !reasons.any() && config.samples > 0 && orig_size > 0 {
        let mut rng = rand::thread_rng();
        let sample_size: u64 = 32;

        for _ in 0..config.samples {
            let max_offset = orig_size.saturating_sub(sample_size);
            let offset = rng.gen_range(0..=max_offset);
            let read_len = (orig_size - offset).min(sample_size) as usize;

            match (
                read_sample(orig, offset, read_len),
                read_sample(backup, offset, read_len),
            ) {
                (Ok(a), Ok(b)) => {
                    if a != b {
                        reasons.sample = true;
                        break;
                    }
                }
                (Err(e), Ok(_)) => {
                    println!("ERROR: Cannot read sample from [{}]: {}", orig.display(), e);
                    stats.inc_errors();
                    return FileCompareResult::OrigError;
                }
                (Ok(_), Err(e)) => {
                    println!("ERROR: Cannot read sample from [{}]: {}", backup.display(), e);
                    stats.inc_errors();
                    return FileCompareResult::BackupError;
                }
                (Err(e1), Err(e2)) => {
                    println!("ERROR: Cannot read sample from [{}]: {}", orig.display(), e1);
                    stats.inc_errors();
                    println!("ERROR: Cannot read sample from [{}]: {}", backup.display(), e2);
                    stats.inc_errors();
                    return FileCompareResult::BothError;
                }
            }
        }
    }

    // BLAKE3 hash check — only if no prior mismatch
    if !reasons.any() && config.all {
        let (orig_result, backup_result) = rayon::join(|| hash_file(orig), || hash_file(backup));

        let orig_hash = match orig_result {
            Ok(h) => Some(h),
            Err(e) => {
                println!("ERROR: Cannot hash [{}]: {}", orig.display(), e);
                stats.inc_errors();
                None
            }
        };
        let backup_hash = match backup_result {
            Ok(h) => Some(h),
            Err(e) => {
                println!("ERROR: Cannot hash [{}]: {}", backup.display(), e);
                stats.inc_errors();
                None
            }
        };

        let (orig_hash, backup_hash) = match (orig_hash, backup_hash) {
            (None, None) => return FileCompareResult::BothError,
            (None, Some(_)) => return FileCompareResult::OrigError,
            (Some(_), None) => return FileCompareResult::BackupError,
            (Some(o), Some(b)) => (o, b),
        };

        if config.verbosity >= Verbosity::Files {
            println!("DEBUG: BLAKE3 {} [{}]", orig_hash.to_hex(), orig.display());
            println!("DEBUG: BLAKE3 {} [{}]", backup_hash.to_hex(), backup.display());
        }

        if orig_hash != backup_hash {
            reasons.hash = true;
        }
    }

    if reasons.any() {
        FileCompareResult::Different(reasons)
    } else {
        FileCompareResult::Same
    }
}

/// The caller must ensure offset + len <= file size, otherwise hitting an EOF
/// will cause an error to be returned.
fn read_sample(path: &Path, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

fn hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_mmap_rayon(path)?;
    Ok(hasher.finalize())
}
