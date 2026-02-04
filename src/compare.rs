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

pub fn compare_dirs(config: &Config, stats: &Stats) {
    compare_recursive(&config.original, &config.backup, config, stats, true);
}

fn compare_recursive(orig_dir: &Path, backup_dir: &Path, config: &Config, stats: &Stats, is_root: bool) {
    if config.verbosity >= Verbosity::Dirs {
        println!("DEBUG: Comparing {} to {}", orig_dir.display(), backup_dir.display());
    }

    // Check ignore list against both original and backup paths
    if config.ignore.iter().any(|ig| ig == orig_dir || ig == backup_dir) {
        println!("SKIP: {}", orig_dir.display());
        stats.inc_skipped();
        return;
    }

    // Count this directory as a processed item. For the root, both original and
    // backup are counted here. For subdirectories, the parent loop already counted
    // them — only the similarity is deferred to here so the ignore check runs first.
    if is_root {
        stats.inc_original_items();
        stats.inc_backup_items();
    }
    stats.inc_similarities();

    let orig_entries = match read_dir_entries(orig_dir) {
        Ok(entries) => entries,
        Err(e) => {
            println!("ERROR: Cannot read directory {}: {}", orig_dir.display(), e);
            stats.inc_errors();
            return;
        }
    };

    let backup_entries = match read_dir_entries(backup_dir) {
        Ok(entries) => entries,
        Err(e) => {
            println!("ERROR: Cannot read directory {}: {}", backup_dir.display(), e);
            stats.inc_errors();
            return;
        }
    };

    let mut backup_set: HashSet<OsString> = backup_entries.iter().cloned().collect();

    let mut orig_entries = orig_entries;
    orig_entries.sort();

    for name in &orig_entries {
        let orig_path = orig_dir.join(name);
        let backup_path = backup_dir.join(name);

        // Check ignore list at the entry level (catches missing/extra dirs too)
        if config.ignore.iter().any(|ig| ig == &orig_path || ig == &backup_path) {
            println!("SKIP: {}", orig_path.display());
            stats.inc_skipped();
            if backup_set.contains(name) {
                // Remove from backup_set so it's not counted as an extra
                backup_set.remove(name);
            }
            continue;
        }

        stats.inc_original_items();

        let in_backup = backup_set.contains(name);
        if in_backup {
            stats.inc_backup_items();
        }

        let orig_meta = match fs::symlink_metadata(&orig_path) {
            Ok(m) => m,
            Err(e) => {
                println!("ERROR: Cannot stat {}: {}", orig_path.display(), e);
                stats.inc_errors();
                continue;
            }
        };

        let orig_is_symlink = orig_meta.file_type().is_symlink();

        if in_backup {
            handle_both_present(
                &orig_path, &backup_path, &orig_meta, orig_is_symlink, config, stats,
            );
        } else {
            handle_missing(&orig_path, &orig_meta, orig_is_symlink, config, stats);
        }
    }

    // Extras: entries in backup but not in original
    let orig_set: HashSet<OsString> = orig_entries.iter().cloned().collect();
    let mut extras: Vec<&OsString> = backup_set.difference(&orig_set).collect();
    extras.sort();

    for name in extras {
        let backup_path = backup_dir.join(name);
        handle_extra(&backup_path, config, stats);
    }
}

// ── Both sides present ──────────────────────────────────────────────────────

