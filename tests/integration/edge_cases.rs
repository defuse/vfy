use super::{cmd, no_line_has, some_line_has, stdout_of, testdata, testdata_base};
use std::process::Command as StdCommand;
use predicates::prelude::*;

#[test]
fn empty_directories() {
    // Git can't track empty directories, so create them at runtime
    let base = testdata_base("empty");
    let a = base.join("a");
    let b = base.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    // Remove any .gitkeep files that might exist
    let _ = std::fs::remove_file(a.join(".gitkeep"));
    let _ = std::fs::remove_file(b.join(".gitkeep"));

    let a = a.to_str().unwrap().to_string();
    let b = b.to_str().unwrap().to_string();
    cmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Original items processed: 1")
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing: 0 (0.00%)"))
                .and(predicate::str::contains("Different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 1"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

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

// ── Zero-byte files ─────────────────────────────────────────

#[test]
fn zero_byte_files_with_all() {
    let (a, b) = testdata("empty_files");
    let assert = cmd().args([&a, &b, "--all", "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);

    // Two identical empty files should match
    assert!(
        !output.contains("DIFFERENT-FILE"),
        "Empty files should match, got:\n{}",
        output
    );
    assert!(output.contains("Similarities: 2"), "got:\n{}", output);
    assert!(output.contains("Errors: 0"), "got:\n{}", output);

    // Verify the known BLAKE3 hash of empty input
    // BLAKE3("") = af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
    let empty_hash = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
    assert!(
        some_line_has(&output, "BLAKE3", empty_hash),
        "Expected known BLAKE3 hash for empty file, got:\n{}",
        output
    );
}

#[test]
fn zero_byte_files_with_sampling() {
    let (a, b) = testdata("empty_files");
    cmd()
        .args([&a, &b, "-s", "10"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE")
                .not()
                .and(predicate::str::contains("Similarities: 2"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

// ── Extras with zero original items ─────────────────────────

#[test]
fn extras_with_zero_originals() {
    // a/ is empty, b/ has files → percentage 0.00% but exit 1 due to extras
    let base = testdata_base("extras_only");
    let a = base.join("a");
    let b = base.join("b");
    // Ensure empty dir exists (git can't track empty dirs)
    std::fs::create_dir_all(&a).unwrap();
    let _ = std::fs::remove_file(a.join(".gitkeep"));

    let a = a.to_str().unwrap().to_string();
    let b = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    assert!(
        output.contains("Original items processed: 1"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Missing: 0 (0.00%)"),
        "got:\n{}",
        output
    );
    assert!(output.contains("Extras: 3"), "got:\n{}", output);
}

// ── Mixed results in one directory ──────────────────────────

#[test]
fn mixed_results_per_file() {
    let (a, b) = testdata("mixed_results");
    let assert = cmd().args([&a, &b, "-s", "10", "--all"]).assert().code(1);
    let output = stdout_of(&assert);

    // match.txt should not appear as DIFFERENT-FILE
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "match.txt"),
        "match.txt should not differ, got:\n{}",
        output
    );
    // size_diff.txt should differ by SIZE
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "size_diff.txt"),
        "Expected DIFFERENT-FILE for size_diff.txt, got:\n{}",
        output
    );
    assert!(
        some_line_has(&output, "SIZE", "size_diff.txt"),
        "Expected SIZE reason for size_diff.txt, got:\n{}",
        output
    );
    // content_diff.txt should differ by SAMPLE and/or HASH
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "content_diff.txt"),
        "Expected DIFFERENT-FILE for content_diff.txt, got:\n{}",
        output
    );

    // Summary: 4 items (root + 3), 2 different, 2 similarities (root + match.txt)
    assert!(
        output.contains("Original items processed: 4"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Backup items processed: 4"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Different: 2"),
        "got:\n{}",
        output
    );
    assert!(output.contains("Similarities: 2"), "got:\n{}", output);
}

// ── Deeply nested identical trees ───────────────────────────

#[test]
fn deep_identical_tree() {
    let (a, b) = testdata("deep_identical");
    cmd()
        .args([&a, &b, "--all"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE")
                .not()
                .and(predicate::str::contains("MISSING").not())
                .and(predicate::str::contains("EXTRA").not())
                .and(predicate::str::contains("ERROR").not())
                // root + l1/ + l2/ + l3/ + deep.txt + mid.txt + top.txt = 7
                .and(predicate::str::contains("Original items processed: 7"))
                .and(predicate::str::contains("Backup items processed: 7"))
                .and(predicate::str::contains("Similarities: 7"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

// ── Special file types ──────────────────────────────────────

#[test]
fn symlink_to_dev_dir_with_follow() {
    // Symlink to /dev/ — contains character devices (null, zero, etc.)
    // With --follow, the tool traverses into /dev/ and should report
    // NOT_A_FILE_OR_DIR for device files, not ERROR or silent pass.
    let tmp = std::env::temp_dir().join("bv_test_dev_dir");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Both sides symlink to /dev/
    std::os::unix::fs::symlink("/dev", a.join("dev")).unwrap();
    std::os::unix::fs::symlink("/dev", b.join("dev")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // /dev/ has character devices like null, zero — should trigger NOT_A_FILE_OR_DIR
    let not_a_file_lines: Vec<&str> = output
        .lines()
        .filter(|l| l.contains("NOT_A_FILE_OR_DIR:"))
        .collect();
    assert!(
        !not_a_file_lines.is_empty(),
        "Expected NOT_A_FILE_OR_DIR for device files in /dev/, got:\n{}",
        output
    );

    // /dev/null specifically should be NOT_A_FILE_OR_DIR, not ERROR
    assert!(
        some_line_has(&output, "NOT_A_FILE_OR_DIR:", "null"),
        "Expected NOT_A_FILE_OR_DIR for /dev/null, got:\n{}",
        output
    );
    assert!(
        no_line_has(&output, "ERROR:", "null"),
        "/dev/null should be NOT_A_FILE_OR_DIR, not ERROR, got:\n{}",
        output
    );

    // Summary should show at least 2 not-a-file-or-dir (/dev/null and /dev/urandom at minimum)
    let naf_line = output
        .lines()
        .find(|l| l.contains("Not a file or dir:"))
        .expect("Expected 'Not a file or dir' in summary");
    let naf_count: u64 = naf_line
        .trim()
        .rsplit(' ')
        .next()
        .unwrap()
        .parse()
        .expect("Failed to parse not-a-file-or-dir count");
    assert!(
        naf_count >= 2,
        "Expected at least 2 not-a-file-or-dir entries (null + urandom), got {}",
        naf_count
    );
}

#[test]
fn special_files_missing_from_backup() {
    // Both sides have symlink to /dev/ so --follow traverses it.
    // Remove a device file from b's view by using a real dir with a subset of entries.
    // Simpler approach: a/ has a real dir with a symlink to a device inside,
    // b/ has same real dir but missing the device symlink.
    let tmp = std::env::temp_dir().join("bv_test_dev_missing");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(a.join("sub")).unwrap();
    std::fs::create_dir_all(b.join("sub")).unwrap();

    // a/sub has a symlink to /dev/null (character device) — missing from b/sub
    std::os::unix::fs::symlink("/dev/null", a.join("sub/devnull")).unwrap();
    // Both have a regular file
    std::fs::write(a.join("sub/ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("sub/ok.txt"), "ok\n").unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // The device symlink is missing from backup — reported as MISSING-SYMLINK
    assert!(
        some_line_has(&output, "MISSING-SYMLINK:", "devnull"),
        "Expected MISSING-SYMLINK for devnull symlink, got:\n{}",
        output
    );
    assert!(
        output.contains("Errors: 0"),
        "Missing special file symlinks should not cause errors, got:\n{}",
        output
    );
}

#[test]
fn special_files_extra_in_backup() {
    // b/sub has a symlink to /dev/null that a/sub does not — extra
    let tmp = std::env::temp_dir().join("bv_test_dev_extra");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(a.join("sub")).unwrap();
    std::fs::create_dir_all(b.join("sub")).unwrap();

    // b/sub has a symlink to /dev/null — extra
    std::os::unix::fs::symlink("/dev/null", b.join("sub/devnull")).unwrap();
    // Both have a regular file
    std::fs::write(a.join("sub/ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("sub/ok.txt"), "ok\n").unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // The device symlink is extra in backup — reported as EXTRA-SYMLINK
    assert!(
        some_line_has(&output, "EXTRA-SYMLINK:", "devnull"),
        "Expected EXTRA-SYMLINK for devnull symlink, got:\n{}",
        output
    );
    assert!(
        !output.contains("Extras: 0"),
        "Should have extras, got:\n{}",
        output
    );
    assert!(
        output.contains("Errors: 0"),
        "Extra special file symlinks should not cause errors, got:\n{}",
        output
    );
}

#[test]
fn special_file_via_symlink_follow() {
    // Symlink to /dev/urandom is a character device (not a file, not a dir).
    // With --follow, the tool should detect this and report NOT_A_FILE_OR_DIR
    // instead of trying to compare it as a regular file.
    let tmp = std::env::temp_dir().join("bv_test_special_file");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Both sides have a symlink to /dev/urandom
    std::os::unix::fs::symlink("/dev/urandom", a.join("special")).unwrap();
    std::os::unix::fs::symlink("/dev/urandom", b.join("special")).unwrap();

    // Also add a regular matching file
    std::fs::write(a.join("ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("ok.txt"), "ok\n").unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        some_line_has(&output, "NOT_A_FILE_OR_DIR:", "special"),
        "Expected NOT_A_FILE_OR_DIR for symlink to /dev/urandom, got:\n{}",
        output
    );
    // Cat4: same-target symlink pair counted as similarity (root + symlink pair + ok.txt = 3)
    assert!(
        output.contains("Similarities: 3"),
        "Expected similarities: root + same-target symlink pair + ok.txt = 3, got:\n{}",
        output
    );
    // Should be counted in the not-a-file-or-dir summary (one per side)
    assert!(
        output.contains("Not a file or dir: 2"),
        "Expected 'Not a file or dir: 2' in summary, got:\n{}",
        output
    );
    // Should not produce ERROR — this is an expected condition, not an I/O failure
    assert!(
        no_line_has(&output, "ERROR:", "special"),
        "Special file should be NOT_A_FILE_OR_DIR, not ERROR, got:\n{}",
        output
    );
}

// ── Special file (FIFO) missing/extra ───────────────────────

#[test]
fn special_file_missing_from_backup() {
    // A FIFO in original but absent from backup should be NOT_A_FILE_OR_DIR,
    // not MISSING-FILE.
    let tmp = std::env::temp_dir().join("bv_test_fifo_missing");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create a FIFO in a/ only
    let fifo_path = a.join("my_fifo");
    StdCommand::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("mkfifo failed");

    // Both sides have a regular file so there's something to compare
    std::fs::write(a.join("ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("ok.txt"), "ok\n").unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // A special file missing from backup should be NOT_A_FILE_OR_DIR, not MISSING-FILE
    assert!(
        some_line_has(&output, "NOT_A_FILE_OR_DIR:", "my_fifo"),
        "Expected NOT_A_FILE_OR_DIR for missing FIFO, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "MISSING-FILE:", "my_fifo"),
        "FIFO should not be reported as MISSING-FILE, got:\n{}",
        output
    );
}

#[test]
fn special_file_extra_in_backup() {
    // A FIFO in backup but absent from original should be NOT_A_FILE_OR_DIR,
    // not EXTRA-FILE.
    let tmp = std::env::temp_dir().join("bv_test_fifo_extra");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create a FIFO in b/ only
    let fifo_path = b.join("my_fifo");
    StdCommand::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("mkfifo failed");

    // Both sides have a regular file
    std::fs::write(a.join("ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("ok.txt"), "ok\n").unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // An extra special file in backup should be NOT_A_FILE_OR_DIR, not EXTRA-FILE
    assert!(
        some_line_has(&output, "NOT_A_FILE_OR_DIR:", "my_fifo"),
        "Expected NOT_A_FILE_OR_DIR for extra FIFO, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "EXTRA-FILE:", "my_fifo"),
        "FIFO should not be reported as EXTRA-FILE, got:\n{}",
        output
    );
}

// ── Special file vs symlink ─────────────────────────────────

#[test]
fn special_file_vs_symlink() {
    // One side has a FIFO, the other has a symlink with the same name.
    // Should be NOT_A_FILE_OR_DIR, not DIFFERENT-SYMLINK-STATUS.
    let tmp = std::env::temp_dir().join("bv_test_fifo_vs_symlink");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/entry is a FIFO
    let fifo_path = a.join("entry");
    StdCommand::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("mkfifo failed");

    // b/entry is a symlink
    std::fs::write(b.join("target.txt"), "content\n").unwrap();
    std::os::unix::fs::symlink("target.txt", b.join("entry")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Special file should always be NOT_A_FILE_OR_DIR regardless of what the other side is
    assert!(
        some_line_has(&output, "NOT_A_FILE_OR_DIR:", "entry"),
        "Expected NOT_A_FILE_OR_DIR when one side is a FIFO, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "DIFFERENT-SYMLINK-STATUS:", "entry"),
        "FIFO vs symlink should not be DIFFERENT-SYMLINK-STATUS, got:\n{}",
        output
    );
}

#[test]
fn symlink_vs_special_file() {
    // Reverse: original is a symlink, backup is a FIFO.
    let tmp = std::env::temp_dir().join("bv_test_symlink_vs_fifo");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/entry is a symlink
    std::fs::write(a.join("target.txt"), "content\n").unwrap();
    std::os::unix::fs::symlink("target.txt", a.join("entry")).unwrap();

    // b/entry is a FIFO
    let fifo_path = b.join("entry");
    StdCommand::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("mkfifo failed");

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        some_line_has(&output, "NOT_A_FILE_OR_DIR:", "entry"),
        "Expected NOT_A_FILE_OR_DIR when one side is a FIFO, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "DIFFERENT-SYMLINK-STATUS:", "entry"),
        "Symlink vs FIFO should not be DIFFERENT-SYMLINK-STATUS, got:\n{}",
        output
    );
}
