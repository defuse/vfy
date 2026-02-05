use super::harness::{setup_legacy_test_dirs, Entry::*};
use super::cmd;
use crate::case;
use predicates::prelude::*;

// ===========================================================================
// case! macro tests for unreadable files and type mismatches
// ===========================================================================

// Unreadable file in backup reports ERROR and counts original as MISSING
// (can't verify the backup matches, so original is effectively missing)
// Tests with --all flag to ensure hash comparison is attempted
// symmetric: false because unreadable-in-orig has different behavior (no EXTRA)
case!(unreadable_file_in_backup {
    orig: [
        File("file.txt", "readable\n"),
        File("noperm.txt", "secret\n"),
    ],
    backup: [
        File("file.txt", "readable\n"),
        FileUnreadable("noperm.txt", "secret\n"),
    ],
    flags: ["--all"],
    lines: [
        "ERROR: noperm.txt",
        "MISSING-FILE: a/noperm.txt",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1", "Missing: 1"],
    output_excludes: ["DIFFERENT-FILE", "EXTRA-FILE"],
    original_processed: 3,
    backup_processed: 3,
    // noperm.txt is counted as missing since we can't verify the backup
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    // root + file.txt match
    similarities: 2,
    skipped: 0,
    errors: 1,
    symmetric: false,
});

// Unreadable directory in original reports ERROR only
// Note: DirUnreadable creates an empty unreadable dir.
// When orig dir is unreadable, we can't verify - but we should NOT tell the user
// the backup contents are "extra" because they might be valid backups.
// This is the safe/conservative behavior - don't suggest deleting potentially valid data.
// symmetric: false because unreadable-in-backup has different behavior (counts as missing)
case!(unreadable_dir_in_original {
    orig: [
        File("ok.txt", "ok\n"),
        DirUnreadable("noread_dir"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        Dir("noread_dir"),
        File("noread_dir/hidden.txt", "hidden\n"),
    ],
    flags: [],
    lines: [
        "ERROR: noread_dir",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: ["EXTRA-DIR", "EXTRA-FILE", "MISSING"],
    original_processed: 3,
    // backup's noread_dir and hidden.txt not counted (can't verify, not processed)
    backup_processed: 2,
    missing: 0,
    different: 0,
    // NOT counted as extras - we can't verify, but shouldn't suggest deletion
    extras: 0,
    special_files: 0,
    // root + ok.txt match
    similarities: 2,
    skipped: 0,
    errors: 1,
    symmetric: false,
});

// Unreadable directory in backup reports ERROR and counts original as MISSING
// When backup dir is unreadable, we can't verify the backup has these files,
// so the original contents are reported as "missing" (conservative/safe).
// symmetric: false because unreadable-in-orig has different behavior (no EXTRA)
case!(unreadable_dir_in_backup {
    orig: [
        File("ok.txt", "ok\n"),
        Dir("noread_dir"),
        File("noread_dir/file.txt", "test\n"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        DirUnreadable("noread_dir"),
    ],
    flags: [],
    lines: [
        "ERROR: noread_dir",
        "MISSING-DIR: a/noread_dir",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1", "Missing: 2"],
    output_excludes: ["EXTRA-DIR", "EXTRA-FILE"],
    original_processed: 4,
    backup_processed: 3,
    // noread_dir + file.txt = 2 missing (can't verify they're backed up)
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    // root + ok.txt match
    similarities: 2,
    skipped: 0,
    errors: 1,
    symmetric: false,
});

// Type mismatch: replicates testdata/type_mismatch exactly
// a/ has: name_a/ (dir with child.txt), name_b (file), same.txt
// b/ has: name_a (file), name_b/ (dir with child.txt), same.txt
// Two type mismatches: name_a (dir vs file) and name_b (file vs dir)
// Without -vv, top-level missing/extra are shown but not children inside dirs
//
// This test covers:
// - dir_in_original_file_in_backup: FILE-DIR-MISMATCH for name_a, "dir vs file"
// - file_in_original_dir_in_backup: FILE-DIR-MISMATCH for name_b, "file vs dir"
// - type_mismatch_summary: all counts verified
case!(type_mismatch_combined {
    orig: [
        Dir("name_a"),
        File("name_a/child.txt", "inside dir"),
        File("name_b", "i am a file"),
        File("same.txt", "same"),
    ],
    backup: [
        File("name_a", "i am a file"),
        Dir("name_b"),
        File("name_b/child.txt", "inside dir"),
        File("same.txt", "same"),
    ],
    flags: [],
    lines: [
        "FILE-DIR-MISMATCH: a/name_a",
        "MISSING-DIR: a/name_a",
        "EXTRA-FILE: b/name_a",
        "FILE-DIR-MISMATCH: a/name_b",
        "MISSING-FILE: a/name_b",
        "EXTRA-DIR: b/name_b",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["dir vs file", "file vs dir"],
    output_excludes: [],
    // root + name_a dir + name_a/child.txt + name_b file + same.txt = 5
    original_processed: 5,
    // root + name_a file + name_b dir + name_b/child.txt + same.txt = 5
    backup_processed: 5,
    // name_a (missing-dir) + name_a/child.txt + name_b (missing-file) = 3
    missing: 3,
    // name_a (type) + name_b (type) = 2
    different: 2,
    // name_a (extra-file) + name_b (extra-dir) + name_b/child.txt = 3
    extras: 3,
    special_files: 0,
    // root + same.txt = 2
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// Type mismatch with -vv shows all missing/extra items individually
// This test covers:
// - type_mismatch_dir_orig_counts_missing_contents: MISSING-FILE for child.txt
// - type_mismatch_dir_backup_counts_extra_contents: EXTRA-FILE for child.txt
case!(type_mismatch_combined_vv {
    orig: [
        Dir("name_a"),
        File("name_a/child.txt", "inside dir"),
        File("name_b", "i am a file"),
        File("same.txt", "same"),
    ],
    backup: [
        File("name_a", "i am a file"),
        Dir("name_b"),
        File("name_b/child.txt", "inside dir"),
        File("same.txt", "same"),
    ],
    flags: ["-vv"],
    lines: [
        "FILE-DIR-MISMATCH: a/name_a",
        "MISSING-DIR: a/name_a",
        "MISSING-FILE: a/name_a/child.txt",
        "EXTRA-FILE: b/name_a",
        "FILE-DIR-MISMATCH: a/name_b",
        "MISSING-FILE: a/name_b",
        "EXTRA-DIR: b/name_b",
        "EXTRA-FILE: b/name_b/child.txt",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["dir vs file", "file vs dir"],
    output_excludes: [],
    original_processed: 5,
    backup_processed: 5,
    missing: 3,
    different: 2,
    extras: 3,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Unreadable file - asymmetric behavior (safe/conservative)
// ===========================================================================
//
// When we can't read a file, the behavior depends on WHICH side is unreadable:
// - Unreadable backup → original counted as MISSING (conservative: alerts user)
// - Unreadable original → backup NOT counted as EXTRA (safe: don't suggest deletion)
//
// This asymmetry is intentional:
// - False "missing" → user investigates, finds file is actually there, no harm
// - False "extra" → user might delete valid backup data, potential data loss

// Unreadable file in original → just ERROR, backup NOT counted as extra
// We can't read the original to verify, but we shouldn't tell the user
// the backup file is "extra" because it might be a valid backup.
// symmetric: false because unreadable-in-backup has different behavior
case!(unreadable_file_in_original {
    orig: [
        File("ok.txt", "ok\n"),
        FileUnreadable("noperm.txt", "secret\n"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        File("noperm.txt", "secret\n"),
    ],
    flags: ["--all"],
    lines: [
        "ERROR: noperm.txt",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1"],
    output_excludes: ["MISSING-FILE", "EXTRA-FILE", "DIFFERENT-FILE"],
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    // NOT counted as extra - safe behavior
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 1,
    symmetric: false,
});

// Unreadable file in backup → original file counted as MISSING
// We can't verify the backup matches, so we conservatively report the
// original as "missing" (not backed up). This alerts the user to investigate.
// symmetric: false because unreadable-in-orig has different behavior
case!(unreadable_file_in_backup_counts_missing {
    orig: [
        File("ok.txt", "ok\n"),
        File("noperm.txt", "secret\n"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        FileUnreadable("noperm.txt", "secret\n"),
    ],
    flags: ["--all"],
    lines: [
        "ERROR: noperm.txt",
        "MISSING-FILE: a/noperm.txt",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1", "Missing: 1"],
    output_excludes: ["EXTRA-FILE", "DIFFERENT-FILE"],
    original_processed: 3,
    backup_processed: 3,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 1,
    symmetric: false,
});

// Permission error during sampling (-s flag)
// Sampling requires reading file content, so unreadable backup file errors.
// Original file counted as missing (conservative - alerts user to investigate).
// symmetric: false for asymmetric error behavior
case!(unreadable_file_with_sampling {
    orig: [
        File("file.txt", "content here\n"),
    ],
    backup: [
        FileUnreadable("file.txt", "content here\n"),
    ],
    flags: ["-s", "5"],
    lines: [
        "ERROR: file.txt",
        "MISSING-FILE: a/file.txt",
    ],
    debug_contains: [],
    debug_excludes: [],
    output_contains: ["Errors: 1", "Missing: 1"],
    output_excludes: ["EXTRA-FILE", "DIFFERENT-FILE"],
    original_processed: 2,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
    symmetric: false,
});

// ===========================================================================
// CLI validation tests (these test argument validation, not file comparison)
// ===========================================================================

#[test]
fn nonexistent_original_exits_2() {
    cmd()
        .args(["/nonexistent/dir/orig", "/tmp"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Cannot resolve"));
}

#[test]
fn original_is_file_not_dir() {
    // Use an existing file as the "original" argument
    let file_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("Cargo.toml")
        .to_str()
        .unwrap()
        .to_string();
    let (_tmp, _a, b) = setup_legacy_test_dirs(&[], &[]);

    cmd()
        .args([&file_path, &b])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is not a directory"));
}

#[test]
fn nonexistent_backup_exits_2() {
    let (_tmp, a, _b) = setup_legacy_test_dirs(&[], &[]);
    cmd()
        .args([&a, "/nonexistent/dir/backup"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Cannot resolve"));
}

#[test]
fn backup_is_file_not_dir() {
    let (_tmp, a, _b) = setup_legacy_test_dirs(&[], &[]);
    let file_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("Cargo.toml")
        .to_str()
        .unwrap()
        .to_string();

    cmd()
        .args([&a, &file_path])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is not a directory"));
}
