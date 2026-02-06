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
#[cfg(unix)]
use serial_test::serial;

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

/// Remove a test directory in /dev/shm left over from a previous failed run.
/// Uses symlink_metadata to verify the path is an actual directory (not a symlink
/// to one) before calling remove_dir_all, so a stale symlink can never cause us
/// to delete the target tree.
#[cfg(unix)]
fn clean_shm_test_dir(path: &std::path::Path) {
    if std::fs::symlink_metadata(path).map_or(false, |m| m.is_dir()) {
        let _ = std::fs::remove_dir_all(path);
    }
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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

/// Case 6: Actual mount point (not symlink) tests the !follow branch.
///
/// Uses / as original with everything ignored except /dev.
/// /dev is an actual mount point (devtmpfs), not a symlink.
/// This tests the !follow branch in report()'s one_filesystem check
/// (lines 557-562) which prints MISSING-DIR before DIFFERENT-FS.
///
/// BUG #1: Panic when / is original - parent() returns Some("") for /.
/// BUG #2: !follow branch in report() not covered by existing tests.
///
/// NOTE: Must ignore everything in / except /dev. No need to ignore
/// anything inside /dev — report() returns early at DIFFERENT-FS.
/// /dev is more stable than /proc (no ephemeral PIDs).
#[test]
#[cfg(unix)]
#[serial]
fn case6_actual_mount_point_tests_no_follow_branch() {
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;

    let root_dev = std::fs::metadata("/").expect("/ must exist").dev();
    let dev_dev = std::fs::metadata("/dev").expect("/dev must exist").dev();
    assert_ne!(root_dev, dev_dev, "/dev must be on different FS than /");

    let tmp = tempfile::tempdir().unwrap();
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&b).unwrap();

    // Ignore everything in / except /dev
    let root_ignores: Vec<PathBuf> = std::fs::read_dir("/")
        .expect("must be able to read /")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name() != Some(std::ffi::OsStr::new("dev")))
        .collect();

    // No need to ignore anything inside /dev - the test verifies that
    // report() returns early at DIFFERENT-FS without enumerating children.

    let mut cmd = cmd();
    cmd.args(["/", b.to_str().unwrap(), "--one-filesystem", "-vv"]);
    for path in &root_ignores {
        cmd.args(["-i", path.to_str().unwrap()]);
    }

    let assert = cmd.assert().failure();
    let output = stdout_of(&assert);

    // The !follow branch should print MISSING-DIR then DIFFERENT-FS
    assert!(
        output.contains("MISSING-DIR: [/dev]"),
        "Expected 'MISSING-DIR: [/dev]'\nOutput:\n{}",
        output
    );
    assert!(
        output.contains("DIFFERENT-FS: [/dev]"),
        "Expected 'DIFFERENT-FS: [/dev]'\nOutput:\n{}",
        output
    );

    // With -vv: should NOT see /dev/urandom - report() should return early
    // at the DIFFERENT-FS check without enumerating /dev's children.
    assert!(
        !output.contains("urandom"),
        "Should not enumerate /dev contents (found 'urandom')\nOutput:\n{}",
        output
    );

    // Missing: 1 (just /dev itself)
    let missing_count = parse_summary_count(&output, "Missing:");
    assert_eq!(
        missing_count, 1,
        "Expected Missing: 1 (just /dev), got {}\nOutput:\n{}",
        missing_count, output
    );

    // Similarities: 1 (just root dirs)
    let sim_count = parse_summary_count(&output, "Similarities:");
    assert_eq!(
        sim_count, 1,
        "Expected Similarities: 1 (root), got {}\nOutput:\n{}",
        sim_count, output
    );

    // Errors: 0
    let errors_count = parse_summary_count(&output, "Errors:");
    assert_eq!(
        errors_count, 0,
        "Expected Errors: 0, got {}\nOutput:\n{}",
        errors_count, output
    );
}

