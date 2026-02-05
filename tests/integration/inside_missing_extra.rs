//! Tests for content inside missing/extra directories.
//!
//! When a directory exists on one side but not the other, all its contents
//! are missing/extra. These tests verify correct handling of various content
//! types (FIFOs, symlinks, unreadable files) inside such directories.

use super::harness::Entry::*;
use crate::case;

// ===========================================================================
// FIFOs Inside Missing/Extra Dirs
// ===========================================================================

// FIFO inside missing directory (orig has dir with FIFO, backup missing dir)
// SPECIAL-FILE is always shown (even without -vv)
// The FIFO counts toward both missing and special_files
case!(fifo_inside_missing_dir {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/mydir",
        "SPECIAL-FILE: a/mydir/pipe",
    ],
    // root + mydir + mydir/file.txt + mydir/pipe = 4
    original_processed: 4,
    // root only
    backup_processed: 1,
    // mydir + mydir/file.txt + mydir/pipe = 3
    missing: 3,
    different: 0,
    extras: 0,
    // FIFO counted as special
    special_files: 1,
    // root
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// FIFO inside missing directory with -vv shows SPECIAL-FILE + MISSING-SPECIAL
case!(fifo_inside_missing_dir_vv {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: ["-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/file.txt",
        "SPECIAL-FILE: a/mydir/pipe",
        "MISSING-SPECIAL: a/mydir/pipe",
    ],
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Multiple FIFOs inside missing/extra dir
case!(multiple_fifos_inside_missing_dir {
    orig: [
        Dir("mydir"),
        Fifo("mydir/pipe1"),
        Fifo("mydir/pipe2"),
        Fifo("mydir/pipe3"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/mydir",
        "SPECIAL-FILE: a/mydir/pipe1",
        "SPECIAL-FILE: a/mydir/pipe2",
        "SPECIAL-FILE: a/mydir/pipe3",
    ],
    // root + mydir + 3 pipes = 5
    original_processed: 5,
    backup_processed: 1,
    // mydir + 3 pipes = 4
    missing: 4,
    different: 0,
    extras: 0,
    special_files: 3,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// FIFO nested deeply inside missing/extra dir
case!(fifo_nested_deeply_inside_missing_dir {
    orig: [
        Dir("level1"),
        Dir("level1/level2"),
        Dir("level1/level2/level3"),
        Fifo("level1/level2/level3/deep_pipe"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/level1",
        "SPECIAL-FILE: a/level1/level2/level3/deep_pipe",
    ],
    // root + level1 + level2 + level3 + deep_pipe = 5
    original_processed: 5,
    backup_processed: 1,
    // level1 + level2 + level3 + deep_pipe = 4
    missing: 4,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// FIFO nested deeply with -vv shows all levels + MISSING-SPECIAL
case!(fifo_nested_deeply_inside_missing_dir_vv {
    orig: [
        Dir("level1"),
        Dir("level1/level2"),
        Dir("level1/level2/level3"),
        Fifo("level1/level2/level3/deep_pipe"),
    ],
    backup: [],
    flags: ["-vv"],
    lines: [
        "MISSING-DIR: a/level1",
        "MISSING-DIR: a/level1/level2",
        "MISSING-DIR: a/level1/level2/level3",
        "SPECIAL-FILE: a/level1/level2/level3/deep_pipe",
        "MISSING-SPECIAL: a/level1/level2/level3/deep_pipe",
    ],
    original_processed: 5,
    backup_processed: 1,
    missing: 4,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Errors Inside Missing/Extra Dirs (need --all to force reading)
// ===========================================================================

// Unreadable file inside missing directory
// Note: When a directory is entirely missing from backup, we only stat (not read)
// the original's contents to list them. So unreadable files don't cause errors here.
// This test verifies that behavior - no error, file still counted as missing.
case!(unreadable_file_inside_missing_dir {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        FileUnreadable("mydir/secret.txt", "secret\n"),
    ],
    backup: [],
    flags: ["--all"],
    lines: [
        "MISSING-DIR: a/mydir",
    ],
    // root + mydir + ok.txt + secret.txt = 4
    original_processed: 4,
    backup_processed: 1,
    // mydir + ok.txt + secret.txt = 3 (can stat even if can't read)
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Unreadable file inside missing directory with -vv
// Still no error since we only stat, not read
case!(unreadable_file_inside_missing_dir_vv {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        FileUnreadable("mydir/secret.txt", "secret\n"),
    ],
    backup: [],
    flags: ["--all", "-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/ok.txt",
        "MISSING-FILE: a/mydir/secret.txt",
    ],
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

// Unreadable subdir inside missing directory
case!(unreadable_subdir_inside_missing_dir {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        DirUnreadable("mydir/secret_dir"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/mydir",
        "ERROR: mydir/secret_dir",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: [],
    // root + mydir + ok.txt + secret_dir = 4
    original_processed: 4,
    backup_processed: 1,
    // mydir + ok.txt + secret_dir = 3
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: true,
});

// Unreadable subdir inside missing directory with -vv shows ERROR + MISSING-ERROR
case!(unreadable_subdir_inside_missing_dir_vv {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        DirUnreadable("mydir/secret_dir"),
    ],
    backup: [],
    flags: ["-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/ok.txt",
        "ERROR: mydir/secret_dir",
        "MISSING-ERROR: a/mydir/secret_dir",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: [],
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: true,
});

// ===========================================================================
// Dangling Symlinks Inside Missing/Extra Dirs
// ===========================================================================

// Dangling symlink inside missing directory (no --follow)
// Without --follow, symlinks are just counted as missing (no SYMLINK-SKIPPED
// because there's no comparison happening - the whole dir is missing)
case!(dangling_symlink_inside_missing_dir_no_follow {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        Sym("mydir/dangling", "nonexistent"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/mydir",
    ],
    // root + mydir + ok.txt + dangling = 4
    original_processed: 4,
    backup_processed: 1,
    // mydir + ok.txt + dangling = 3
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Dangling symlink inside missing directory with -vv (no --follow)
case!(dangling_symlink_inside_missing_dir_no_follow_vv {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        Sym("mydir/dangling", "nonexistent"),
    ],
    backup: [],
    flags: ["-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/ok.txt",
        "MISSING-SYMLINK: a/mydir/dangling",
    ],
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

// Dangling symlink inside missing directory (with --follow)
// With --follow, dangling symlinks produce DANGLING-SYMLINK errors
// DANGLING-SYMLINK is always shown, with -vv also MISSING-SYMLINK
case!(dangling_symlink_inside_missing_dir_with_follow {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        Sym("mydir/dangling", "nonexistent"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-DIR: a/mydir",
        "DANGLING-SYMLINK: a/mydir/dangling",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: [],
    // root + mydir + ok.txt + dangling + dangling resolution attempt = 5
    original_processed: 5,
    backup_processed: 1,
    // mydir + ok.txt + dangling = 3
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: true,
});

// Dangling symlink inside missing directory with --follow -vv
case!(dangling_symlink_inside_missing_dir_with_follow_vv {
    orig: [
        Dir("mydir"),
        File("mydir/ok.txt", "ok\n"),
        Sym("mydir/dangling", "nonexistent"),
    ],
    backup: [],
    flags: ["--follow", "-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/ok.txt",
        "DANGLING-SYMLINK: a/mydir/dangling",
        "MISSING-SYMLINK: a/mydir/dangling",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: [],
    original_processed: 5,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: true,
});

// ===========================================================================
// Counting Inside Missing/Extra Dirs
// ===========================================================================

// Counting with --follow when symlinks inside missing/extra dir
// Symlink to file gets followed and resolved content is also counted
case!(counting_with_follow_symlink_inside_missing_dir {
    orig: [
        Dir("mydir"),
        File("mydir/target.txt", "content\n"),
        Sym("mydir/link", "target.txt"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-DIR: a/mydir",
    ],
    // root + mydir + target.txt + link + link resolved = 5
    original_processed: 5,
    backup_processed: 1,
    // mydir + target.txt + link + link resolved content = 4
    missing: 4,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Counting with --follow -vv shows all items
case!(counting_with_follow_symlink_inside_missing_dir_vv {
    orig: [
        Dir("mydir"),
        File("mydir/target.txt", "content\n"),
        Sym("mydir/link", "target.txt"),
    ],
    backup: [],
    flags: ["--follow", "-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/target.txt",
        "MISSING-SYMLINK: a/mydir/link",
        "MISSING-FILE: a/mydir/link",
    ],
    original_processed: 5,
    backup_processed: 1,
    missing: 4,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Mixed content (files, dirs, symlinks, FIFOs) inside missing dir - verify counts
// Without --follow, symlinks are just counted (no SYMLINK-SKIPPED)
case!(mixed_content_inside_missing_dir {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Dir("mydir/subdir"),
        File("mydir/subdir/nested.txt", "nested\n"),
        Sym("mydir/link", "file.txt"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/mydir",
        "SPECIAL-FILE: a/mydir/pipe",
    ],
    // root + mydir + file.txt + subdir + nested.txt + link + pipe = 7
    original_processed: 7,
    backup_processed: 1,
    // mydir + file.txt + subdir + nested.txt + link + pipe = 6
    missing: 6,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Mixed content with -vv shows all items individually
case!(mixed_content_inside_missing_dir_vv {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Dir("mydir/subdir"),
        File("mydir/subdir/nested.txt", "nested\n"),
        Sym("mydir/link", "file.txt"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: ["-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/file.txt",
        "MISSING-DIR: a/mydir/subdir",
        "MISSING-FILE: a/mydir/subdir/nested.txt",
        "MISSING-SYMLINK: a/mydir/link",
        "SPECIAL-FILE: a/mydir/pipe",
        "MISSING-SPECIAL: a/mydir/pipe",
    ],
    original_processed: 7,
    backup_processed: 1,
    missing: 6,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Mixed content with --follow
// Symlink is followed so no skipped, and resolved content is counted
case!(mixed_content_inside_missing_dir_follow {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Dir("mydir/subdir"),
        File("mydir/subdir/nested.txt", "nested\n"),
        Sym("mydir/link", "file.txt"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-DIR: a/mydir",
        "SPECIAL-FILE: a/mydir/pipe",
    ],
    // root + mydir + file.txt + subdir + nested.txt + link + link resolved + pipe = 8
    original_processed: 8,
    backup_processed: 1,
    // mydir + file.txt + subdir + nested.txt + link + link resolved + pipe = 7
    missing: 7,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Verbosity for Missing/Extra Dir Contents
// ===========================================================================

// -v (single) behavior for missing dir contents
// Single -v should behave like no -v for missing/extra dir contents (only top-level shown)
case!(single_v_missing_dir_contents {
    orig: [
        Dir("mydir"),
        File("mydir/file1.txt", "content1\n"),
        File("mydir/file2.txt", "content2\n"),
        Dir("mydir/subdir"),
        File("mydir/subdir/nested.txt", "nested\n"),
    ],
    backup: [],
    flags: ["-v"],
    lines: [
        "MISSING-DIR: a/mydir",
    ],
    // Verify -v doesn't show individual children (same as no -v)
    debug_contains: ["Comparing"],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["MISSING-FILE"],
    // root + mydir + file1 + file2 + subdir + nested = 6
    original_processed: 6,
    backup_processed: 1,
    // mydir + file1 + file2 + subdir + nested = 5
    missing: 5,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
    symmetric: true,
});

// Verify FIFO inside missing dir output at verbosity levels
// No -v: MISSING-DIR + SPECIAL-FILE (SPECIAL-FILE always shown)
case!(fifo_verbosity_no_v {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/mydir",
        "SPECIAL-FILE: a/mydir/pipe",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["MISSING-FILE: a/mydir/file.txt"],
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
    symmetric: true,
});

// -v: same as no -v for missing dir contents (SPECIAL-FILE still shown)
case!(fifo_verbosity_single_v {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: ["-v"],
    lines: [
        "MISSING-DIR: a/mydir",
        "SPECIAL-FILE: a/mydir/pipe",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["MISSING-FILE: a/mydir/file.txt"],
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
    symmetric: true,
});

// -vv: shows SPECIAL-FILE + MISSING-SPECIAL
case!(fifo_verbosity_vv {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Fifo("mydir/pipe"),
    ],
    backup: [],
    flags: ["-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/file.txt",
        "SPECIAL-FILE: a/mydir/pipe",
        "MISSING-SPECIAL: a/mydir/pipe",
    ],
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Verify dangling symlink inside missing dir output at verbosity levels
// No -v with no --follow: just MISSING-DIR (symlink is just missing content)
case!(dangling_verbosity_no_v_no_follow {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Sym("mydir/dangling", "nonexistent"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/mydir",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["MISSING-FILE", "MISSING-SYMLINK", "DANGLING", "SYMLINK-SKIPPED"],
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
    symmetric: true,
});

// No -v with --follow: MISSING-DIR + DANGLING-SYMLINK error
case!(dangling_verbosity_no_v_with_follow {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Sym("mydir/dangling", "nonexistent"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-DIR: a/mydir",
        "DANGLING-SYMLINK: a/mydir/dangling",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: ["MISSING-FILE", "MISSING-SYMLINK"],
    original_processed: 5,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: true,
});

// -vv with --follow: shows DANGLING-SYMLINK + MISSING-SYMLINK
case!(dangling_verbosity_vv_with_follow {
    orig: [
        Dir("mydir"),
        File("mydir/file.txt", "content\n"),
        Sym("mydir/dangling", "nonexistent"),
    ],
    backup: [],
    flags: ["--follow", "-vv"],
    lines: [
        "MISSING-DIR: a/mydir",
        "MISSING-FILE: a/mydir/file.txt",
        "DANGLING-SYMLINK: a/mydir/dangling",
        "MISSING-SYMLINK: a/mydir/dangling",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: [],
    original_processed: 5,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: true,
});