/// Both original and backup contain this entry.
///
/// Decision order (see docs/symlink-handling.md):
///   1. Special files (device/FIFO/socket) → NOT_A_FILE_OR_DIR
///   2. Neither is a symlink → compare as dir/file
///   3. One is a symlink, the other is not → DIFFERENT-SYMLINK-STATUS
///   4. Both are symlinks → handle_both_symlinks
fn handle_both_present(
    orig_path: &Path,
    backup_path: &Path,
    orig_meta: &fs::Metadata,
    orig_is_symlink: bool,
    config: &Config,
    stats: &Stats,
) {
    let backup_meta = match fs::symlink_metadata(backup_path) {
        Ok(m) => m,
        Err(e) => {
            println!("ERROR: Cannot stat {}: {}", backup_path.display(), e);
            stats.inc_errors();
            return;
        }
    };
    let backup_is_symlink = backup_meta.file_type().is_symlink();

    // Special files take priority over everything
    let orig_special = is_special(orig_meta, orig_is_symlink);
    let backup_special = is_special(&backup_meta, backup_is_symlink);
    if orig_special || backup_special {
        if orig_special {
            println!("NOT_A_FILE_OR_DIR: {}", orig_path.display());
            stats.inc_not_a_file_or_dir();
        }
        if backup_special {
            println!("NOT_A_FILE_OR_DIR: {}", backup_path.display());
            stats.inc_not_a_file_or_dir();
        }
        // The non-special side's content is effectively missing/extra
        if !orig_special {
            report_missing(orig_path, orig_meta, orig_is_symlink, config, stats);
        }
        if !backup_special {
            report_extra(backup_path, &backup_meta, backup_is_symlink, config, stats);
        }
        return;
    }

    // Neither side is a symlink → compare as dir/file
    if !orig_is_symlink && !backup_is_symlink {
        compare_entries(orig_path, backup_path, orig_meta, &backup_meta, config, stats);
        return;
    }

    // One side is a symlink, the other is not
    if orig_is_symlink != backup_is_symlink {
        println!("DIFFERENT-SYMLINK-STATUS: {} (symlink mismatch)", orig_path.display());
        stats.inc_different();
        // The non-symlink side's content is effectively missing/extra
        if !orig_is_symlink {
            report_missing(orig_path, orig_meta, false, config, stats);
        }
        if !backup_is_symlink {
            report_extra(backup_path, &backup_meta, false, config, stats);
        }
        return;
    }

    // Both are symlinks
    handle_both_symlinks(orig_path, backup_path, config, stats);
}

/// True if the entry is a special file (device, FIFO, socket) — not a regular
/// file, directory, or symlink.
fn is_special(meta: &fs::Metadata, is_symlink: bool) -> bool {
    !is_symlink && !meta.is_file() && !meta.is_dir()
}

// ── Both symlinks ───────────────────────────────────────────────────────────

/// Both sides are symlinks. Compare targets, then either follow or skip.
fn handle_both_symlinks(
    orig_path: &Path,
    backup_path: &Path,
    config: &Config,
    stats: &Stats,
) {
    // 1. Compare targets (always, regardless of --follow)
    let orig_target = match fs::read_link(orig_path) {
        Ok(t) => t,
        Err(e) => {
            println!("ERROR: Cannot read symlink target for {}: {}", orig_path.display(), e);
            stats.inc_errors();
            return;
        }
    };
    let backup_target = match fs::read_link(backup_path) {
        Ok(t) => t,
        Err(e) => {
            println!("ERROR: Cannot read symlink target for {}: {}", backup_path.display(), e);
            stats.inc_errors();
            return;
        }
    };

    let targets_differ = orig_target != backup_target;
    if targets_differ {
        println!(
            "DIFFERENT-SYMLINK-TARGET: {} (targets differ: {:?} vs {:?})",
            orig_path.display(), orig_target, backup_target
        );
        stats.inc_different();
    }

    // 2. With --follow: resolve and compare content
    if config.follow {
        follow_symlinks(orig_path, backup_path, targets_differ, config, stats);
    } else {
        // 3. Without --follow: content was not verified.
        //    If targets match, it's a similarity (same symlink) but still skipped
        //    (content behind it wasn't compared).
        println!(
            "SYMLINK: {} (symlink, use --follow to compare content)",
            orig_path.display()
        );
        if !targets_differ {
            stats.inc_similarities();
        }
        stats.inc_skipped();
    }
}