/// Case 11: Original on different FS than its parent is incorrectly flagged.
///
/// BUG: compare() always checks orig.dev() against parent.dev(), even for
/// the root directory of our comparison. When the original is a mount point
/// (or otherwise on a different FS than its parent), it gets flagged as
/// DIFFERENT-FS and skipped entirely.
///
/// The root of our comparison should never be checked against its parent -
/// only entries INSIDE it should be checked. This bug affects any mount
/// point used as the original: /dev, /mnt/usb, /home/user/nfs_mount, etc.
#[test]
#[cfg(unix)]
#[serial]
fn case11_mount_point_as_root_incorrectly_flagged() {
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;

    // Verify /dev is on a different FS than / (precondition for test)
    let root_dev = std::fs::metadata("/").expect("/ must exist").dev();
    let dev_dev = std::fs::metadata("/dev").expect("/dev must exist").dev();
    assert_ne!(root_dev, dev_dev, "/dev must be on different FS than /");

    let tmp = tempfile::tempdir().unwrap();
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&b).unwrap();

    // Ignore everything in /dev except urandom
    let dev_ignores: Vec<PathBuf> = std::fs::read_dir("/dev")
        .expect("must be able to read /dev")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name() != Some(std::ffi::OsStr::new("urandom")))
        .collect();

    let mut cmd = cmd();
    cmd.args(["/dev", b.to_str().unwrap(), "--one-filesystem"]);
    for path in &dev_ignores {
        cmd.args(["-i", path.to_str().unwrap()]);
    }

    let assert = cmd.assert().failure();
    let output = stdout_of(&assert);

    // BUG: Currently shows "DIFFERENT-FS: [/dev]" and stops
    // Correct: Should process /dev's contents and show "MISSING-SPECIAL: [/dev/urandom]"
    assert!(
        !output.contains("DIFFERENT-FS: [/dev]"),
        "BUG: Original directory should not be checked against its parent\nOutput:\n{}",
        output
    );

    // /dev/urandom is a character device, should be reported as MISSING-SPECIAL
    assert!(
        output.contains("MISSING-SPECIAL:") && output.contains("urandom"),
        "Expected MISSING-SPECIAL for /dev/urandom\nOutput:\n{}",
        output
    );
}

/// Case 12: Backup on different FS than its parent is incorrectly flagged.
///
/// Same bug as case11 but for the backup directory. When the backup is a
/// mount point (or otherwise on a different FS than its parent), it gets
/// flagged as DIFFERENT-FS and skipped entirely.
///
/// The root of our comparison should never be checked against its parent -
/// only entries INSIDE it should be checked.
#[test]
#[cfg(unix)]
#[serial]
fn case12_backup_on_different_fs_than_parent_incorrectly_flagged() {
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;

    // Verify /dev is on a different FS than / (precondition for test)
    let root_dev = std::fs::metadata("/").expect("/ must exist").dev();
    let dev_dev = std::fs::metadata("/dev").expect("/dev must exist").dev();
    assert_ne!(root_dev, dev_dev, "/dev must be on different FS than /");

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    std::fs::create_dir_all(&a).unwrap();

    // Ignore everything in /dev except urandom
    let dev_ignores: Vec<PathBuf> = std::fs::read_dir("/dev")
        .expect("must be able to read /dev")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name() != Some(std::ffi::OsStr::new("urandom")))
        .collect();

    let mut cmd = cmd();
    // Note: original is empty temp dir, backup is /dev
    cmd.args([a.to_str().unwrap(), "/dev", "--one-filesystem"]);
    for path in &dev_ignores {
        cmd.args(["-i", path.to_str().unwrap()]);
    }

    let assert = cmd.assert().failure();
    let output = stdout_of(&assert);

    // BUG: Currently shows "DIFFERENT-FS: [/dev]" and stops
    // Correct: Should process /dev's contents and show "EXTRA-SPECIAL: [/dev/urandom]"
    assert!(
        !output.contains("DIFFERENT-FS: [/dev]"),
        "BUG: Backup directory should not be checked against its parent\nOutput:\n{}",
        output
    );

    // /dev/urandom is a character device, should be reported as EXTRA-SPECIAL
    assert!(
        output.contains("EXTRA-SPECIAL:") && output.contains("urandom"),
        "Expected EXTRA-SPECIAL for /dev/urandom\nOutput:\n{}",
        output
    );
}

