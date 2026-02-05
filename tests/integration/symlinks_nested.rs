//! Nested symlink (symlink chain) tests.
//!
//! Tests verify behavior when symlinks point to other symlinks:
//! - Without --follow: only immediate targets are compared
//! - With --follow: chains are fully resolved and compared
//!
//! Chain depth, dangling chains, and directory traversal through chains are covered.

use super::harness::Entry::*;
use crate::case;

// ===========================================================================
// 2-level chains (symlink -> symlink -> target)
// ===========================================================================

// Symlink chain to file: link1 -> link2 -> file.txt
// Without --follow: immediate targets compared (link2 vs link2), SYMLINK-SKIPPED
case!(chain_to_file_no_follow {
    orig: [
        File("file.txt", "content\n"),
        Sym("link2", "file.txt"),
        Sym("link1", "link2"),
    ],
    backup: [
        File("file.txt", "content\n"),
        Sym("link2", "file.txt"),
        Sym("link1", "link2"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link1",
        "SYMLINK-SKIPPED: a/link2",
    ],
    // root + file.txt + link2 + link1 = 4
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 2,
    errors: 0,
});

// Symlink chain to file with --follow: resolves to actual file content
case!(chain_to_file_with_follow {
    orig: [
        File("file.txt", "content\n"),
        Sym("link2", "file.txt"),
        Sym("link1", "link2"),
    ],
    backup: [
        File("file.txt", "content\n"),
        Sym("link2", "file.txt"),
        Sym("link1", "link2"),
    ],
    flags: ["--follow"],
    lines: [],
    // root + file.txt + link2 + link2 resolved + link1 + link1 resolved = 6
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

// Symlink chain to directory: link1 -> link2 -> dir/
// Without --follow: immediate targets compared, both SYMLINK-SKIPPED
case!(chain_to_dir_no_follow {
    orig: [
        Dir("realdir"),
        File("realdir/inside.txt", "inside\n"),
        Sym("link2", "realdir"),
        Sym("link1", "link2"),
    ],
    backup: [
        Dir("realdir"),
        File("realdir/inside.txt", "inside\n"),
        Sym("link2", "realdir"),
        Sym("link1", "link2"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link1",
        "SYMLINK-SKIPPED: a/link2",
    ],
    // root + realdir + realdir/inside.txt + link2 + link1 = 5
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 5,
    skipped: 2,
    errors: 0,
});

// Symlink chain to directory with --follow: traverses through chain into dir
case!(chain_to_dir_with_follow {
    orig: [
        Dir("realdir"),
        File("realdir/inside.txt", "inside\n"),
        Sym("link2", "realdir"),
        Sym("link1", "link2"),
    ],
    backup: [
        Dir("realdir"),
        File("realdir/inside.txt", "inside\n"),
        Sym("link2", "realdir"),
        Sym("link1", "link2"),
    ],
    flags: ["--follow"],
    lines: [],
    // Each symlink to dir adds: symlink + resolved dir + resolved dir contents
    // root + realdir + realdir/inside.txt + link2 + link2/ + link2/inside.txt + link1 + link1/ + link1/inside.txt = 9
    original_processed: 9,
    backup_processed: 9,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 9,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Chain ending in dangling symlink
// ===========================================================================

// Chain ending in dangling: link1 -> link2 -> nonexistent
// Without --follow: immediate targets compared (link2 vs link2), both SYMLINK-SKIPPED
case!(chain_to_dangling_no_follow {
    orig: [
        Sym("link2", "nonexistent"),
        Sym("link1", "link2"),
    ],
    backup: [
        Sym("link2", "nonexistent"),
        Sym("link1", "link2"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link1",
        "SYMLINK-SKIPPED: a/link2",
    ],
    // root + link2 + link1 = 3
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

// Chain ending in dangling with --follow: DANGLING-SYMLINK errors
// The chain resolves through to find the dangling target
case!(chain_to_dangling_with_follow {
    orig: [
        Sym("link2", "nonexistent"),
        Sym("link1", "link2"),
    ],
    backup: [
        Sym("link2", "nonexistent"),
        Sym("link1", "link2"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: a/link1",
        "DANGLING-SYMLINK: b/link1",
        "DANGLING-SYMLINK: a/link2",
        "DANGLING-SYMLINK: b/link2",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 4"],
    output_excludes: [],
    // root + link2 + link2 resolve attempt + link1 + link1 resolve attempt = 5
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 4,
    symmetric: true,
});

// ===========================================================================
// Deep symlink chains (3+ levels)
// ===========================================================================

// 3-level chain: link1 -> link2 -> link3 -> file.txt
case!(deep_chain_three_levels_no_follow {
    orig: [
        File("file.txt", "deep\n"),
        Sym("link3", "file.txt"),
        Sym("link2", "link3"),
        Sym("link1", "link2"),
    ],
    backup: [
        File("file.txt", "deep\n"),
        Sym("link3", "file.txt"),
        Sym("link2", "link3"),
        Sym("link1", "link2"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link1",
        "SYMLINK-SKIPPED: a/link2",
        "SYMLINK-SKIPPED: a/link3",
    ],
    // root + file.txt + link3 + link2 + link1 = 5
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 5,
    skipped: 3,
    errors: 0,
});

// 3-level chain with --follow: all resolve to same file
case!(deep_chain_three_levels_with_follow {
    orig: [
        File("file.txt", "deep\n"),
        Sym("link3", "file.txt"),
        Sym("link2", "link3"),
        Sym("link1", "link2"),
    ],
    backup: [
        File("file.txt", "deep\n"),
        Sym("link3", "file.txt"),
        Sym("link2", "link3"),
        Sym("link1", "link2"),
    ],
    flags: ["--follow"],
    lines: [],
    // root + file.txt + link3 + link3 resolved + link2 + link2 resolved + link1 + link1 resolved = 8
    original_processed: 8,
    backup_processed: 8,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 8,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Chain with content differences
// ===========================================================================

// Chain to file, but file content differs (with --follow)
case!(chain_to_file_different_content_with_follow {
    orig: [
        File("file.txt", "original\n"),
        Sym("link2", "file.txt"),
        Sym("link1", "link2"),
    ],
    backup: [
        File("file.txt", "different\n"),
        Sym("link2", "file.txt"),
        Sym("link1", "link2"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/file.txt",
        "DIFFERENT-FILE [SIZE]: a/link2",
        "DIFFERENT-FILE [SIZE]: a/link1",
    ],
    // root + file.txt + link2 + link2 resolved + link1 + link1 resolved = 6
    original_processed: 6,
    backup_processed: 6,
    missing: 0,
    different: 3,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Directory containing symlinks (with --follow traversal)
// ===========================================================================

// Directory with symlink inside, --follow traverses into dir and finds the symlink
case!(dir_containing_symlink_no_follow {
    orig: [
        Dir("dir"),
        File("dir/target.txt", "target\n"),
        Sym("dir/link", "target.txt"),
    ],
    backup: [
        Dir("dir"),
        File("dir/target.txt", "target\n"),
        Sym("dir/link", "target.txt"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/dir/link",
    ],
    // root + dir + dir/target.txt + dir/link = 4
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

// Directory with symlink inside, --follow resolves the symlink inside the dir
case!(dir_containing_symlink_with_follow {
    orig: [
        Dir("dir"),
        File("dir/target.txt", "target\n"),
        Sym("dir/link", "target.txt"),
    ],
    backup: [
        Dir("dir"),
        File("dir/target.txt", "target\n"),
        Sym("dir/link", "target.txt"),
    ],
    flags: ["--follow"],
    lines: [],
    // root + dir + dir/target.txt + dir/link + dir/link resolved = 5
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 5,
    skipped: 0,
    errors: 0,
});

// Symlink to dir, that dir contains another symlink
// Tests: --follow traverses into dir via symlink, then finds another symlink to follow
case!(symlink_to_dir_containing_symlink_no_follow {
    orig: [
        Dir("realdir"),
        File("realdir/file.txt", "content\n"),
        Sym("realdir/inner_link", "file.txt"),
        Sym("outer_link", "realdir"),
    ],
    backup: [
        Dir("realdir"),
        File("realdir/file.txt", "content\n"),
        Sym("realdir/inner_link", "file.txt"),
        Sym("outer_link", "realdir"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/outer_link",
        "SYMLINK-SKIPPED: a/realdir/inner_link",
    ],
    // root + realdir + realdir/file.txt + realdir/inner_link + outer_link = 5
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 5,
    skipped: 2,
    errors: 0,
});

// Symlink to dir containing symlink, with --follow
// outer_link -> realdir -> contains inner_link -> file.txt
// --follow traverses: outer_link resolves to dir, enters dir, finds inner_link, resolves it
case!(symlink_to_dir_containing_symlink_with_follow {
    orig: [
        Dir("realdir"),
        File("realdir/file.txt", "content\n"),
        Sym("realdir/inner_link", "file.txt"),
        Sym("outer_link", "realdir"),
    ],
    backup: [
        Dir("realdir"),
        File("realdir/file.txt", "content\n"),
        Sym("realdir/inner_link", "file.txt"),
        Sym("outer_link", "realdir"),
    ],
    flags: ["--follow"],
    lines: [],
    // Items: root + realdir + realdir/file.txt + realdir/inner_link + realdir/inner_link resolved
    //        + outer_link + outer_link/ + outer_link/file.txt + outer_link/inner_link + outer_link/inner_link resolved = 10
    original_processed: 10,
    backup_processed: 10,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 10,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Chain differences between orig and backup
// ===========================================================================

// Immediate target differs in chain (link1 -> X vs link1 -> Y)
// Without --follow: DIFFERENT-SYMLINK-TARGET for link1
case!(chain_immediate_target_differs_no_follow {
    orig: [
        File("file.txt", "content\n"),
        Sym("link2_a", "file.txt"),
        Sym("link2_b", "file.txt"),
        Sym("link1", "link2_a"),
    ],
    backup: [
        File("file.txt", "content\n"),
        Sym("link2_a", "file.txt"),
        Sym("link2_b", "file.txt"),
        Sym("link1", "link2_b"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link1",
        "SYMLINK-SKIPPED: a/link1",
        "SYMLINK-SKIPPED: a/link2_a",
        "SYMLINK-SKIPPED: a/link2_b",
    ],
    // root + file.txt + link2_a + link2_b + link1 = 5
    original_processed: 5,
    backup_processed: 5,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 3,
    errors: 0,
});

// Immediate target differs but final target is same (with --follow)
// link1 -> link2_a -> file.txt  vs  link1 -> link2_b -> file.txt
// With --follow: link1 still shows DIFFERENT-SYMLINK-TARGET but content matches
case!(chain_immediate_target_differs_with_follow {
    orig: [
        File("file.txt", "content\n"),
        Sym("link2_a", "file.txt"),
        Sym("link2_b", "file.txt"),
        Sym("link1", "link2_a"),
    ],
    backup: [
        File("file.txt", "content\n"),
        Sym("link2_a", "file.txt"),
        Sym("link2_b", "file.txt"),
        Sym("link1", "link2_b"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link1",
    ],
    // root + file.txt + link2_a + link2_a resolved + link2_b + link2_b resolved + link1 + link1 resolved = 8
    original_processed: 8,
    backup_processed: 8,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 7,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Mixed scenarios
// ===========================================================================

// Chain to dir with content differences inside (--follow)
case!(chain_to_dir_with_differences_follow {
    orig: [
        Dir("realdir"),
        File("realdir/same.txt", "same\n"),
        File("realdir/only_orig.txt", "orig only\n"),
        Sym("link", "realdir"),
    ],
    backup: [
        Dir("realdir"),
        File("realdir/same.txt", "same\n"),
        File("realdir/only_backup.txt", "backup only\n"),
        Sym("link", "realdir"),
    ],
    flags: ["--follow"],
    lines: [
        "MISSING-FILE: a/realdir/only_orig.txt",
        "EXTRA-FILE: b/realdir/only_backup.txt",
        "MISSING-FILE: a/link/only_orig.txt",
        "EXTRA-FILE: b/link/only_backup.txt",
    ],
    // orig: root + realdir + realdir/same.txt + realdir/only_orig.txt + link + link/ + link/same.txt + link/only_orig.txt = 8
    // backup: root + realdir + realdir/same.txt + realdir/only_backup.txt + link + link/ + link/same.txt + link/only_backup.txt = 8
    original_processed: 8,
    backup_processed: 8,
    missing: 2,
    different: 0,
    extras: 2,
    special_files: 0,
    similarities: 6,
    skipped: 0,
    errors: 0,
});
