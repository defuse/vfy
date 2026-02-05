//! Critical pre-release tests covering edge cases that must work correctly.
//!
//! These tests verify:
//! - Special characters in filenames (spaces, unicode, emoji, shell metacharacters)
//! - Empty vs non-empty file comparison
//! - Long filenames and deeply nested paths
//! - Exit code precedence

use super::cmd;
use super::harness::Entry::*;
use crate::case;

// =============================================================================
// Special Characters in Filenames
// =============================================================================

// Spaces in filename - identical
case!(filename_with_spaces {
    orig: [File("file with spaces.txt", "content")],
    backup: [File("file with spaces.txt", "content")],
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

// Spaces in filename - different size
case!(filename_with_spaces_different {
    orig: [File("file with spaces.txt", "short")],
    backup: [File("file with spaces.txt", "this is longer content")],
    flags: [],
    lines: ["DIFFERENT-FILE [SIZE]: a/file with spaces.txt"],
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

// Unicode in filename - identical
case!(filename_with_unicode {
    orig: [File("日本語.txt", "content")],
    backup: [File("日本語.txt", "content")],
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

// Unicode in filename - missing
case!(filename_with_unicode_missing {
    orig: [File("日本語.txt", "content")],
    backup: [],
    flags: [],
    lines: ["MISSING-FILE: a/日本語.txt"],
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

// Emoji in filename - identical
case!(filename_with_emoji {
    orig: [File("📁data.txt", "content")],
    backup: [File("📁data.txt", "content")],
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

// Emoji in filename - different size
case!(filename_with_emoji_different {
    orig: [File("📁data.txt", "short")],
    backup: [File("📁data.txt", "this is longer content")],
    flags: [],
    lines: ["DIFFERENT-FILE [SIZE]: a/📁data.txt"],
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

// Shell metacharacters in filename - identical
// Note: backticks are valid in filenames on ext4/most Unix filesystems
case!(filename_with_shell_chars {
    orig: [File("$var'\"*.txt", "content")],
    backup: [File("$var'\"*.txt", "content")],
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

// Shell metacharacters in filename - extra in backup
case!(filename_with_shell_chars_extra {
    orig: [],
    backup: [File("$var'\"*.txt", "content")],
    flags: [],
    lines: ["EXTRA-FILE: b/$var'\"*.txt"],
    original_processed: 1,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// =============================================================================
// Empty vs Non-empty File Comparison
// =============================================================================

// Empty original, non-empty backup (size difference)
case!(empty_vs_nonempty {
    orig: [File("data.txt", "")],
    backup: [File("data.txt", "x")],
    flags: [],
    lines: ["DIFFERENT-FILE [SIZE]: a/data.txt"],
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

// Non-empty original, empty backup (size difference)
case!(nonempty_vs_empty {
    orig: [File("data.txt", "content")],
    backup: [File("data.txt", "")],
    flags: [],
    lines: ["DIFFERENT-FILE [SIZE]: a/data.txt"],
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

// Both empty - should be identical
case!(both_empty {
    orig: [File("data.txt", "")],
    backup: [File("data.txt", "")],
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

// =============================================================================
// Long Filenames/Paths
// =============================================================================

#[test]
fn long_filename_max_length() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // 255 chars is typical max filename on ext4/most filesystems
    let long_name = "x".repeat(251) + ".txt"; // 255 total with extension
    std::fs::write(a.join(&long_name), "content").unwrap();
    std::fs::write(b.join(&long_name), "content").unwrap();

    cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn long_filename_different_size() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let long_name = "x".repeat(251) + ".txt";
    std::fs::write(a.join(&long_name), "short").unwrap();
    std::fs::write(b.join(&long_name), "this is longer content").unwrap();

    let assert = cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .code(1);

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        output.contains("DIFFERENT-FILE [SIZE]"),
        "Expected DIFFERENT-FILE [SIZE] in output"
    );
    assert!(output.contains(&long_name), "Expected long filename in output");
}

#[test]
fn deeply_nested_directory_structure() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    // Create 50-level deep structure
    let mut nested_path_a = a.clone();
    for i in 0..50 {
        nested_path_a = nested_path_a.join(format!("d{}", i));
    }
    std::fs::create_dir_all(&nested_path_a).unwrap();
    std::fs::write(nested_path_a.join("file.txt"), "deep").unwrap();

    // Same for b
    let mut nested_path_b = b.clone();
    for i in 0..50 {
        nested_path_b = nested_path_b.join(format!("d{}", i));
    }
    std::fs::create_dir_all(&nested_path_b).unwrap();
    std::fs::write(nested_path_b.join("file.txt"), "deep").unwrap();

    cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn deeply_nested_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    // Create 50-level deep structure in a
    let mut nested_path_a = a.clone();
    for i in 0..50 {
        nested_path_a = nested_path_a.join(format!("d{}", i));
    }
    std::fs::create_dir_all(&nested_path_a).unwrap();
    std::fs::write(nested_path_a.join("file.txt"), "deep").unwrap();

    // Create same structure in b but without the file
    let mut nested_path_b = b.clone();
    for i in 0..50 {
        nested_path_b = nested_path_b.join(format!("d{}", i));
    }
    std::fs::create_dir_all(&nested_path_b).unwrap();
    // No file.txt in b

    let assert = cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .code(1);

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        output.contains("MISSING-FILE"),
        "Expected MISSING-FILE in output"
    );
    assert!(output.contains("file.txt"), "Expected file.txt in output");
}

// =============================================================================
// Exit Code Precedence
// =============================================================================

#[test]
fn exit_0_only_when_perfect_match() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("file.txt"), "content").unwrap();
    std::fs::write(b.join("file.txt"), "content").unwrap();

    cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .success()
        .code(0);
}

#[test]
fn exit_1_when_only_errors_no_differences() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("file.txt"), "content").unwrap();
    std::fs::write(b.join("file.txt"), "content").unwrap();

    // Make original unreadable - causes error but no missing/different/extras
    // Note: --all is required to trigger file reading (default only checks size)
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(a.join("file.txt"), std::fs::Permissions::from_mode(0o000)).unwrap();

    let result = cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap(), "--all"])
        .assert();

    // Restore permissions before asserting (so cleanup works even if test fails)
    std::fs::set_permissions(a.join("file.txt"), std::fs::Permissions::from_mode(0o644)).unwrap();

    result.code(1); // Exit 1 due to error
}

#[test]
fn exit_1_when_errors_and_differences() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // File that will error (requires --all to trigger file reading)
    std::fs::write(a.join("unreadable.txt"), "content").unwrap();
    std::fs::write(b.join("unreadable.txt"), "content").unwrap();

    // File that is missing in backup (causes "missing" difference)
    std::fs::write(a.join("missing.txt"), "only in original").unwrap();

    // Make one file unreadable
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        a.join("unreadable.txt"),
        std::fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    let result = cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap(), "--all"])
        .assert();

    // Restore permissions before asserting
    std::fs::set_permissions(
        a.join("unreadable.txt"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    // Should still be exit 1 (errors + differences both count)
    result.code(1);
}

#[test]
fn exit_1_when_only_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("missing.txt"), "only in original").unwrap();
    // b is empty (except root dir)

    cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .code(1);
}

#[test]
fn exit_1_when_only_extras() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a is empty (except root dir)
    std::fs::write(b.join("extra.txt"), "only in backup").unwrap();

    cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .code(1);
}

#[test]
fn exit_1_when_only_different() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Use different sizes so the difference is detected (size check is default)
    std::fs::write(a.join("file.txt"), "short").unwrap();
    std::fs::write(b.join("file.txt"), "this is longer content").unwrap();

    cmd()
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .code(1);
}