/// Case 13: compare() only checks original side, not backup side.
///
/// BUG: compare_directories() checks if orig.dev() differs from orig.parent().dev(),
/// but never checks if backup.dev() differs from backup.parent().dev().
///
/// When both sides have symlinks to directories but only the backup crosses
/// filesystem boundaries, compare() proceeds to compare contents because it
/// only checks the original side.
///
/// Setup:
/// - a/link -> local_dir/ (symlink to directory on SAME FS)
/// - b/link -> /dev/shm/vfy_test/ (symlink to directory on DIFFERENT FS)
///
/// Both are symlinks to directories, so compare_directories() is called.
/// Original's symlink stays on same FS (passes check), but backup's symlink
/// crosses to different FS (not checked).
///
/// Expected: DIFFERENT-FS for backup side, skip comparison
/// Actual (buggy): Compares contents, crossing FS boundary on backup side
#[test]
#[cfg(unix)]
#[serial]
fn case13_compare_only_checks_original_not_backup() {
    require_dev_shm_different_fs();
    clean_shm_test_dir(std::path::Path::new("/dev/shm/vfy_test_case13"));

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create a local directory (same filesystem as temp)
    let local_dir = tmp.path().join("local_target");
    std::fs::create_dir_all(&local_dir).unwrap();
    std::fs::write(local_dir.join("file.txt"), "content on same fs").unwrap();

    // Create structure in /dev/shm (different filesystem)
    let shm_dir = std::path::Path::new("/dev/shm/vfy_test_case13");
    std::fs::create_dir_all(shm_dir).expect("must be able to create dirs in /dev/shm");
    std::fs::write(shm_dir.join("file.txt"), "content on different fs").unwrap();

    // Original: symlink to local directory (SAME FS)
    std::os::unix::fs::symlink(&local_dir, a.join("link")).unwrap();

    // Backup: symlink to /dev/shm (DIFFERENT FS)
    std::os::unix::fs::symlink(shm_dir, b.join("link")).unwrap();

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
        ])
        .assert();

    let output = stdout_of(&assert);

    // Cleanup
    let _ = std::fs::remove_dir_all(shm_dir);

    // BUG: compare() only checks original side, so it doesn't detect that
    // backup's link target is on a different filesystem
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link"),
        "BUG: Expected DIFFERENT-FS for backup symlink to different FS\nOutput:\n{}",
        output
    );

    // Should have Skipped: 1 (the different-fs directory)
    let skipped = parse_summary_count(&output, "Skipped:");
    assert_eq!(
        skipped, 1,
        "BUG: Expected Skipped: 1 for different-fs backup dir\nOutput:\n{}",
        output
    );

    // Should NOT compare file.txt - it's inside the different FS on backup side
    assert!(
        !output.contains("file.txt"),
        "BUG: Should not compare files inside different-fs backup dir\nOutput:\n{}",
        output
    );
}

