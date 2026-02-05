//! Comprehensive type-comparison matrix tests.
//!
//! Each `case!` invocation generates a test that:
//!   1. Creates filesystem entries in temp orig/backup dirs
//!   2. Runs `vfy` and checks output lines + summary counts
//!   3. Automatically runs the reversed direction (orig↔backup swapped)
//!
//! Entry types: File, Dir, Sym (symlink), Fifo (named pipe).
//! Entries are composable: `Sym("entry","target"), File("target","x")`
//! creates a symlink with its target file.

// Import shared test infrastructure from harness module
use super::harness::Entry;
use Entry::*;

// Re-use the case! macro from harness
use crate::case;

// ===========================================================================
// Test cases
// ===========================================================================

// === Orig = File ===

case!(file_x_file_same {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        File("entry", "contents"),
    ],
    flags: [],
    lines: [],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

case!(file_x_file_diff {
    orig: [
        File("entry", "original-contents"),
    ],
    backup: [
        File("entry", "different-contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_dir {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    flags: [],
    lines: [
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
    ],
    // Both sides reported: MISSING-FILE for orig file, EXTRA-DIR for backup dir
    // child counted by report but only printed at -vv
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_dir_vv {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    flags: ["-vv"],
    lines: [
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
        "EXTRA-FILE: b/entry/child",
    ],
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_fifo {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-FILE: a/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_absent {
    orig: [
        File("entry", "contents"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-FILE: a/entry",
    ],
    original_processed: 2,
    backup_processed: 1,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// === Orig = Dir ===

case!(dir_x_dir {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    flags: [],
    lines: [],
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

case!(dir_x_fifo {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-DIR: a/entry",
    ],
    // Backup is special → SPECIAL-FILE for the fifo.
    // Orig is a dir → MISSING-DIR + count_recursive counts the child as missing too.
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_absent {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/entry",
    ],
    original_processed: 3,
    backup_processed: 1,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// === Orig = Fifo ===

case!(fifo_x_fifo {
    orig: [
        Fifo("entry"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: a/entry",
        "SPECIAL-FILE: b/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 2,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(fifo_x_absent {
    orig: [
        Fifo("entry"),
    ],
    backup: [],
    flags: [],
    lines: [
        "SPECIAL-FILE: a/entry",
        "MISSING-SPECIAL: a/entry",
    ],
    original_processed: 2,
    backup_processed: 1,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// === One-side symlink: File x Sym (no --follow) ===

case!(file_x_symlink_to_file {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_to_dir {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_dangling {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// === One-side symlink: File x Sym (--follow) ===
// Currently --follow has no effect on one-side-symlink mismatch.
// Same expected results as without --follow for now.

case!(file_x_symlink_to_file_follow {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved file counted as item (+1 backup, +1 extras)
    original_processed: 2,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_to_dir_follow {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved dir + child counted as items
    // "target" dir + child only in backup → EXTRA-DIR + child
    original_processed: 2,
    backup_processed: 6,
    missing: 1,
    different: 1,
    extras: 5,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_dangling_follow {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → dangling → error
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === One-side symlink: Dir x Sym (no --follow) ===

case!(dir_x_symlink_to_file {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    // orig is a real dir → report counts dir + child as missing
    // symlink side also reported as EXTRA-SYMLINK
    // "target" only in backup → EXTRA-FILE
    original_processed: 3,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_to_file_follow {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved file counted (+1 backup, +1 extras)
    // orig dir → report: dir(1) + child(1) = 2 missing
    original_processed: 3,
    backup_processed: 4,
    missing: 2,
    different: 1,
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_to_dir {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child2", "data2"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    // orig dir → report: dir(1) + child(1) = 2 missing
    // symlink side also reported as EXTRA-SYMLINK
    // "target" dir + child2 only in backup → EXTRA-DIR(1) + child2(1) = 2+1 extras
    original_processed: 3,
    backup_processed: 4,
    missing: 2,
    different: 1,
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_to_dir_follow {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
        File("target/child2", "data2"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved dir(1) + child(1) + child2(1) = 3 more extras
    // "target" dir + children also in backup = EXTRA-DIR + child + child2
    original_processed: 3,
    backup_processed: 8,
    missing: 2,
    different: 1,
    extras: 7,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_dangling {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
    ],
    // orig dir → report: dir(1) + child(1) = 2 missing
    // symlink side also reported as EXTRA-SYMLINK
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_dangling_follow {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows dangling symlink → error
    original_processed: 3,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === One-side symlink: Sym x Fifo ===

case!(symlink_to_file_x_fifo {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    // "target": only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_dir_x_fifo {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    // "target": only in orig → MISSING-DIR
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_dangling_x_fifo {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
    ],
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    original_processed: 2,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// --follow doesn't change Sym x Fifo (special takes priority)

case!(symlink_to_file_x_fifo_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: ["--follow"],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Cat3: report follows symlink → resolved file counted as separate item (+1 orig, +1 missing)
    // "target" only in orig → MISSING-FILE
    original_processed: 4,
    backup_processed: 2,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_dir_x_fifo_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: ["--follow"],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Cat3: report follows symlink → resolved dir counted as separate item (+1 orig, +1 missing)
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    // "target": only in orig → MISSING-DIR
    original_processed: 4,
    backup_processed: 2,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_dangling_x_fifo_follow {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: ["--follow"],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: a/entry",
    ],
    // Cat3: report follows symlink → dangling counted as separate item (+1 orig)
    // "entry": backup FIFO → SPECIAL-FILE, orig dangling symlink → MISSING-SYMLINK + DANGLING-SYMLINK (error)
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === One-side symlink: Sym x Absent ===

case!(symlink_to_file_x_absent {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // "entry" is a symlink only in orig → MISSING-SYMLINK
    // "target" is a file only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 1,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_file_x_absent_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Cat3: report follows symlink → resolved file counted as separate item (+1 orig, +1 missing)
    // Resolved MISSING-FILE not printed at default verbosity
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_dir_x_absent {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // "target/child" counted in missing but not printed without -vv
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// With --follow, the symlink resolves to a dir, so its contents are also counted as missing.
case!(symlink_to_dir_x_absent_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Cat3: report follows symlink → resolved dir+child counted as separate items
    // entry + entry/child + target + target/child = 5 missing
    // entry/child and target/child not printed without -vv
    original_processed: 6,
    backup_processed: 1,
    missing: 5,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_dangling_x_absent {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/entry",
    ],
    original_processed: 2,
    backup_processed: 1,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// With --follow, the dangling symlink is reported as an error.
case!(symlink_dangling_x_absent_follow {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: a/entry",
    ],
    // Cat3: report follows symlink → dangling counted as separate item (+1 orig)
    original_processed: 3,
    backup_processed: 1,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === Both symlinks: same type, same target ===

case!(symfile_x_symfile_same {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/entry",
    ],
    // Both sides have identical symlinks → SYMLINK (skipped)
    // "target" files match → similarity
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symfile_same_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: ["--follow"],
    lines: [],
    // Cat2: resolved file counted as separate item (+1 orig, +1 backup)
    // Cat4: same-target symlink pair counted as similarity (+1 sim)
    // entry resolves to file with same contents → similarity
    // "target" files also match → similarity
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 0,
    errors: 0,
});

case!(symdir_x_symdir_same {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/entry",
    ],
    // Both sides have identical symlinks → SYMLINK (skipped)
    // "target" dirs match, "target/child" files match → similarities
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 1,
    errors: 0,
});

case!(symdir_x_symdir_same_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: ["--follow"],
    lines: [],
    // Cat2: resolved dir+child counted as separate items (+1 orig, +1 backup each)
    // With --follow, entry resolves to target dir → compare_recursive
    // entry/child discovered via followed symlink
    // Cat4: same-target symlink pair counted as similarity
    // target dir also walked independently → target/child discovered
    // All match → similarities
    original_processed: 6,
    backup_processed: 6,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 6,
    skipped: 0,
    errors: 0,
});

case!(symdangling_x_symdangling_same {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/entry",
    ],
    // Both symlinks point to same target → SYMLINK (skipped)
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symdangling_x_symdangling_same_follow {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Cat2: resolved (dangling) content counted as separate items (+1 orig, +1 backup)
    // Both dangling with same target — symlinks match, both dangling = 2 errors
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 2,
});

// === Both symlinks: same type, different target ===

case!(symfile_x_symfile_diff {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "target-b"),
        File("target-b", "contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK-SKIPPED: a/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-FILE: b/target-b",
    ],
    // "entry" symlinks differ in target → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" only in orig → MISSING-FILE
    // "target-b" only in backup → EXTRA-FILE
    original_processed: 3,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symfile_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "same-content"),
    ],
    backup: [
        Sym("entry", "target-b"),
        File("target-b", "same-content"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-FILE: b/target-b",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved files counted as separate items (+1 orig, +1 backup)
    // With --follow, resolved files have same content → similarity for entry
    // "target-a" only in orig → MISSING-FILE
    // "target-b" only in backup → EXTRA-FILE
    original_processed: 4,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

case!(symdir_x_symdir_diff {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK-SKIPPED: a/entry",
        "MISSING-DIR: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // "entry" symlinks differ in target → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" dir only in orig → MISSING-DIR (+ child counted)
    // "target-b" dir only in backup → EXTRA-DIR (+ child counted)
    original_processed: 4,
    backup_processed: 4,
    missing: 2,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symdir_x_symdir_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
        File("target-a/child2", "original"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
        File("target-b/child2", "different"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DIFFERENT-FILE [SIZE]: a/entry/child2",
        "MISSING-DIR: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved dir+children counted as separate items (+1 orig, +1 backup each)
    // With --follow, entry resolves to dir → compare_recursive
    //   entry/child matches → similarity
    //   entry/child2 differs → different
    // "target-a" only in orig → MISSING-DIR (+ 2 children counted)
    // "target-b" only in backup → EXTRA-DIR (+ 2 children counted)
    original_processed: 8,
    backup_processed: 8,
    missing: 3,
    different: 2,
    extras: 3,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

case!(symdangling_x_symdangling_diff {
    orig: [
        Sym("entry", "nonexistent-a"),
    ],
    backup: [
        Sym("entry", "nonexistent-b"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK-SKIPPED: a/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symdangling_x_symdangling_diff_follow {
    orig: [
        Sym("entry", "nonexistent-a"),
    ],
    backup: [
        Sym("entry", "nonexistent-b"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DANGLING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET, both dangling = 2 errors
    // Cat2: resolved (dangling) content counted as separate items (+1 orig, +1 backup)
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 2,
});

// === Both symlinks: cross-type, different targets ===

case!(symfile_x_symdir_diff {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK-SKIPPED: a/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // "entry" symlinks differ → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" file only in orig → MISSING-FILE
    // "target-b" dir only in backup → EXTRA-DIR (+ child counted)
    original_processed: 3,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdir_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved content counted as separate items
    // Cat1: both sides reported → MISSING-FILE + EXTRA-DIR
    // With --follow, entry resolves to file vs dir → FILE-DIR-MISMATCH + MISSING-FILE + EXTRA-DIR (+ child)
    // "target-a" only in orig → MISSING-FILE
    // "target-b" dir only in backup → EXTRA-DIR (+ child counted)
    original_processed: 4,
    backup_processed: 6,
    missing: 2,
    different: 2,
    extras: 4,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symfile_x_symdangling_diff {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK-SKIPPED: a/entry",
        "MISSING-FILE: a/target-a",
    ],
    // "entry" symlinks differ → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdangling_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DANGLING-SYMLINK: b/entry",
        "MISSING-FILE: a/entry",
        "MISSING-FILE: a/target-a",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved content counted as separate items
    // Backup dangling → DANGLING-SYMLINK (error), inc_backup_items
    // Orig entry resolves to file → report(entry, Missing): MISSING-FILE
    // "target-a" only in orig → MISSING-FILE
    original_processed: 4,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

case!(symdir_x_symdangling_diff {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK-SKIPPED: a/entry",
        "MISSING-DIR: a/target-a",
    ],
    // "entry" symlinks differ → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 4,
    backup_processed: 2,
    missing: 2,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symdir_x_symdangling_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DANGLING-SYMLINK: b/entry",
        "MISSING-DIR: a/entry",
        "MISSING-DIR: a/target-a",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved content counted as separate items
    // Backup dangling → DANGLING-SYMLINK (error), inc_backup_items
    // Orig resolves to dir → report(entry, Missing): dir(1) + child(1) counted as missing
    // "target-a" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 6,
    backup_processed: 3,
    missing: 4,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === Both symlinks: cross-type, same target (different resolution per side) ===

case!(symfile_x_symdir_same_target {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/entry",
        "FILE-DIR-MISMATCH: a/target",
        "MISSING-FILE: a/target",
        "EXTRA-DIR: b/target",
    ],
    // Same symlink target → SYMLINK (skipped), similarity
    // "target" is file in orig vs dir in backup → FILE-DIR-MISMATCH
    //   Both sides reported: MISSING-FILE + EXTRA-DIR (+ child counted)
    original_processed: 3,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdir_same_target_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: ["--follow"],
    lines: [
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
        "FILE-DIR-MISMATCH: a/target",
        "MISSING-FILE: a/target",
        "EXTRA-DIR: b/target",
    ],
    // Cat2: resolved content counted as separate items
    // Cat1: both sides reported in type mismatch → MISSING-FILE + EXTRA-DIR
    // With --follow, entry resolves: file vs dir → FILE-DIR-MISMATCH + MISSING-FILE + EXTRA-DIR (+ child)
    // "target" also file vs dir → FILE-DIR-MISMATCH + MISSING-FILE + EXTRA-DIR (+ child)
    // Cat4: same-target symlink pair counted as similarity
    original_processed: 4,
    backup_processed: 6,
    missing: 2,
    different: 2,
    extras: 4,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

case!(symfile_x_symdangling_same_target {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Same target → SYMLINK (skipped), similarity
    // "target" file only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdangling_same_target_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: b/entry",
        "MISSING-FILE: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Cat2: resolved content counted as separate items
    // Cat4: same-target symlink pair counted as similarity
    // Same target → similarity. Backup dangling → DANGLING-SYMLINK (error)
    // Orig entry resolves to file → report(entry, Missing): MISSING-FILE
    // "target" file only in orig → MISSING-FILE
    original_processed: 4,
    backup_processed: 3,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 1,
});

case!(symdir_x_symdangling_same_target {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Same target → SYMLINK (skipped), similarity
    // "target" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 4,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symdir_x_symdangling_same_target_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: b/entry",
        "MISSING-DIR: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Cat2: resolved content counted as separate items
    // Cat4: same-target symlink pair counted as similarity
    // Same target → similarity. Backup dangling → DANGLING-SYMLINK (error)
    // Orig entry resolves to dir → report(entry, Missing): dir(1) + child(1) counted as missing
    // "target" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 6,
    backup_processed: 3,
    missing: 4,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 1,
});
