//! Tests for --one-filesystem (-o) behavior across filesystem boundaries.
//!
//! These tests use symlinks to /dev/shm (which is devtmpfs, a different filesystem
//! from tmpfs) to test cross-filesystem detection.
//!
//! Four cases are tested:
//! - Case 1: Mount point in compare() path - WORKS
//! - Case 2: Mount point in report() path - needs fix in report()
//! - Case 3: Symlink to different FS in compare() - WORKS
//! - Case 4: Symlink to different FS in report() - needs fix in report()

use super::{cmd, stdout_of};
use crate::harness::{create_entries, Entry::*};

/// Verify /dev/shm is on a different filesystem than temp directories.
/// Panics if /dev/shm doesn't exist or isn't on a different filesystem.
/// We use /dev/shm specifically because it's a small, predictable directory.
#[cfg(unix)]
fn require_dev_shm_different_fs() {
    use std::os::unix::fs::MetadataExt;
    let tmp_dev = std::env::temp_dir()
        .metadata()
        .expect("Cannot stat temp directory")
        .dev();
    let dev_shm_dev = std::fs::metadata("/dev/shm")
        .expect("/dev/shm does not exist - these tests require /dev/shm")
        .dev();
    assert_ne!(
        tmp_dev, dev_shm_dev,
        "/dev/shm must be on a different filesystem than temp dir for these tests"
    );
}

/// Parse a count from a summary line like "    Missing: 2 (66.67%)"
fn parse_summary_count(output: &str, label: &str) -> u64 {
    output
        .lines()
        .find(|l| l.contains(label))
        .and_then(|line| {
            // Line format: "    Missing: 2 (66.67%)" - we want the number after the colon
            line.split(':')
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
        .unwrap_or(0)
}

/// Case 1: Both sides have symlink to /dev/shm, compare() detects different FS.
///
/// This tests the working path: compare_directories() checks device IDs
/// when following symlinks to directories.
#[test]
#[cfg(unix)]
fn case1_different_fs_in_compare_both_sides_symlink() {
    require_dev_shm_different_fs();

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Both sides have symlink to /dev/shm (a directory on devtmpfs)
    create_entries(&a, &[Sym("dev_link", "/dev/shm")]);
    create_entries(&b, &[Sym("dev_link", "/dev/shm")]);

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
        ])
        .assert()
        .success(); // Should succeed - just skipped

    let output = stdout_of(&assert);

    // Should see DIFFERENT-FS for the symlink target
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("dev_link"),
        "Expected DIFFERENT-FS for dev_link, got:\n{}",
        output
    );

    // Should NOT recurse into /dev/shm - verify by checking we don't see MISSING/EXTRA
    assert!(
        !output.contains("MISSING-") && !output.contains("EXTRA-"),
        "Should not report missing/extra from /dev/shm, got:\n{}",
        output
    );

    // Check counts: skipped should be 1 (the different-fs entry)
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

/// Case 2: Directory missing from backup contains symlink to /dev/shm.
///
/// report() should check --one-filesystem and NOT recurse into /dev/shm.
///
/// Expected behavior:
/// - MISSING-DIR for missing_dir
/// - MISSING-SYMLINK for dev_link
/// - DIFFERENT-FS for dev_link (when following)
/// - Missing: 2, Skipped: 1
#[test]
#[cfg(unix)]
fn case2_different_fs_in_report_missing_dir_contains_mount() {
    require_dev_shm_different_fs();

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Original has a directory with a symlink to /dev/shm
    // Backup is missing this directory entirely
    create_entries(
        &a,
        &[
            Dir("missing_dir"),
            Sym("missing_dir/dev_link", "/dev/shm"),
        ],
    );
    // b is empty - missing_dir doesn't exist

    // Use -vv to see all children (including the symlink inside missing_dir)
    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
            "-vv",
        ])
        .assert()
        .failure(); // Will fail due to missing items

    let output = stdout_of(&assert);

    // The directory itself should be reported as missing
    assert!(
        output.contains("MISSING-DIR:") && output.contains("missing_dir"),
        "Expected MISSING-DIR for missing_dir, got:\n{}",
        output
    );

    // With -vv, we should see the symlink inside missing_dir
    assert!(
        output.contains("MISSING-SYMLINK:") && output.contains("dev_link"),
        "Expected MISSING-SYMLINK for dev_link (with -vv), got:\n{}",
        output
    );

    // Should see DIFFERENT-FS when report() tries to follow the symlink
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("dev_link"),
        "Expected DIFFERENT-FS for dev_link, got:\n{}",
        output
    );

    // Should NOT see any entries from inside /dev/shm
    // (no MISSING-FILE or MISSING-DIR for /dev/shm contents)
    let missing_count = parse_summary_count(&output, "Missing:");
    assert_eq!(
        missing_count, 2,
        "Expected Missing: 2 (dir + symlink only), got Missing: {}\n\
         Should not count /dev/shm contents.\nOutput:\n{}",
        missing_count, output
    );

    // Should have skipped the different-fs entry
    let skipped_count = parse_summary_count(&output, "Skipped:");
    assert_eq!(
        skipped_count, 1,
        "Expected Skipped: 1 (the different-fs symlink target), got Skipped: {}\nOutput:\n{}",
        skipped_count, output
    );
}