/// Case 7: Symlink to non-dir on different FS - report() checks all types.
///
/// Tests that report() checks one_filesystem for non-directory targets.
/// This test covers three target types:
///
/// 1. link_to_file → regular file on /dev/shm → DIFFERENT-FS, skipped
/// 2. link_to_symlink → symlink on /dev/shm → chain resolves, DIFFERENT-FS, skipped
/// 3. link_to_dangling → nonexistent on /dev/shm → DANGLING-SYMLINK (see note)
///
/// NOTE on dangling symlinks (issue #33): The dangling case shows DANGLING-SYMLINK
/// instead of DIFFERENT-FS because we fully resolve symlinks before checking.
/// To properly detect this, we'd need to resolve symlinks one level at a time
/// and check each intermediate path's filesystem. This is tracked in issue #33.
///
/// IMPORTANT: Targets are nested DEEP inside /dev/shm (not directly in it).
/// This ensures that a naive fix using parent comparison would still fail:
///   - file.dev() = devshm, parent.dev() = devshm → same! Not caught.
/// The correct fix must compare against the ROOT device, not the parent.
#[test]
#[cfg(unix)]
#[serial]
fn case7_symlink_to_non_dir_on_different_fs_in_report() {
    require_dev_shm_different_fs();
    clean_shm_test_dir(std::path::Path::new("/dev/shm/vfy_test_case7"));

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create targets DEEP in /dev/shm (different filesystem).
    // Must be nested so parent comparison wouldn't detect it.
    let shm_dir = std::path::Path::new("/dev/shm/vfy_test_case7/nested");
    std::fs::create_dir_all(shm_dir).expect("must be able to create dirs in /dev/shm");

    // Target 1: regular file
    let shm_file = shm_dir.join("file.txt");
    std::fs::write(&shm_file, "test content").unwrap();

    // Target 2: symlink → another file (tests symlink chain resolution)
    let shm_target = shm_dir.join("target.txt");
    std::fs::write(&shm_target, "target content").unwrap();
    std::os::unix::fs::symlink(&shm_target, shm_dir.join("inner_link")).unwrap();

    // Target 3: nonexistent path (tests Meta::Dangling)
    // No file created - shm_dir.join("nonexistent") doesn't exist

    // Original has three symlinks to different types on /dev/shm
    std::os::unix::fs::symlink(&shm_file, a.join("link_to_file")).unwrap();
    std::os::unix::fs::symlink(shm_dir.join("inner_link"), a.join("link_to_symlink")).unwrap();
    std::os::unix::fs::symlink(shm_dir.join("nonexistent"), a.join("link_to_dangling")).unwrap();
    // b is empty

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
            "-vv", // Verbose to see wrongly-printed MISSING-FILE and DANGLING-SYMLINK
        ])
        .assert()
        .failure();

    let output = stdout_of(&assert);

    // Cleanup
    let _ = std::fs::remove_dir_all("/dev/shm/vfy_test_case7");

    // All three symlinks should be reported as missing
    assert!(
        output.contains("MISSING-SYMLINK:") && output.contains("link_to_file"),
        "Expected MISSING-SYMLINK for link_to_file, got:\n{}",
        output
    );
    assert!(
        output.contains("MISSING-SYMLINK:") && output.contains("link_to_symlink"),
        "Expected MISSING-SYMLINK for link_to_symlink, got:\n{}",
        output
    );
    assert!(
        output.contains("MISSING-SYMLINK:") && output.contains("link_to_dangling"),
        "Expected MISSING-SYMLINK for link_to_dangling, got:\n{}",
        output
    );

    // File and symlink-chain targets should show DIFFERENT-FS
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link_to_file"),
        "Expected DIFFERENT-FS for link_to_file, got:\n{}",
        output
    );
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link_to_symlink"),
        "Expected DIFFERENT-FS for link_to_symlink, got:\n{}",
        output
    );

    // Should NOT see MISSING-FILE - file targets are on different FS and skipped
    assert!(
        !output.contains("MISSING-FILE:"),
        "Should NOT see MISSING-FILE - targets are on different FS, got:\n{}",
        output
    );

    // Dangling symlinks show DANGLING-SYMLINK (see issue #33 - we'd need to
    // resolve symlinks one level at a time to detect this properly)
    assert!(
        output.contains("DANGLING-SYMLINK:"),
        "Expected DANGLING-SYMLINK for dangling target (issue #33), got:\n{}",
        output
    );

    // File and symlink-chain should be skipped (2), dangling is an error
    let skipped_count = parse_summary_count(&output, "Skipped:");
    assert_eq!(
        skipped_count, 2,
        "Expected Skipped: 2 (file + symlink-chain), got Skipped: {}\nOutput:\n{}",
        skipped_count, output
    );

    // Missing should be 3 (the three symlinks)
    let missing_count = parse_summary_count(&output, "Missing:");
    assert_eq!(
        missing_count, 3,
        "Expected Missing: 3 (symlinks), got Missing: {}\nOutput:\n{}",
        missing_count, output
    );

    // Original items: 5 (root + 3 symlinks + 1 dangling re-counted, see issue #33)
    // The dangling symlink counts twice because we can't detect it's on a different FS
    let items_count = parse_summary_count(&output, "Original items processed:");
    assert_eq!(
        items_count, 5,
        "Expected Original items: 5, got {}\nOutput:\n{}",
        items_count, output
    );

    // Errors: 1 (dangling symlink on /dev/shm - see issue #33)
    let errors_count = parse_summary_count(&output, "Errors:");
    assert_eq!(
        errors_count, 1,
        "Expected Errors: 1 (dangling - issue #33), got {}\nOutput:\n{}",
        errors_count, output
    );
}

