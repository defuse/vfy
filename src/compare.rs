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

// -- Symlink loop detection ---------------------------------------------------

/// Identity of a directory, for deciding whether we have been here before.
///
/// Only needed with --follow. A symlink pointing at a directory it lives under
/// (`latest -> .`, `sub/up -> ..`) resolves successfully every time, so nothing
/// stops us walking through it again and again except the kernel's 40-symlink
/// path limit. That alone re-walks the subtree ~40 times over, and two such
/// symlinks in one directory make the walk branch exponentially and never
/// finish.
///
/// Cheap: load_meta() already stat()ed the directory, so this is just field
/// access, no extra syscall.
#[cfg(unix)]
fn dir_identity(meta: &fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((meta.dev(), meta.ino()))
}

/// No cheap inode identity on non-Unix, so loop detection is disabled there.
/// Same caveat as the rest of our non-Unix behavior, see #29.
#[cfg(not(unix))]
fn dir_identity(_meta: &fs::Metadata) -> Option<(u64, u64)> {
    None
}

/// The directories we walked through to reach where we are now, innermost
/// first. Built on the stack as we recurse, so there is nothing to allocate and
/// nothing to unwind on the way back out.
struct DirChain<'a> {
    id: (u64, u64),
    parent: Option<&'a DirChain<'a>>,
}

impl<'a> DirChain<'a> {
    /// True if `meta`'s directory is one we are already inside. Always false
    /// when we have no usable identity for it.
    fn is_ancestor(chain: Option<&DirChain>, meta: &fs::Metadata) -> bool {
        let Some(id) = dir_identity(meta) else {
            return false;
        };
        let mut node = chain;
        while let Some(n) = node {
            if n.id == id {
                return true;
            }
            node = n.parent;
        }
        false
    }

    /// Chain to hand to the children of `meta`'s directory. Returns None when we
    /// have no usable identity, in which case callers keep the parent's chain.
    fn extend(chain: Option<&'a DirChain<'a>>, meta: &fs::Metadata) -> Option<DirChain<'a>> {
        dir_identity(meta).map(|id| DirChain { id, parent: chain })
    }
}

/// The two chains, one per tree. They are always threaded together, since
/// compare() walks both trees in lockstep.
#[derive(Clone, Copy)]
struct Chains<'a> {
    orig: Option<&'a DirChain<'a>>,
    backup: Option<&'a DirChain<'a>>,
}

/// Report that `path` resolved back into a directory we are already walking.
///
/// Counts as an error as well as a skip. No data goes uncompared -- the
/// directory we looped back to is one we are already walking, so its contents
/// get compared at that level -- but a tree that links back into itself is
/// malformed enough that the user should be told, and told loudly enough to
/// affect the exit code.
fn report_loop(path: &Path, stats: &Stats) {
    println!(
        "SYMLINK-LOOP: [{}] (resolves to a directory we are already inside)",
        path.display()
    );
    stats.inc_errors();
    stats.inc_skipped();
}

// -- Ignore matching ----------------------------------------------------------

/// True if `path` was ignored via --ignore, either directly or because the same
/// path in the other tree was ignored.
///
/// Ignoring one side ignores the other, so that --ignore describes a location in
/// the backup rather than a location in one particular tree. compare() gets this
/// for free by checking both sides of the pair it holds, but a subtree that only
/// exists on one side is walked by report(), which only ever sees one path and
/// has to map it across itself.
fn is_ignored(path: &Path, config: &Config) -> bool {
    if config.ignore.iter().any(|ig| ig == path) {
        return true;
    }

    // Same relative position in the other tree. Both roots are canonical and so
    // is `path`, so this is a plain prefix swap.
    let mirrored = if let Ok(suffix) = path.strip_prefix(&config.original) {
        config.backup.join(suffix)
    } else if let Ok(suffix) = path.strip_prefix(&config.backup) {
        config.original.join(suffix)
    } else {
        return false;
    };

    config.ignore.contains(&mirrored)
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
    let chains = Chains { orig: None, backup: None };
    compare(&config.original, &config.backup, false, chains, config, stats);
}

// -- Comparison ---------------------------------------------------------------

/// Compare two paths at the same relative position in both trees.
///
/// orig, backup, and ignore paths in the Config MUST be absolute paths for
/// ignore comparisons to work properly.
///
/// Pre: Neither path counted as an item yet. Both sides expected to exist.
/// Post: Both counted. Fully categorized. Descendants processed.
fn compare<'a>(
    orig: &Path,
    backup: &Path,
    follow: bool,
    chains: Chains<'a>,
    config: &Config,
    stats: &Stats,
) {
    // Check ignore first so we don't encounter errors, different fs, or special files that the user ignored.
    if is_ignored(orig, config) || is_ignored(backup, config) {
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
            compare_files(orig, backup, om, bm, chains.orig, config, stats);
            return;
        }
        (Meta::Dir(_, _), Meta::Dir(_, _)) => {
            compare_directories(orig, backup, &meta_orig, &meta_back, chains, config, stats);
            return;
        }
        (Meta::Symlink(_), Meta::Symlink(_)) => {
            compare_symlinks(orig, backup, chains, config, stats);
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
        // We pass false so that a symlink is reported as MISSING-SYMLINK here and
        // its resolved target is counted separately by the recursive report()
        // call. Only -vv prints a line for that target; at lower verbosity it is
        // counted but not shown, like the children of a MISSING-DIR.
        report(orig, Direction::Missing, false, true, chains.orig, config, stats);
    }

    if meta_back.is_file_dir_or_symlink() {
        match meta_orig {
            // Error means we can't verify, don't suggest deletion of potentially
            // valid backup by calling it "EXTRA".
            Meta::Error(_) => {
                // Let the user know we are skipping the other side, though.
                println!("SKIP: [{}]", backup.display());
                stats.inc_skipped();
            },
            _ => {
                // TODO: See #24, reporting backup files as "extra" when the
                // backup has real files but the original contains some other
                // type may not be what we want to do, i.e. EXTRA might indicate
                // to the user they can safely delete files, when that's not
                // necesarily true.
                report(backup, Direction::Extra, false, true, chains.backup, config, stats);
            }
        }
    }
}

