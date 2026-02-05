//! Symlink loop detection tests.
//!
//! Tests verify that symlink loops are handled gracefully:
//! - Without --follow: loops are not a problem (symlinks compared by target)
//! - With --follow: loops produce ERROR with "Too many levels of symbolic links"

use super::harness::Entry::*;
use crate::case;

// ===========================================================================
// Without --follow: symlink loops are not a problem
// ===========================================================================

// Self-referential symlink without --follow: just compares targets, no error
// Both sides have symlink pointing to "self" → targets match, SYMLINK-SKIPPED
case!(self_loop_no_follow {
    orig: [
        Sym("loop", "loop"),
    ],
    backup: [
        Sym("loop", "loop"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/loop",
    ],
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

// Two-symlink loop without --follow: just compares targets, no error
case!(two_link_loop_no_follow {
    orig: [
        Sym("link_a", "link_b"),
        Sym("link_b", "link_a"),
    ],
    backup: [
        Sym("link_a", "link_b"),
        Sym("link_b", "link_a"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link_a",
        "SYMLINK-SKIPPED: a/link_b",
    ],
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 2,
    errors: 0,
});

// ===========================================================================
// With --follow: symlink loops produce errors
// ===========================================================================

// Self-referential symlink with --follow: ELOOP error
// symmetric: false because errors have asymmetric behavior
case!(self_loop_with_follow {
    orig: [
        Sym("loop", "loop"),
    ],
    backup: [
        Sym("loop", "loop"),
    ],
    flags: ["--follow"],
    lines: [
        "ERROR: a/loop",
        "ERROR: b/loop",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Too many levels of symbolic links", "Errors: 2"],
    output_excludes: [],
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    // root + symlink itself match (before resolution fails)
    similarities: 2,
    skipped: 0,
    // Both sides produce ELOOP error
    errors: 2,
    symmetric: false,
});

// Two-symlink loop with --follow: ELOOP error for both links
case!(two_link_loop_with_follow {
    orig: [
        Sym("link_a", "link_b"),
        Sym("link_b", "link_a"),
    ],
    backup: [
        Sym("link_a", "link_b"),
        Sym("link_b", "link_a"),
    ],
    flags: ["--follow"],
    lines: [
        "ERROR: a/link_a",
        "ERROR: b/link_a",
        "ERROR: a/link_b",
        "ERROR: b/link_b",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Too many levels of symbolic links", "Errors: 4"],
    output_excludes: [],
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    // root + both symlinks match (before resolution fails)
    similarities: 3,
    skipped: 0,
    // All 4 symlinks (2 per side) produce ELOOP
    errors: 4,
    symmetric: false,
});

// Three-symlink chain loop with --follow
// Using sym1/sym2/sym3 to avoid confusion with a/ b/ directories
case!(three_link_chain_loop_with_follow {
    orig: [
        Sym("sym1", "sym2"),
        Sym("sym2", "sym3"),
        Sym("sym3", "sym1"),
    ],
    backup: [
        Sym("sym1", "sym2"),
        Sym("sym2", "sym3"),
        Sym("sym3", "sym1"),
    ],
    flags: ["--follow"],
    lines: [
        "ERROR: a/sym1",
        "ERROR: b/sym1",
        "ERROR: a/sym2",
        "ERROR: b/sym2",
        "ERROR: a/sym3",
        "ERROR: b/sym3",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Too many levels of symbolic links", "Errors: 6"],
    output_excludes: [],
    original_processed: 7,
    backup_processed: 7,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    // root + 3 symlinks match
    similarities: 4,
    skipped: 0,
    // 3 symlinks × 2 sides = 6 errors
    errors: 6,
    symmetric: false,
});

// Loop in subdirectory with --follow
case!(loop_in_subdir_with_follow {
    orig: [
        File("ok.txt", "ok\n"),
        Dir("sub"),
        Sym("sub/loop", "loop"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        Dir("sub"),
        Sym("sub/loop", "loop"),
    ],
    flags: ["--follow"],
    lines: [
        "ERROR: a/sub/loop",
        "ERROR: b/sub/loop",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Too many levels of symbolic links", "Errors: 2"],
    output_excludes: [],
    // root + ok.txt + sub + sub/loop + resolved sub/loop (error)
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    // root + ok.txt + sub + sub/loop
    similarities: 4,
    skipped: 0,
    errors: 2,
    symmetric: false,
});

// Loop in one tree, valid file in other tree
// orig has a self-loop, backup has a real file
// With --follow, orig loop errors, backup file becomes extra
// The failed resolution attempt also counts as missing
case!(loop_in_orig_valid_in_backup {
    orig: [
        Sym("entry", "entry"),
    ],
    backup: [
        File("entry", "content\n"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "ERROR: entry",
        "MISSING-SYMLINK: a/entry",
        "EXTRA-FILE: b/entry",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Too many levels of symbolic links", "Errors: 1"],
    output_excludes: [],
    original_processed: 3,
    backup_processed: 2,
    // Symlink + failed resolution = 2 missing
    missing: 2,
    // Type mismatch
    different: 1,
    // File is extra
    extras: 1,
    special_files: 0,
    // root
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: false,
});

// Valid file in orig, loop in backup
// With --follow, backup loop errors, orig file becomes missing
// The failed resolution attempt also counts as extra
case!(valid_in_orig_loop_in_backup {
    orig: [
        File("entry", "content\n"),
    ],
    backup: [
        Sym("entry", "entry"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "ERROR: entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Too many levels of symbolic links", "Errors: 1"],
    output_excludes: [],
    original_processed: 2,
    backup_processed: 3,
    // File counted as missing
    missing: 1,
    // Type mismatch
    different: 1,
    // Symlink + failed resolution = 2 extras
    extras: 2,
    special_files: 0,
    // root
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: false,
});