/// Case 8: Symlink to non-dir on different FS - compare() checks all types.
///
/// Tests that compare() checks one_filesystem for non-directory targets.
/// This test covers three target types:
///
/// 1. link_to_file → regular file on /dev/shm → DIFFERENT-FS, skipped
/// 2. link_to_symlink → symlink on /dev/shm → chain resolves, DIFFERENT-FS, skipped
/// 3. link_to_dangling → nonexistent on /dev/shm → DANGLING-SYMLINK (see note)
///
/// NOTE on dangling symlinks (issue #33): The dangling case shows DANGLING-SYMLINK
/// instead of DIFFERENT-FS because we fully resolve symlinks before checking.
/// To properly detect this, we'd need to resolve symlinks one level at a time
/// and check each intermediate path's filesystem. This is tracked in issue #33.
///
/// IMPORTANT: Targets are nested DEEP inside /dev/shm (not directly in it).
/// This ensures that a naive fix using parent comparison would still fail:
///   - file.dev() = devshm, parent.dev() = devshm → same! Not caught.
/// The correct fix must compare against the ROOT device, not the parent.
#[test]
#[cfg(unix)]
#[serial]
fn case8_symlink_to_non_dir_on_different_fs_in_compare() {
    require_dev_shm_different_fs();
    clean_shm_test_dir(std::path::Path::new("/dev/shm/vfy_test_case8"));

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create targets DEEP in /dev/shm (different filesystem).
    // Must be nested so parent comparison wouldn't detect it.
    let shm_dir = std::path::Path::new("/dev/shm/vfy_test_case8/nested");
    std::fs::create_dir_all(shm_dir).expect("must be able to create dirs in /dev/shm");

    // Target 1: regular file
    let shm_file = shm_dir.join("file.txt");
    std::fs::write(&shm_file, "test content").unwrap();

    // Target 2: symlink → another file (tests symlink chain resolution)
    let shm_target = shm_dir.join("target.txt");
    std::fs::write(&shm_target, "target content").unwrap();
    std::os::unix::fs::symlink(&shm_target, shm_dir.join("inner_link")).unwrap();

    // Target 3: nonexistent path (tests Meta::Dangling)
    // No file created - shm_dir.join("nonexistent") doesn't exist

    // Both sides have the same three symlinks
    for dir in [&a, &b] {
        std::os::unix::fs::symlink(&shm_file, dir.join("link_to_file")).unwrap();
        std::os::unix::fs::symlink(shm_dir.join("inner_link"), dir.join("link_to_symlink"))
            .unwrap();
        std::os::unix::fs::symlink(shm_dir.join("nonexistent"), dir.join("link_to_dangling"))
            .unwrap();
    }

    // Don't assert exit code: buggy behavior produces errors (DANGLING-SYMLINK)
    // which cause non-zero exit
    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
            "--all", // Required to make it actually read file contents
        ])
        .assert();

    let output = stdout_of(&assert);

    // Cleanup
    let _ = std::fs::remove_dir_all("/dev/shm/vfy_test_case8");

    // File and symlink-chain targets should show DIFFERENT-FS
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link_to_file"),
        "Expected DIFFERENT-FS for link_to_file, got:\n{}",
        output
    );
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link_to_symlink"),
        "Expected DIFFERENT-FS for link_to_symlink, got:\n{}",
        output
    );

    // Dangling symlinks show DANGLING-SYMLINK (see issue #33 - we'd need to
    // resolve symlinks one level at a time to detect this properly)
    assert!(
        output.contains("DANGLING-SYMLINK:"),
        "Expected DANGLING-SYMLINK for dangling target (issue #33), got:\n{}",
        output
    );

    // File and symlink-chain should be skipped (2), dangling is an error
    let skipped_count = parse_summary_count(&output, "Skipped:");
    assert_eq!(
        skipped_count, 2,
        "Expected Skipped: 2 (file + symlink-chain), got Skipped: {}\nOutput:\n{}",
        skipped_count, output
    );

    // Similarities: 4 (roots + symlinks themselves, not their targets)
    let sim_count = parse_summary_count(&output, "Similarities:");
    assert_eq!(
        sim_count, 4,
        "Expected Similarities: 4 (roots + symlinks), got {}\nOutput:\n{}",
        sim_count, output
    );

    // Errors: 2 (both dangling symlinks - see issue #33)
    let errors_count = parse_summary_count(&output, "Errors:");
    assert_eq!(
        errors_count, 2,
        "Expected Errors: 2 (dangling symlinks, issue #33), got {}\nOutput:\n{}",
        errors_count, output
    );
}