/// Resolve both symlinks and compare their content. Handles dangling symlinks.
fn follow_symlinks(
    orig_path: &Path,
    backup_path: &Path,
    targets_differ: bool,
    config: &Config,
    stats: &Stats,
) {
    let orig_meta = resolve_symlink(orig_path, stats);
    let backup_meta = resolve_symlink(backup_path, stats);

    // If either resolution hit a non-NotFound error, bail (already reported)
    let orig_meta = match orig_meta {
        Some(m) => m,
        None => return,
    };
    let backup_meta = match backup_meta {
        Some(m) => m,
        None => return,
    };

    let orig_dangling = orig_meta.is_none();
    let backup_dangling = backup_meta.is_none();

    if orig_dangling || backup_dangling {
        // Report which side(s) are dangling
        if orig_dangling {
            println!("DANGLING-SYMLINK: {}", orig_path.display());
            stats.inc_errors();
        }
        if backup_dangling {
            println!("DANGLING-SYMLINK: {}", backup_path.display());
            stats.inc_errors();
        }
        if !targets_differ {
            // The symlinks are identical (same target), so count as a similarity.
            stats.inc_similarities();
        }

        // If only one side is dangling, the resolved side's content is missing/extra
        if let (Some(om), None) = (&orig_meta, &backup_meta) {
            if om.is_dir() {
                count_recursive(orig_path, config, stats, Direction::Missing);
            }
        }
        if let (None, Some(bm)) = (&orig_meta, &backup_meta) {
            if bm.is_dir() {
                count_recursive(backup_path, config, stats, Direction::Extra);
            }
        }
        return;
    }

    // Both resolved — compare using dir/file/special logic
    let orig_meta = orig_meta.unwrap();
    let backup_meta = backup_meta.unwrap();
    compare_entries(orig_path, backup_path, &orig_meta, &backup_meta, config, stats);
}