/// Compare two files.
///
/// Pre: Both are files. Neither counted. Metadata loaded.
/// Post: Both counted. Content compared. Log messages shown.
fn compare_files<'a>(
    orig: &Path,
    backup: &Path,
    orig_meta: &fs::Metadata,
    backup_meta: &fs::Metadata,
    orig_chain: Option<&'a DirChain<'a>>,
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
            report(orig, Direction::Missing, false, true, orig_chain, config, stats);
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
fn compare_directories<'a>(
    orig: &Path,
    backup: &Path,
    orig_meta: &Meta,
    backup_meta: &Meta,
    chains: Chains<'a>,
    config: &Config,
    stats: &Stats,
) {
    if config.verbosity >= Verbosity::Dirs {
        println!("DEBUG: Comparing [{}] to [{}]", orig.display(), backup.display());
    }

    let (orig_fs_meta, orig_entries) = match orig_meta {
        Meta::Dir(m, e) => (m, e),
        _ => unreachable!()
    };

    let (backup_fs_meta, backup_entries) = match backup_meta {
        Meta::Dir(m, e) => (m, e),
        _ => unreachable!()
    };

    stats.inc_original_items();
    stats.inc_backup_items();
    // Both directories being present counts as a similarity, even if their contents differ
    stats.inc_similarities();

    let orig_child = DirChain::extend(chains.orig, orig_fs_meta);
    let backup_child = DirChain::extend(chains.backup, backup_fs_meta);
    let child_chains = Chains {
        orig: orig_child.as_ref().or(chains.orig),
        backup: backup_child.as_ref().or(chains.backup),
    };

    // The directories themselves matched, but if --follow walked us back into a
    // directory we are already inside, stop before descending.
    let orig_loops = DirChain::is_ancestor(chains.orig, orig_fs_meta);
    let backup_loops = DirChain::is_ancestor(chains.backup, backup_fs_meta);
    if orig_loops || backup_loops {
        // A looping side contributes nothing new: it resolved to a directory we
        // are already walking, whose contents get compared there.
        if orig_loops {
            report_loop(orig, stats);
        } else {
            // The original did not loop, so it holds real data that this
            // comparison never got to check against anything. Report it missing,
            // the same as when the backup side errors out: we cannot claim it is
            // backed up, and saying nothing would let it pass silently.
            for name in orig_entries {
                report(&orig.join(name), Direction::Missing, false, true, child_chains.orig, config, stats);
            }
        }

        if backup_loops {
            report_loop(backup, stats);
        } else {
            // Deliberately NOT reported as extra. We never established that the
            // original lacks this data, and EXTRA reads as "safe to delete".
            println!("SKIP: [{}]", backup.display());
            stats.inc_skipped();
        }

        return;
    }

    let mut backup_set: HashSet<&OsString> = backup_entries.into_iter().collect();

    for name in orig_entries {
        let orig_path = orig.join(name);
        let backup_path = backup.join(name);

        let in_backup = backup_set.remove(name);
        if in_backup {
            compare(&orig_path, &backup_path, false, child_chains, config, stats);
        } else {
            report(&orig_path, Direction::Missing, false, true, child_chains.orig, config, stats);
        }
    }

    // Remaining in backup_set are extras
    let mut extras: Vec<&OsString> = backup_set.into_iter().collect();
    extras.sort();

    for name in &extras {
        let backup_path = backup.join(name);
        report(&backup_path, Direction::Extra, false, true, child_chains.backup, config, stats);
    }
}

/// Compare two symlinks.
///
/// Pre: Both are symlinks. Neither counted.
/// Post: Both counted. Targets compared. If --follow, resolved content
/// compared via compare. Logs output.
fn compare_symlinks<'a>(
    orig: &Path,
    backup: &Path,
    chains: Chains<'a>,
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
            report(orig, Direction::Missing, false, true, chains.orig, config, stats);

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
        compare(orig, backup, true, chains, config, stats);
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
fn report<'a>(
    path: &Path,
    direction: Direction,
    follow: bool,
    // Gets set to true for the top-level call, and false for recursive calls unless verbosity level is >= Files.
    print: bool,
    chain: Option<&'a DirChain<'a>>,
    config: &Config,
    stats: &Stats,
) {
    // Check ignore first (before any I/O)
    if is_ignored(path, config) {
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
        Meta::Dir(m, entries) => {
            // Same loop guard as compare_directories. Here there is only one
            // tree in play, and whatever we looped back to is already being
            // reported at the level we looped back to, so there is no
            // one-sided data to account for.
            if DirChain::is_ancestor(chain, m) {
                report_loop(path, stats);
                return;
            }

            let child_chain = DirChain::extend(chain, m);
            let child_chain = child_chain.as_ref().or(chain);

            for name in entries {
                report(&path.join(name), direction, false, print_children, child_chain, config, stats);
            }
        }
        Meta::Symlink(_) if config.follow => {
            report(path, direction, true, print_children, chain, config, stats);
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
        let mut rng = rand::rng();
        let sample_size: u64 = 32;

        for _ in 0..config.samples {
            let max_offset = orig_size.saturating_sub(sample_size);
            let offset = rng.random_range(0..=max_offset);
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
