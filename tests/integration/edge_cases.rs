//! Edge case integration tests using the case! macro infrastructure.
//!
//! Tests cover: empty directories, zero-byte files, extras with no originals,
//! mixed results, deep trees, and special files (FIFOs, device symlinks).

use super::harness::Entry::*;
use super::{cmd, stdout_of, testdata};
use crate::case;
use predicates::prelude::*;

// ===========================================================================
// Empty directories
// ===========================================================================

// Both orig and backup are empty directories
case!(empty_directories {
    orig: [],
    backup: [],
    flags: [],
    lines: [],
    original_processed: 1,
    backup_processed: 1,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Zero-byte files
// ===========================================================================

// Two identical empty files with --all flag (hash comparison)
case!(zero_byte_files_with_all {
    orig: [
        File("empty.txt", ""),
    ],
    backup: [
        File("empty.txt", ""),
    ],
    flags: ["--all", "-vv"],
    lines: [],
    debug_contains: [],
    debug_excludes: [],
    // BLAKE3("") = af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
    output_contains: ["af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"],
    output_excludes: ["DIFFERENT-FILE"],
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

// Two identical empty files with sampling
case!(zero_byte_files_with_sampling {
    orig: [
        File("empty.txt", ""),
    ],
    backup: [
        File("empty.txt", ""),
    ],
    flags: ["-s", "10"],
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

// ===========================================================================
// Extras with zero original items
// ===========================================================================

// a/ is empty, b/ has files and directory → exit 1 due to extras
// Replicates testdata/extras_only: b has extra1.txt, extra_dir/, extra_dir/extra2.txt
case!(extras_with_zero_originals {
    orig: [],
    backup: [
        File("extra1.txt", "extra\n"),
        Dir("extra_dir"),
        File("extra_dir/extra2.txt", "more extra\n"),
    ],
    flags: [],
    lines: [
        "EXTRA-FILE: b/extra1.txt",
        "EXTRA-DIR: b/extra_dir",
    ],
    original_processed: 1,
    // root + extra1.txt + extra_dir + extra2.txt = 4
    backup_processed: 4,
    missing: 0,
    different: 0,
    // extra1.txt + extra_dir + extra2.txt = 3
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Mixed results in one directory
// ===========================================================================

// Various outcomes in one test: match, size diff, content diff
case!(mixed_results_per_file {
    orig: [
        File("match.txt", "same content\n"),
        File("size_diff.txt", "short\n"),
        File("content_diff.txt", "aaaa"),
    ],
    backup: [
        File("match.txt", "same content\n"),
        File("size_diff.txt", "this is much longer content\n"),
        File("content_diff.txt", "bbbb"),
    ],
    flags: ["-s", "10", "--all"],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/size_diff.txt",
        "DIFFERENT-FILE [SAMPLE]: a/content_diff.txt",
    ],
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 2,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Deeply nested identical trees
// ===========================================================================

// Multi-level nesting, all identical
case!(deep_identical_tree {
    orig: [
        File("top.txt", "top level\n"),
        Dir("l1"),
        File("l1/mid.txt", "mid level\n"),
        Dir("l1/l2"),
        Dir("l1/l2/l3"),
        File("l1/l2/l3/deep.txt", "deep level\n"),
    ],
    backup: [
        File("top.txt", "top level\n"),
        Dir("l1"),
        File("l1/mid.txt", "mid level\n"),
        Dir("l1/l2"),
        Dir("l1/l2/l3"),
        File("l1/l2/l3/deep.txt", "deep level\n"),
    ],
    flags: ["--all"],
    lines: [],
    // root + top.txt + l1 + mid.txt + l2 + l3 + deep.txt = 7
    original_processed: 7,
    backup_processed: 7,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 7,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Special file tests (FIFOs)
// ===========================================================================

// FIFO missing from backup should be SPECIAL-FILE, not MISSING-FILE
case!(special_file_missing_from_backup {
    orig: [
        Fifo("my_fifo"),
        File("ok.txt", "ok\n"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: a/my_fifo",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["MISSING-FILE: my_fifo"],
    original_processed: 3,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// FIFO extra in backup should be SPECIAL-FILE, not EXTRA-FILE
case!(special_file_extra_in_backup {
    orig: [
        File("ok.txt", "ok\n"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        Fifo("my_fifo"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/my_fifo",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["EXTRA-FILE: my_fifo"],
    original_processed: 2,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// One side has FIFO, other has symlink → SPECIAL-FILE
// The FIFO is a special file; the symlink and target.txt are both extras
case!(special_file_vs_symlink {
    orig: [
        Fifo("entry"),
    ],
    backup: [
        File("target.txt", "content\n"),
        Sym("entry", "target.txt"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target.txt",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["DIFFERENT-SYMLINK-STATUS"],
    original_processed: 2,
    backup_processed: 3,
    missing: 0,
    different: 0,
    // Both the symlink and target.txt are extras
    extras: 2,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Reverse: original is symlink, backup is FIFO
// The FIFO is a special file; the symlink and target.txt are both missing
case!(symlink_vs_special_file {
    orig: [
        File("target.txt", "content\n"),
        Sym("entry", "target.txt"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target.txt",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: [],
    output_excludes: ["DIFFERENT-SYMLINK-STATUS"],
    original_processed: 3,
    backup_processed: 2,
    // Both the symlink and target.txt are missing
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Special files via symlinks to /dev
// ===========================================================================

// Symlink to /dev/urandom is a character device with --follow
// With --follow, symlinks count as 2 items: the symlink entry + resolved target
case!(special_file_via_symlink_follow {
    orig: [
        File("ok.txt", "ok\n"),
        Sym("special", "/dev/urandom"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        Sym("special", "/dev/urandom"),
    ],
    flags: ["--follow"],
    lines: [
        "SPECIAL-FILE: a/special",
        "SPECIAL-FILE: b/special",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Special files: 2"],
    output_excludes: ["ERROR:"],
    // root + ok.txt + special symlink (2 with --follow) = 4
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 2,
    // root + ok.txt + special symlink entry = 3 similarities
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// Symlink to /dev/null missing from backup with --follow
// With --follow, symlinks count as 2 items: the symlink entry + resolved target
case!(special_files_missing_from_backup {
    orig: [
        Dir("sub"),
        File("sub/ok.txt", "ok\n"),
        Sym("sub/devnull", "/dev/null"),
    ],
    backup: [
        Dir("sub"),
        File("sub/ok.txt", "ok\n"),
    ],
    flags: ["--follow"],
    lines: [
        "MISSING-SYMLINK: a/sub/devnull",
    ],
    // root + sub + ok.txt + devnull symlink (2 with --follow) = 5
    original_processed: 5,
    backup_processed: 3,
    missing: 1,
    different: 0,
    extras: 0,
    // The resolved /dev/null is a special file, counted
    special_files: 1,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// Symlink to /dev/null extra in backup with --follow
// With --follow, symlinks count as 2 items: the symlink entry + resolved target
case!(special_files_extra_in_backup {
    orig: [
        Dir("sub"),
        File("sub/ok.txt", "ok\n"),
    ],
    backup: [
        Dir("sub"),
        File("sub/ok.txt", "ok\n"),
        Sym("sub/devnull", "/dev/null"),
    ],
    flags: ["--follow"],
    lines: [
        "EXTRA-SYMLINK: b/sub/devnull",
    ],
    original_processed: 3,
    // root + sub + ok.txt + devnull symlink (2 with --follow) = 5
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 1,
    // The resolved /dev/null is a special file, counted
    special_files: 1,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Symlink to /dev directory with --follow
// ===========================================================================

// Both sides symlink to /dev/ — contains device files
// This test cannot use case! macro because /dev contents vary by system
#[test]
fn symlink_to_dev_dir_with_follow() {
    use std::os::unix::fs::symlink;

    let tmp = std::env::temp_dir().join("bv_test_dev_symlink");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create symlinks to /dev in both directories
    symlink("/dev", a.join("dev")).unwrap();
    symlink("/dev", b.join("dev")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    // Don't check exit code - /dev may contain items that cause non-zero exit
    let output = cmd()
        .args([&a_str, &b_str, "--follow"])
        .output()
        .expect("Failed to run vfy");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let _ = std::fs::remove_dir_all(&tmp);

    // Device files like /dev/null, /dev/zero should be SPECIAL-FILE, not ERROR
    assert!(
        stdout.contains("SPECIAL-FILE:"),
        "Expected some SPECIAL-FILE entries for /dev device files, got:\n{}",
        stdout
    );
    // Check that typical device files aren't reported as errors
    assert!(
        !stdout.contains("ERROR: a/dev/null") && !stdout.contains("ERROR: b/dev/null"),
        "Should not report ERROR for /dev/null, got:\n{}",
        stdout
    );
}

// ===========================================================================
// Tests that remain manual (need special handling)
// ===========================================================================

#[test]
fn same_directory_warning() {
    let (a, _) = testdata("identical");
    cmd()
        .args([&a, &a])
        .assert()
        .success()
        .stderr(predicate::str::contains("same directory"));
}

#[test]
fn output_is_sorted() {
    // sorted/ has alpha.txt, bravo.txt, charlie.txt in a/ but only charlie.txt in b/
    // MISSING-FILE lines should appear in alphabetical order
    let (a, b) = testdata("sorted");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    let missing_lines: Vec<&str> = output
        .lines()
        .filter(|l| l.contains("MISSING-FILE:"))
        .collect();

    assert_eq!(
        missing_lines.len(),
        2,
        "Expected 2 MISSING-FILE lines, got: {:?}",
        missing_lines
    );

    // alpha.txt must come before bravo.txt
    let alpha_pos = output.find("alpha.txt").expect("alpha.txt not in output");
    let bravo_pos = output.find("bravo.txt").expect("bravo.txt not in output");
    assert!(
        alpha_pos < bravo_pos,
        "alpha.txt should appear before bravo.txt in sorted output"
    );
}