/// Resolve a symlink's target via fs::metadata (follows symlinks).
/// Returns Some(Some(metadata)) if resolved, Some(None) if dangling (NotFound),
/// or None if a non-recoverable error occurred (already reported).
fn resolve_symlink(path: &Path, stats: &Stats) -> Option<Option<fs::Metadata>> {
    match fs::metadata(path) {
        Ok(m) => Some(Some(m)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(None),
        Err(e) => {
            println!("ERROR: Cannot resolve symlink {}: {}", path.display(), e);
            stats.inc_errors();
            None
        }
    }
}

// ── Dir/file comparison (shared by non-symlink and followed-symlink paths) ──

/// Compare two entries by their resolved metadata (dir vs dir, file vs file, etc.).
/// Used both for non-symlink entries and for resolved symlink targets with --follow.
fn compare_entries(
    orig_path: &Path,
    backup_path: &Path,
    orig_meta: &fs::Metadata,
    backup_meta: &fs::Metadata,
    config: &Config,
    stats: &Stats,
) {
    // Special files (symlink targets that resolve to devices, FIFOs, etc.)
    let orig_is_regular = orig_meta.is_file() || orig_meta.is_dir();
    let backup_is_regular = backup_meta.is_file() || backup_meta.is_dir();
    if !orig_is_regular || !backup_is_regular {
        if !orig_is_regular {
            println!("NOT_A_FILE_OR_DIR: {}", orig_path.display());
            stats.inc_not_a_file_or_dir();
        }
        if !backup_is_regular {
            println!("NOT_A_FILE_OR_DIR: {}", backup_path.display());
            stats.inc_not_a_file_or_dir();
        }
        return;
    }

    if orig_meta.is_dir() {
        if !backup_meta.is_dir() {
            println!("DIFFERENT-TYPE: {} (dir vs file)", orig_path.display());
            stats.inc_different();
            report_missing(orig_path, orig_meta, false, config, stats);
            return;
        }

        // Check one-filesystem
        if config.one_filesystem {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let parent = orig_path
                    .parent()
                    .expect("BUG: orig_path inside tree must have a parent");
                match fs::metadata(parent) {
                    Ok(parent_meta) => {
                        if orig_meta.dev() != parent_meta.dev() {
                            println!("DIFFERENT-FS: {}", orig_path.display());
                            stats.inc_skipped();
                            return;
                        }
                    }
                    Err(e) => {
                        println!(
                            "ERROR: Cannot stat parent directory {}: {}",
                            parent.display(),
                            e
                        );
                        stats.inc_errors();
                        return;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                // --one-filesystem is not supported on this platform
            }
        }

        compare_recursive(orig_path, backup_path, config, stats, false);
    } else {
        if !backup_meta.is_file() {
            println!("DIFFERENT-TYPE: {} (file vs dir)", orig_path.display());
            stats.inc_different();
            report_extra(backup_path, backup_meta, false, config, stats);
            return;
        }

        if config.verbosity >= Verbosity::Files {
            println!(
                "DEBUG: Comparing file {} to {}",
                orig_path.display(),
                backup_path.display()
            );
        }

        let reasons = compare_file(orig_path, backup_path, config, stats);
        match reasons {
            Some(r) if r.any() => {
                println!("DIFFERENT-FILE [{}]: {}", r, orig_path.display());
                stats.inc_different();
            }
            Some(_) => {
                stats.inc_similarities();
            }
            None => {
                // Error already reported inside compare_file
            }
        }
    }
}

// ── Missing / Extra ─────────────────────────────────────────────────────────

/// Entry exists in original but not in backup.
fn handle_missing(
    orig_path: &Path,
    orig_meta: &fs::Metadata,
    orig_is_symlink: bool,
    config: &Config,
    stats: &Stats,
) {
    if is_special(orig_meta, orig_is_symlink) {
        println!("NOT_A_FILE_OR_DIR: {}", orig_path.display());
        stats.inc_not_a_file_or_dir();
        return;
    }
    report_missing(orig_path, orig_meta, orig_is_symlink, config, stats);
}

/// Entry exists in backup but not in original.
fn handle_extra(backup_path: &Path, config: &Config, stats: &Stats) {
    let meta = match fs::symlink_metadata(backup_path) {
        Ok(m) => m,
        Err(e) => {
            println!("ERROR: Cannot stat {}: {}", backup_path.display(), e);
            stats.inc_errors();
            return;
        }
    };

    // Check ignore list before counting
    if config.ignore.iter().any(|ig| ig == backup_path) {
        println!("SKIP: {}", backup_path.display());
        stats.inc_skipped();
        return;
    }

    stats.inc_backup_items();

    let is_symlink = meta.file_type().is_symlink();
    if is_special(&meta, is_symlink) {
        println!("NOT_A_FILE_OR_DIR: {}", backup_path.display());
        stats.inc_not_a_file_or_dir();
        return;
    }
    report_extra(backup_path, &meta, is_symlink, config, stats);
}

/// Print diagnostic line and count an entry as missing.
fn report_missing(path: &Path, meta: &fs::Metadata, is_symlink: bool, config: &Config, stats: &Stats) {
    if is_symlink {
        println!("MISSING-SYMLINK: {}", path.display());
    } else if meta.is_dir() {
        println!("MISSING-DIR: {}", path.display());
    } else {
        println!("MISSING-FILE: {}", path.display());
    }
    stats.inc_missing();
    if is_symlink && config.follow {
        // With --follow, resolve the symlink and count its contents if it's a dir
        match fs::metadata(path) {
            Ok(resolved) if resolved.is_dir() => {
                count_recursive(path, config, stats, Direction::Missing);
            }
            Err(_) => {
                println!("DANGLING-SYMLINK: {}", path.display());
                stats.inc_errors();
            }
            _ => {}
        }
    } else if meta.is_dir() {
        count_recursive(path, config, stats, Direction::Missing);
    }
}

/// Print diagnostic line and count an entry as extra.
fn report_extra(path: &Path, meta: &fs::Metadata, is_symlink: bool, config: &Config, stats: &Stats) {
    if is_symlink {
        println!("EXTRA-SYMLINK: {}", path.display());
    } else if meta.is_dir() {
        println!("EXTRA-DIR: {}", path.display());
    } else {
        println!("EXTRA-FILE: {}", path.display());
    }
    stats.inc_extras();
    if is_symlink && config.follow {
        match fs::metadata(path) {
            Ok(resolved) if resolved.is_dir() => {
                count_recursive(path, config, stats, Direction::Extra);
            }
            Err(_) => {
                println!("DANGLING-SYMLINK: {}", path.display());
                stats.inc_errors();
            }
            _ => {}
        }
    } else if meta.is_dir() {
        count_recursive(path, config, stats, Direction::Extra);
    }
}

// ── Recursive counting ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Missing,
    Extra,
}

/// Recursively count contents of a missing or extra directory.
fn count_recursive(dir: &Path, config: &Config, stats: &Stats, direction: Direction) {
    let entries = match read_dir_entries(dir) {
        Ok(e) => e,
        Err(e) => {
            println!("ERROR: Cannot read directory {}: {}", dir.display(), e);
            stats.inc_errors();
            return;
        }
    };

    let mut entries = entries;
    entries.sort();

    for name in &entries {
        let path = dir.join(name);

        // Check ignore list before counting
        if config.ignore.iter().any(|ig| ig == &path) {
            println!("SKIP: {}", path.display());
            stats.inc_skipped();
            continue;
        }

        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                println!("ERROR: Cannot stat {}: {}", path.display(), e);
                stats.inc_errors();
                continue;
            }
        };

        match direction {
            Direction::Missing => {
                stats.inc_original_items();
                stats.inc_missing();
            }
            Direction::Extra => {
                stats.inc_backup_items();
                stats.inc_extras();
            }
        }

        if meta.is_symlink() {
            if config.verbosity >= Verbosity::Files {
                match direction {
                    Direction::Missing => println!("MISSING-SYMLINK: {}", path.display()),
                    Direction::Extra => println!("EXTRA-SYMLINK: {}", path.display()),
                }
            }
        } else if meta.is_dir() {
            if config.verbosity >= Verbosity::Files {
                match direction {
                    Direction::Missing => println!("MISSING-DIR: {}", path.display()),
                    Direction::Extra => println!("EXTRA-DIR: {}", path.display()),
                }
            }
            count_recursive(&path, config, stats, direction);
        } else if config.verbosity >= Verbosity::Files {
            match direction {
                Direction::Missing => println!("MISSING-FILE: {}", path.display()),
                Direction::Extra => println!("EXTRA-FILE: {}", path.display()),
            }
        }
    }
}