/// Case 9: report() parent check fails inside followed symlink.
///
/// BUG: one_filesystem check compares parent.dev vs entry.dev.
/// Inside a followed symlink, both are on the same (different) FS:
///   - path = a/link/subdir/file
///   - parent = a/link/subdir → fs::metadata follows link → /dev/shm/subdir
///   - parent.dev = devshm's device
///   - entry.dev = devshm's device
///   - Same! Not detected.
/// Should compare to ROOT device, not parent device.
///
/// Expected: MISSING-SYMLINK + DIFFERENT-FS for link, NO entries from inside
/// Actual (buggy): Also reports MISSING-DIR/FILE for contents of /dev/shm
#[test]
#[cfg(unix)]
#[serial]
fn case9_report_parent_check_inside_symlink() {
    require_dev_shm_different_fs();
    clean_shm_test_dir(std::path::Path::new("/dev/shm/vfy_test_case9"));

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create structure in /dev/shm
    let shm_base = std::path::Path::new("/dev/shm/vfy_test_case9");
    let shm_nested = shm_base.join("nested");
    std::fs::create_dir_all(&shm_nested).expect("must be able to create dirs in /dev/shm");
    std::fs::write(shm_nested.join("file.txt"), "nested file").unwrap();

    // Original has: missing_dir/link -> /dev/shm/vfy_test_case9
    // Backup is empty (missing_dir doesn't exist)
    std::fs::create_dir_all(a.join("missing_dir")).unwrap();
    std::os::unix::fs::symlink(shm_base, a.join("missing_dir/link")).unwrap();
    // b is empty

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
            "-vv", // verbose to see all children
        ])
        .assert()
        .failure();

    let output = stdout_of(&assert);

    // Cleanup
    let _ = std::fs::remove_dir_all(shm_base);

    // Should see MISSING-DIR for missing_dir
    assert!(
        output.contains("MISSING-DIR:") && output.contains("missing_dir"),
        "Expected MISSING-DIR for missing_dir, got:\n{}",
        output
    );

    // Should see DIFFERENT-FS for the link
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link"),
        "Expected DIFFERENT-FS for link, got:\n{}",
        output
    );

    // BUG: Should NOT see "nested" or "file.txt" - these are inside /dev/shm
    assert!(
        !output.contains("nested"),
        "BUG: Should not report entries from inside /dev/shm (nested)\nOutput:\n{}",
        output
    );
    assert!(
        !output.contains("file.txt"),
        "BUG: Should not report entries from inside /dev/shm (file.txt)\nOutput:\n{}",
        output
    );

    // Missing count should be 2 (missing_dir + link), NOT more from /dev/shm
    let missing_count = parse_summary_count(&output, "Missing:");
    assert_eq!(
        missing_count, 2,
        "BUG: Expected Missing: 2 (dir + link), got {} - reported /dev/shm contents!\nOutput:\n{}",
        missing_count, output
    );
}

