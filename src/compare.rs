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

// ── Enums ────────────────────────────────────────────────────────────────────

/// Result of loading metadata for a path.
#[derive(Debug)]
enum Meta {
    Error(String),
    Dangling,
    Special,
    File(fs::Metadata),
    Dir(fs::Metadata, Vec<OsString>),
    Symlink,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Missing,
    Extra,
}

#[derive(Clone, Copy)]
enum EntryKind {
    File,
    Dir,
    Symlink,
}

impl Direction {
    fn prefix(self, kind: EntryKind) -> &'static str {
        match (self, kind) {
            (Direction::Missing, EntryKind::File) => "MISSING-FILE",
            (Direction::Missing, EntryKind::Dir) => "MISSING-DIR",
            (Direction::Missing, EntryKind::Symlink) => "MISSING-SYMLINK",
            (Direction::Extra, EntryKind::File) => "EXTRA-FILE",
            (Direction::Extra, EntryKind::Dir) => "EXTRA-DIR",
            (Direction::Extra, EntryKind::Symlink) => "EXTRA-SYMLINK",
        }
    }

    fn inc_count(self, stats: &Stats) {
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

impl Meta {
    fn classify(&self) -> EntryKind {
        match self {
            Meta::File(_) => EntryKind::File,
            Meta::Dir(_, _) => EntryKind::Dir,
            Meta::Symlink => EntryKind::Symlink,
            _ => unreachable!("classify called on {:?}", self),
        }
    }

    fn is_error_or_dangling(&self) -> bool {
        matches!(self, Meta::Error(_) | Meta::Dangling)
    }

    fn is_file_dir_or_symlink(&self) -> bool {
        matches!(self, Meta::File(_) | Meta::Dir(_, _) | Meta::Symlink)
    }
}

// ── Metadata loading ─────────────────────────────────────────────────────────

/// Load metadata for a path. All I/O happens here.
///
/// `follow=false`: uses symlink_metadata (default)
/// `follow=true`: uses metadata (follows symlinks), plus readdir for dirs.
///   Returns Dangling if the target doesn't exist (NotFound).
///
/// A directory that stats OK but can't be read is Error.
fn load_meta(path: &Path, follow: bool) -> Meta {
    let meta = if follow {
        match fs::metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Meta::Dangling,
            Err(e) => {
                return Meta::Error(format!("Cannot stat [{}]: {}", path.display(), e));
            }
        }
    } else {
        match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                return Meta::Error(format!("Cannot stat [{}]: {}", path.display(), e));
            }
        }
    };

    let ft = meta.file_type();

    // With follow=false, detect symlinks before anything else
    if !follow && ft.is_symlink() {
        return Meta::Symlink;
    }

    if ft.is_dir() {
        match read_dir_entries(path) {
            Ok(mut entries) => {
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
        Meta::Special
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn compare_dirs(config: &Config, stats: &Stats) {
    compare(&config.original, &config.backup, false, config, stats);
}

// ── compare ──────────────────────────────────────────────────────────────────

/// Compare two paths at the same relative position in both trees.
///
/// Pre: Neither path counted as an item yet.
/// Post: Both counted. Fully categorized. Descendants processed.
fn compare(
    orig: &Path,
    backup: &Path,
    follow: bool,
    config: &Config,
    stats: &Stats,
) {
    // Ignore check
    if config.ignore.iter().any(|ig| ig == orig || ig == backup) {
        println!("SKIP: [{}]", orig.display());
        stats.inc_skipped();
        return;
    }

    let meta_orig = load_meta(orig, follow);
    let meta_back = load_meta(backup, follow);

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

    if meta_orig.is_error_or_dangling() && meta_back.is_error_or_dangling() {
        return;
    }

    // --- Special files ---
    if matches!(meta_orig, Meta::Special) {
        stats.inc_original_items();
        println!("NOT_A_FILE_OR_DIR: [{}]", orig.display());
        stats.inc_not_a_file_or_dir();
    }

    if matches!(meta_back, Meta::Special) {
        stats.inc_backup_items();
        println!("NOT_A_FILE_OR_DIR: [{}]", backup.display());
        stats.inc_not_a_file_or_dir();
    }

    if matches!(meta_orig, Meta::Special) && matches!(meta_back, Meta::Special) {
        return;
    }

    // --- Same type: helpers count items ---
    match (&meta_orig, &meta_back) {
        (Meta::File(om), Meta::File(bm)) => {
            compare_files(orig, backup, om, bm, config, stats);
            return;
        }
        (Meta::Dir(_, _), Meta::Dir(_, _)) => {
            // Move entries out of meta
            let (om, oentries) = match meta_orig {
                Meta::Dir(m, e) => (m, e),
                _ => unreachable!(),
            };
            let (bm, bentries) = match meta_back {
                Meta::Dir(m, e) => (m, e),
                _ => unreachable!(),
            };
            compare_directories(orig, backup, &om, oentries, &bm, bentries, config, stats);
            return;
        }
        (Meta::Symlink, Meta::Symlink) => {
            compare_symlinks(orig, backup, config, stats);
            return;
        }
        _ => {}
    }

    // --- Type mismatch ---
    // Error/Dangling/Special sides already counted above.
    // File/Dir/Symlink sides counted by report() below.

    if (matches!(meta_orig, Meta::Symlink) && matches!(meta_back, Meta::File(_) | Meta::Dir(_, _)))
        || (matches!(meta_back, Meta::Symlink)
            && matches!(meta_orig, Meta::File(_) | Meta::Dir(_, _)))
    {
        println!(
            "DIFFERENT-SYMLINK-STATUS: [{}] (symlink mismatch)",
            orig.display()
        );
        stats.inc_different();
    } else if matches!(meta_orig, Meta::File(_)) && matches!(meta_back, Meta::Dir(_, _)) {
        println!("DIFFERENT-TYPE: [{}] (file vs dir)", orig.display());
        stats.inc_different();
    } else if matches!(meta_orig, Meta::Dir(_, _)) && matches!(meta_back, Meta::File(_)) {
        println!("DIFFERENT-TYPE: [{}] (dir vs file)", orig.display());
        stats.inc_different();
    }

    if meta_orig.is_file_dir_or_symlink() {
        report(orig, Direction::Missing, follow, true, config, stats);
    }
    if meta_back.is_file_dir_or_symlink() {
        report(backup, Direction::Extra, follow, true, config, stats);
    }
}

// ── compare_files ────────────────────────────────────────────────────────────

/// Compare two files.
///
/// Pre: Both are files. Neither counted. Metadata loaded.
/// Post: Both counted. Content compared.
fn compare_files(
    orig: &Path,
    backup: &Path,
    orig_meta: &fs::Metadata,
    backup_meta: &fs::Metadata,
    config: &Config,
    stats: &Stats,
) {
    stats.inc_original_items();
    stats.inc_backup_items();

    if config.verbosity >= Verbosity::Files {
        println!(
            "DEBUG: Comparing file [{}] to [{}]",
            orig.display(),
            backup.display()
        );
    }

    match compare_file_content(orig, backup, orig_meta, backup_meta, config, stats) {
        FileCompareResult::Different(r) => {
            println!("DIFFERENT-FILE [{}]: [{}]", r, orig.display());
            stats.inc_different();
        }
        FileCompareResult::Same => {
            stats.inc_similarities();
        }
        FileCompareResult::ErrorAlreadyReported => {}
    }
}

// ── compare_directories ──────────────────────────────────────────────────────

/// Compare two directories.
///
/// Pre: Both are dirs. Entries pre-loaded. Neither counted.
/// Post: Both counted. All children processed.
fn compare_directories(
    orig: &Path,
    backup: &Path,
    orig_meta: &fs::Metadata,
    orig_entries: Vec<OsString>,
    _backup_meta: &fs::Metadata,
    backup_entries: Vec<OsString>,
    config: &Config,
    stats: &Stats,
) {
    if config.verbosity >= Verbosity::Dirs {
        println!("DEBUG: Comparing [{}] to [{}]", orig.display(), backup.display());
    }

    // Check one-filesystem before counting
    if config.one_filesystem {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let parent = orig
                .parent()
                .expect("BUG: orig_path inside tree must have a parent");
            match fs::metadata(parent) {
                Ok(parent_meta) => {
                    if orig_meta.dev() != parent_meta.dev() {
                        stats.inc_original_items();
                        stats.inc_backup_items();
                        println!("DIFFERENT-FS: [{}]", orig.display());
                        stats.inc_skipped();
                        return;
                    }
                }
                Err(e) => {
                    stats.inc_original_items();
                    stats.inc_backup_items();
                    println!(
                        "ERROR: Cannot stat parent directory [{}]: {}",
                        parent.display(),
                        e
                    );
                    stats.inc_errors();
                    return;
                }
            }
        }
    }

    stats.inc_original_items();
    stats.inc_backup_items();
    stats.inc_similarities();

    let mut backup_set: HashSet<OsString> = backup_entries.into_iter().collect();

    for name in &orig_entries {
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
    let mut extras: Vec<OsString> = backup_set.into_iter().collect();
    extras.sort();

    for name in &extras {
        let backup_path = backup.join(name);
        report(&backup_path, Direction::Extra, false, true, config, stats);
    }
}

// ── compare_symlinks ─────────────────────────────────────────────────────────

/// Compare two symlinks.
///
/// Pre: Both are symlinks. Neither counted.
/// Post: Both counted. Targets compared. If --follow, resolved content
/// compared via compare.
fn compare_symlinks(
    orig: &Path,
    backup: &Path,
    config: &Config,
    stats: &Stats,
) {
    let orig_target = match fs::read_link(orig) {
        Ok(t) => t,
        Err(e) => {
            stats.inc_original_items();
            println!(
                "ERROR: Cannot read symlink target for [{}]: {}",
                orig.display(),
                e
            );
            stats.inc_errors();
            report(backup, Direction::Extra, false, true, config, stats);
            return;
        }
    };
    let backup_target = match fs::read_link(backup) {
        Ok(t) => t,
        Err(e) => {
            stats.inc_backup_items();
            println!(
                "ERROR: Cannot read symlink target for [{}]: {}",
                backup.display(),
                e
            );
            stats.inc_errors();
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
    }

    if !targets_differ {
        stats.inc_similarities();
    }

    if !config.follow {
        println!(
            "SYMLINK: [{}] (symlink, use --follow to compare content)",
            orig.display()
        );
        stats.inc_skipped();
        return;
    }

    // --follow: compare resolved content as additional items.
    // Symlinks are already counted above. The resolved content is
    // counted separately by compare (via its helpers or report).
    compare(orig, backup, true, config, stats);
}

// ── report ───────────────────────────────────────────────────────────────────

/// Report a path and all descendants as missing or extra.
///
/// Pre: Entry has NOT been counted as an item.
/// Post: Entry counted. Classified. Descendants processed.
fn report(
    path: &Path,
    direction: Direction,
    follow: bool,
    print: bool,
    config: &Config,
    stats: &Stats,
) {
    // Check ignore first (before any I/O)
    if config.ignore.iter().any(|ig| ig == path) {
        if print {
            println!("SKIP: [{}]", path.display());
        }
        stats.inc_skipped();
        return;
    }

    direction.inc_items(stats);

    let meta = load_meta(path, follow);

    match &meta {
        Meta::Error(msg) => {
            println!("ERROR: {}", msg);
            stats.inc_errors();
            return;
        }
        Meta::Dangling => {
            println!("DANGLING-SYMLINK: [{}]", path.display());
            stats.inc_errors();
            return;
        }
        _ => {}
    }

    if matches!(meta, Meta::Special) {
        if print {
            println!("NOT_A_FILE_OR_DIR: [{}]", path.display());
        }
        stats.inc_not_a_file_or_dir();
        return;
    }

    let kind = meta.classify();
    if print {
        println!("{}: [{}]", direction.prefix(kind), path.display());
    }
    direction.inc_count(stats);

    let print_children = config.verbosity >= Verbosity::Files;

    match &meta {
        Meta::Dir(_, entries) => {
            for name in entries {
                report(&path.join(name), direction, false, print_children, config, stats);
            }
        }
        Meta::Symlink if config.follow => {
            report(path, direction, true, print_children, config, stats);
        }
        Meta::File(_) | Meta::Symlink => {}
        _ => unreachable!("report: unexpected meta {:?} after classify", meta),
    }
}

// ── File content comparison ──────────────────────────────────────────────────

enum FileCompareResult {
    Same,
    Different(DiffReasons),
    /// An I/O error occurred; `compare_file_content` already called `inc_errors`
    /// for each failing side.
    ErrorAlreadyReported,
}

/// Compare two files by content. Reports errors for each side independently.
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
            let max_offset = if orig_size > sample_size {
                orig_size - sample_size
            } else {
                0
            };
            let offset = if max_offset > 0 {
                rng.gen_range(0..=max_offset)
            } else {
                0
            };

            let read_len = std::cmp::min(sample_size, orig_size) as usize;

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
                    return FileCompareResult::ErrorAlreadyReported;
                }
                (Ok(_), Err(e)) => {
                    println!(
                        "ERROR: Cannot read sample from [{}]: {}",
                        backup.display(),
                        e
                    );
                    stats.inc_errors();
                    return FileCompareResult::ErrorAlreadyReported;
                }
                (Err(e1), Err(e2)) => {
                    println!("ERROR: Cannot read sample from [{}]: {}", orig.display(), e1);
                    stats.inc_errors();
                    println!(
                        "ERROR: Cannot read sample from [{}]: {}",
                        backup.display(),
                        e2
                    );
                    stats.inc_errors();
                    return FileCompareResult::ErrorAlreadyReported;
                }
            }
        }
    }

    // BLAKE3 hash check — only if no prior mismatch
    if !reasons.any() && config.all {
        let (orig_result, backup_result) =
            rayon::join(|| hash_file(orig), || hash_file(backup));

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

        if orig_hash.is_none() || backup_hash.is_none() {
            return FileCompareResult::ErrorAlreadyReported;
        }

        let orig_hash = orig_hash.unwrap();
        let backup_hash = backup_hash.unwrap();

        if config.verbosity >= Verbosity::Files {
            println!("DEBUG: BLAKE3 {} [{}]", orig_hash.to_hex(), orig.display());
            println!(
                "DEBUG: BLAKE3 {} [{}]",
                backup_hash.to_hex(),
                backup.display()
            );
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

// ── Utilities ────────────────────────────────────────────────────────────────

fn read_dir_entries(dir: &Path) -> Result<Vec<OsString>, std::io::Error> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        entries.push(entry.file_name());
    }
    Ok(entries)
}

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
