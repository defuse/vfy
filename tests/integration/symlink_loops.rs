//! Symlink loop detection tests.
//!
//! Tests verify that symlink loops are handled gracefully:
//! - Without --follow: loops are not a problem (symlinks compared by target)
//! - With --follow: loops produce ERROR with "Too many levels of symbolic links"
//!
//! There are two shapes of loop, and they fail very differently:
//!
//! - A symlink pointing at *itself* (`loop -> loop`) is rejected by the kernel
//!   on the first stat(), so we just see ELOOP.
//! - A symlink pointing at a *directory it lives under* (`latest -> .`,
//!   `sub/up -> ..`) resolves fine every single time. Nothing stops the walk
//!   except the kernel's 40-symlink path limit, so the subtree is re-walked ~40
//!   times over, and two such symlinks in one directory make the walk branch
//!   exponentially. Covered by the cases at the bottom of this file.

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

// ===========================================================================
// With --follow: symlinks pointing at an enclosing directory
//
// These resolve successfully every time, so ELOOP never saves us. The walk has
// to notice on its own that it is back inside a directory it is already
// walking, and stop.
// ===========================================================================

// `latest -> .` is a very common convention. Resolving it lands back on the
// directory that contains it, which is already being walked, so there is
// nothing new below it to compare.
case!(self_dir_link_with_follow {
    orig: [
        File("f.txt", "hi\n"),
        Sym("latest", "."),
    ],
    backup: [
        File("f.txt", "hi\n"),
        Sym("latest", "."),
    ],
    flags: ["--follow"],
    lines: [
        "SYMLINK-LOOP: a/latest",
        "SYMLINK-LOOP: b/latest",
    ],
    // root + f.txt + latest + latest's resolved directory
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    // One per side: we stop descending, but the directories themselves matched.
    skipped: 2,
    // A tree that links back into itself is malformed enough to flag, and to
    // exit non-zero over.
    errors: 2,
});

// `sub/up -> ..` resolves to the root, two levels up the walk.
case!(parent_dir_link_with_follow {
    orig: [
        Dir("sub"),
        Sym("sub/up", ".."),
    ],
    backup: [
        Dir("sub"),
        Sym("sub/up", ".."),
    ],
    flags: ["--follow"],
    lines: [
        "SYMLINK-LOOP: a/sub/up",
        "SYMLINK-LOOP: b/sub/up",
    ],
    // root + sub + sub/up + sub/up's resolved directory
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 2,
    errors: 2,
});

// Two loop symlinks in one directory. Each level of the walk branches in two,
// so without loop detection this is 2^40 paths — it never finishes.
case!(two_parent_dir_links_with_follow {
    orig: [
        Dir("sub"),
        Sym("sub/up1", ".."),
        Sym("sub/up2", ".."),
    ],
    backup: [
        Dir("sub"),
        Sym("sub/up1", ".."),
        Sym("sub/up2", ".."),
    ],
    flags: ["--follow"],
    lines: [
        "SYMLINK-LOOP: a/sub/up1",
        "SYMLINK-LOOP: b/sub/up1",
        "SYMLINK-LOOP: a/sub/up2",
        "SYMLINK-LOOP: b/sub/up2",
    ],
    // root + sub + (up1 + its resolved dir) + (up2 + its resolved dir)
    original_processed: 6,
    backup_processed: 6,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 6,
    skipped: 4,
    errors: 4,
});

// The same loop, but inside a directory that is missing from the other side, so
// it is report() rather than compare() doing the walking.
case!(loop_inside_missing_dir_with_follow {
    orig: [
        Dir("miss"),
        Sym("miss/self", "."),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-DIR: a/miss",
        "SYMLINK-LOOP: a/miss/self",
    ],
    // root + miss + miss/self + self's resolved directory
    original_processed: 4,
    backup_processed: 1,
    // miss, miss/self, and the resolved directory
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 1,
});

// ===========================================================================
// One side loops, the other holds real data
//
// The loop stops that side's descent, but the other side still points at data
// this comparison never checked. Which way it points decides what we say about
// it, following the same rule the error paths use: unverified original data is
// reported missing, unverified backup data is never called extra, because EXTRA
// reads as "safe to delete from your backup".
// ===========================================================================

// Backup loops, original points at real data. We cannot claim that data is
// backed up -- the backup has a loop where it should be -- so report it missing.
case!(backup_loops_original_has_data_is_missing {
    orig: [
        Dir("payload"),
        File("payload/x.txt", "x\n"),
        Sym("link", "payload"),
    ],
    backup: [
        Dir("payload"),
        File("payload/x.txt", "x\n"),
        Sym("link", "."),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link",
        "MISSING-FILE: a/link/x.txt",
        "SYMLINK-LOOP: b/link",
    ],
    debug_contains: [],
    debug_excludes: [],
    // The original's data is accounted for rather than silently passing.
    output_contains: ["Missing: 1"],
    // Nothing here justifies telling the user to delete from their backup.
    output_excludes: ["EXTRA"],
    // root + link + resolved dir + link/x.txt + payload + payload/x.txt
    original_processed: 6,
    // root + link + resolved dir + payload + payload/x.txt
    backup_processed: 5,
    missing: 1,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 1,
    errors: 1,
    symmetric: false,
});

// Mirror image: the original loops and the backup points at real data. We have
// not shown the original lacks that data, so it is skipped, never called extra.
case!(original_loops_backup_has_data_is_not_extra {
    orig: [
        Dir("payload"),
        File("payload/x.txt", "x\n"),
        Sym("link", "."),
    ],
    backup: [
        Dir("payload"),
        File("payload/x.txt", "x\n"),
        Sym("link", "payload"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link",
        "SYMLINK-LOOP: a/link",
        "SKIP: b/link",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Extras: 0"],
    output_excludes: ["EXTRA"],
    // root + link + resolved dir + payload + payload/x.txt
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 2,
    errors: 1,
    symmetric: false,
});
