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
    compare_recursive(&config.original, &config.backup, config, stats);
}

fn compare_recursive(orig_dir: &Path, backup_dir: &Path, config: &Config, stats: &Stats) {
    if config.verbosity >= Verbosity::Dirs {
        println!("DEBUG: Comparing {} to {}", orig_dir.display(), backup_dir.display());
    }

    // Check ignore list against both original and backup paths
    if config.ignore.iter().any(|ig| ig == orig_dir || ig == backup_dir) {
        println!("SKIP: {}", orig_dir.display());
        stats.inc_skipped();
        return;
    }

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

    let backup_set: HashSet<OsString> = backup_entries.iter().cloned().collect();

    let mut orig_entries = orig_entries;
    orig_entries.sort();

    for name in &orig_entries {
        let orig_path = orig_dir.join(name);
        let backup_path = backup_dir.join(name);

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

/// Both original and backup contain this entry.
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

    // Determine the "resolved" type of each side (following symlinks)
    let orig_is_dir = if orig_is_symlink {
        fs::metadata(orig_path).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        orig_meta.is_dir()
    };
    let backup_is_dir = if backup_is_symlink {
        fs::metadata(backup_path).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        backup_meta.is_dir()
    };

    // If either side is a symlink, check symlink-specific logic first
    if orig_is_symlink || backup_is_symlink {
        // One is symlink and the other isn't → mismatch
        if orig_is_symlink != backup_is_symlink {
            println!("SYMMIS: {} (symlink mismatch)", orig_path.display());
            stats.inc_different();
            return;
        }

        // Both are symlinks
        // If they point to directories: respect --follow
        if orig_is_dir && backup_is_dir {
            if config.follow {
                // Traverse the symlinked directories
                compare_recursive(orig_path, backup_path, config, stats);
            } else {
                println!(
                    "SYMLINK: {} (symlink to directory, use --follow to traverse)",
                    orig_path.display()
                );
            }
            return;
        }

        // Both are symlinks to non-directories: compare targets
        let orig_target = fs::read_link(orig_path);
        let backup_target = fs::read_link(backup_path);
        match (orig_target, backup_target) {
            (Ok(ot), Ok(bt)) => {
                if ot != bt {
                    println!(
                        "SYMMIS: {} (targets differ: {:?} vs {:?})",
                        orig_path.display(),
                        ot,
                        bt
                    );
                    stats.inc_different();
                }
            }
            _ => {
                println!("ERROR: Cannot read symlink targets for {}", orig_path.display());
                stats.inc_errors();
            }
        }
        return;
    }

    // Neither side is a symlink
    if orig_meta.is_dir() {
        if !backup_meta.is_dir() {
            println!("DIFFERENT-FILE [TYPE]: {} (dir vs file)", orig_path.display());
            stats.inc_different();
            return;
        }

        // Check one-filesystem
        if config.one_filesystem {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let Ok(parent_meta) = fs::metadata(orig_path.parent().unwrap_or(orig_path)) {
                    if orig_meta.dev() != parent_meta.dev() {
                        println!("DIFFFS: {}", orig_path.display());
                        stats.inc_skipped();
                        return;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                // --one-filesystem is not supported on this platform
            }
        }

        compare_recursive(orig_path, backup_path, config, stats);
    } else if orig_meta.is_file() {
        if !backup_meta.is_file() {
            println!("DIFFERENT-FILE [TYPE]: {} (file vs dir)", orig_path.display());
            stats.inc_different();
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
            None => {
                // Error already reported inside compare_file
            }
            _ => {}
        }
    } else {
        // Special file type (socket, FIFO, device, etc.)
        println!("ERROR: Unsupported file type for {}", orig_path.display());
        stats.inc_errors();
    }
}

/// Entry exists in original but not in backup.
fn handle_missing(
    orig_path: &Path,
    orig_meta: &fs::Metadata,
    orig_is_symlink: bool,
    config: &Config,
    stats: &Stats,
) {
    if !orig_is_symlink && orig_meta.is_dir() {
        println!("MISSING-DIR: {}", orig_path.display());
        stats.inc_missing();
        count_recursive(orig_path, config, stats, Direction::Missing);
    } else {
        println!("MISSING-FILE: {}", orig_path.display());
        stats.inc_missing();
    }
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

    stats.inc_backup_items();
    stats.inc_extras();

    if meta.is_dir() {
        // Check ignore list for extra dirs in backup tree
        if config.ignore.iter().any(|ig| ig == backup_path) {
            println!("SKIP: {}", backup_path.display());
            // Undo the extras/backup_items increments since we're skipping
            stats.dec_extras();
            stats.dec_backup_items();
            stats.inc_skipped();
            return;
        }
        println!("EXTRA-DIR: {}", backup_path.display());
        count_recursive(backup_path, config, stats, Direction::Extra);
    } else {
        println!("EXTRA-FILE: {}", backup_path.display());
    }
}

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

        if meta.is_dir() {
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

    // Sample check — only if sizes match and samples > 0
    if !reasons.size && config.samples > 0 && orig_size > 0 {
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

    // BLAKE3 hash check
    if config.all {
        let orig_hash = match hash_file(orig) {
            Ok(h) => h,
            Err(e) => {
                println!("ERROR: Cannot hash {}: {}", orig.display(), e);
                stats.inc_errors();
                return None;
            }
        };
        let backup_hash = match hash_file(backup) {
            Ok(h) => h,
            Err(e) => {
                println!("ERROR: Cannot hash {}: {}", backup.display(), e);
                stats.inc_errors();
                return None;
            }
        };

        if config.verbosity >= Verbosity::Files {
            println!("DEBUG: BLAKE3 {} {}", orig_hash, orig.display());
            println!("DEBUG: BLAKE3 {} {}", backup_hash, backup.display());
        }

        if orig_hash != backup_hash {
            reasons.hash = true;
        }
    }

    Some(reasons)
}

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

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut file = fs::File::open(path)?;
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}