/// Case 10: compare() parent check - symlink deep into different FS.
///
/// BUG: one_filesystem check compares parent.dev vs entry.dev.
/// For a symlink to /dev/shm, compare() should detect DIFFERENT-FS at
/// the symlink level and not recurse.
///
/// This test verifies:
/// 1. DIFFERENT-FS is detected for the symlink
/// 2. Contents inside /dev/shm are NOT compared
///
/// If the parent check bug manifested (e.g., after refactoring), nested
/// entries would be compared because parent and entry would both be on
/// the different filesystem.
#[test]
#[cfg(unix)]
#[serial]
fn case10_compare_parent_check_inside_symlink() {
    require_dev_shm_different_fs();
    clean_shm_test_dir(std::path::Path::new("/dev/shm/vfy_test_case10"));

    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create structure in /dev/shm (different filesystem)
    let shm_base = std::path::Path::new("/dev/shm/vfy_test_case10");
    let shm_nested = shm_base.join("nested");
    std::fs::create_dir_all(&shm_nested).expect("must be able to create dirs in /dev/shm");
    std::fs::write(shm_nested.join("file.txt"), "nested file").unwrap();

    // Both sides have: link -> /dev/shm/vfy_test_case10
    // This goes through compare_directories(), not report()
    std::os::unix::fs::symlink(shm_base, a.join("link")).unwrap();
    std::os::unix::fs::symlink(shm_base, b.join("link")).unwrap();

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--follow",
            "--one-filesystem",
        ])
        .assert()
        .success(); // Should succeed - only root dirs match, link is skipped

    let output = stdout_of(&assert);

    // Cleanup
    let _ = std::fs::remove_dir_all(shm_base);

    // Should see DIFFERENT-FS for the link
    assert!(
        output.contains("DIFFERENT-FS:") && output.contains("link"),
        "Expected DIFFERENT-FS for link, got:\n{}",
        output
    );

    // BUG TEST: Should NOT see "nested" or "file.txt" - these are inside /dev/shm
    // If the parent check bug manifested, compare() would recurse into the
    // symlink and compare these entries (since parent and entry would have
    // the same device).
    assert!(
        !output.contains("nested"),
        "BUG: Should not compare entries inside /dev/shm (nested)\nOutput:\n{}",
        output
    );
    assert!(
        !output.contains("file.txt"),
        "BUG: Should not compare entries inside /dev/shm (file.txt)\nOutput:\n{}",
        output
    );

    // Should have 1 skip (the different-fs link)
    let skipped = parse_summary_count(&output, "Skipped:");
    assert_eq!(
        skipped, 1,
        "Expected Skipped: 1 for the different-fs link, got {}.\nOutput:\n{}",
        skipped, output
    );

    // Similarities should only be roots (2), not nested entries
    let sim_count = parse_summary_count(&output, "Similarities:");
    assert_eq!(
        sim_count, 2,
        "BUG: Expected Similarities: 2 (just roots), got {} - nested entries compared!\nOutput:\n{}",
        sim_count, output
    );
}