// ── File comparison ─────────────────────────────────────────────────────────

/// Compare two files. Returns None if an I/O error prevented comparison.
fn compare_file(orig: &Path, backup: &Path, config: &Config, stats: &Stats) -> Option<DiffReasons> {
    let mut reasons = DiffReasons::default();

    // Size check
    let orig_size = match fs::metadata(orig) {
        Ok(m) => m.len(),
        Err(e) => {
            println!("ERROR: Cannot read metadata for {}: {}", orig.display(), e);
            stats.inc_errors();
            return None;
        }
    };
    let backup_size = match fs::metadata(backup) {
        Ok(m) => m.len(),
        Err(e) => {
            println!("ERROR: Cannot read metadata for {}: {}", backup.display(), e);
            stats.inc_errors();
            return None;
        }
    };

    if orig_size != backup_size {
        reasons.size = true;
    }

    // Sample check — only if sizes match (short-circuit) and samples > 0
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

            match (read_sample(orig, offset, read_len), read_sample(backup, offset, read_len)) {
                (Ok(a), Ok(b)) => {
                    if a != b {
                        reasons.sample = true;
                        break;
                    }
                }
                (Err(e), _) => {
                    println!("ERROR: Cannot read sample from {}: {}", orig.display(), e);
                    stats.inc_errors();
                    return None;
                }
                (_, Err(e)) => {
                    println!("ERROR: Cannot read sample from {}: {}", backup.display(), e);
                    stats.inc_errors();
                    return None;
                }
            }
        }
    }

    // BLAKE3 hash check — only if no prior mismatch (short-circuit)
    if !reasons.any() && config.all {
        let (orig_result, backup_result) = rayon::join(
            || hash_file(orig),
            || hash_file(backup),
        );

        let orig_hash = match orig_result {
            Ok(h) => h,
            Err(e) => {
                println!("ERROR: Cannot hash {}: {}", orig.display(), e);
                stats.inc_errors();
                return None;
            }
        };
        let backup_hash = match backup_result {
            Ok(h) => h,
            Err(e) => {
                println!("ERROR: Cannot hash {}: {}", backup.display(), e);
                stats.inc_errors();
                return None;
            }
        };

        if config.verbosity >= Verbosity::Files {
            println!("DEBUG: BLAKE3 {} {}", orig_hash.to_hex(), orig.display());
            println!("DEBUG: BLAKE3 {} {}", backup_hash.to_hex(), backup.display());
        }

        if orig_hash != backup_hash {
            reasons.hash = true;
        }
    }

    Some(reasons)
}

// ── Utilities ───────────────────────────────────────────────────────────────

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
    // TODO: what happens if we try to read past the end of the file?
    file.read_exact(&mut buf)?;
    Ok(buf)
}

fn hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_mmap_rayon(path)?;
    Ok(hasher.finalize())
}