/// Case 3: Both sides have symlink to /dev/shm, targets match.
///
/// This is similar to Case 1 but explicitly tests the symlink comparison path.
/// compare_symlinks() matches the targets, then compare() follows and hits
/// the device check in compare_directories().
#[test]
#[cfg(unix)]
fn case3_different_fs_symlink_both_sides_match() {
    require_dev_shm_different_fs();

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Both have identical symlinks to /dev/shm
    create_entries(&a, &[Sym("link", "/dev/shm")]);
    create_entries(&b, &[Sym("link", "/dev/shm")]);

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
        ])
        .assert()
        .success();

    let output = stdout_of(&assert);

    // Symlinks match, so no DIFFERENT-SYMLINK-TARGET
    assert!(
        !output.contains("DIFFERENT-SYMLINK-TARGET"),
        "Symlinks have same target, should not differ:\n{}",
        output
    );

    // When following, should hit DIFFERENT-FS
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link"),
        "Expected DIFFERENT-FS when following symlink, got:\n{}",
        output
    );

    // Should not recurse into /dev/shm
    assert!(
        !output.contains("MISSING-") && !output.contains("EXTRA-"),
        "Should not report missing/extra from /dev/shm, got:\n{}",
        output
    );
}

/// Case 4: Symlink to /dev/shm exists only in original, missing from backup.
///
/// report() should check --one-filesystem and NOT recurse into /dev/shm.
///
/// Expected behavior:
/// - MISSING-SYMLINK for dev_link
/// - DIFFERENT-FS for dev_link (when following)
/// - Missing: 1, Skipped: 1
#[test]
#[cfg(unix)]
fn case4_different_fs_symlink_missing_from_backup() {
    require_dev_shm_different_fs();

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Original has symlink to /dev/shm, backup doesn't have it
    create_entries(&a, &[Sym("dev_link", "/dev/shm")]);
    // b is empty

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
        ])
        .assert()
        .failure(); // Will fail due to missing items

    let output = stdout_of(&assert);

    // The symlink itself should be reported as missing
    assert!(
        output.contains("MISSING-SYMLINK:") && output.contains("dev_link"),
        "Expected MISSING-SYMLINK for dev_link, got:\n{}",
        output
    );

    // Should see DIFFERENT-FS when report() tries to follow the symlink
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("dev_link"),
        "Expected DIFFERENT-FS for dev_link, got:\n{}",
        output
    );

    // Should NOT count /dev/shm contents as missing
    let missing_count = parse_summary_count(&output, "Missing:");
    assert_eq!(
        missing_count, 1,
        "Expected Missing: 1 (just the symlink), got Missing: {}\n\
         Should not count /dev/shm contents.\nOutput:\n{}",
        missing_count, output
    );

    // Should have skipped the different-fs entry
    let skipped_count = parse_summary_count(&output, "Skipped:");
    assert_eq!(
        skipped_count, 1,
        "Expected Skipped: 1 (the different-fs symlink target), got Skipped: {}\nOutput:\n{}",
        skipped_count, output
    );
}

/// Verify that without --one-filesystem, we DO recurse into /dev/shm.
/// This confirms that the --one-filesystem flag is what prevents recursion.
#[test]
#[cfg(unix)]
fn baseline_without_one_filesystem_recurses() {
    require_dev_shm_different_fs();

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Both have symlink to /dev/shm
    create_entries(&a, &[Sym("dev_link", "/dev/shm")]);
    create_entries(&b, &[Sym("dev_link", "/dev/shm")]);

    // WITHOUT --one-filesystem
    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            // No --one-filesystem
        ])
        .assert()
        .success();

    let output = stdout_of(&assert);

    // Should NOT see DIFFERENT-FS
    assert!(
        !output.contains("DIFFERENT-FS"),
        "Without -o, should not see DIFFERENT-FS:\n{}",
        output
    );

    // Should see similarities from comparing /dev/shm contents
    let sim_count = parse_summary_count(&output, "Similarities:");

    // Should have more than just the root dirs and symlinks
    // (i.e., we recursed into /dev/shm and found matching entries)
    assert!(
        sim_count > 2,
        "Without -o, should recurse into /dev/shm and find similarities, got: {}\n{}",
        sim_count,
        output
    );
}
