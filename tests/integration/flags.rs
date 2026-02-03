use super::{cmd, some_line_has, stdout_of, testdata, testdata_base};
use predicates::prelude::*;

// ── Verbosity ────────────────────────────────────────────────

#[test]
fn verbose_dirs_only() {
    let (a, b) = testdata("identical");
    let assert = cmd().args([&a, &b, "-v"]).assert().success();
    let output = stdout_of(&assert);

    assert!(output.contains("DEBUG: Comparing"));
    assert!(
        !output.contains("DEBUG: Comparing file"),
        "File-level DEBUG requires -vv"
    );
    assert!(output.contains("Similarities: 4"));
}

#[test]
fn verbose_files() {
    let (a, b) = testdata("identical");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);

    assert!(output.contains("DEBUG: Comparing file"));
    assert!(output.contains("Similarities: 4"));
}

#[test]
fn verbose_blake3_known_hashes() {
    let (a, b) = testdata("identical");
    let assert = cmd()
        .args([&a, &b, "-v", "-v", "--all"])
        .assert()
        .success();
    let output = stdout_of(&assert);

    // BLAKE3("hello world\n")
    let hello_hash = "dc5a4edb8240b018124052c330270696f96771a63b45250a5c17d3000e823355";
    // BLAKE3("nested file\n")
    let nested_hash = "4e3d3cb1a85c88eed97f72907ee8cd68b467b4dd6abb914f4c0c859aa13843e2";

    assert!(
        some_line_has(&output, "BLAKE3", hello_hash),
        "Expected known BLAKE3 hash for hello.txt"
    );
    assert!(
        some_line_has(&output, "BLAKE3", nested_hash),
        "Expected known BLAKE3 hash for nested.txt"
    );

    // Verify format: 64-char hex
    let has_valid_format = output.lines().any(|line| {
        if line.contains("DEBUG: BLAKE3") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.len() >= 3
                && parts[2].len() == 64
                && parts[2].chars().all(|c| c.is_ascii_hexdigit())
        } else {
            false
        }
    });
    assert!(has_valid_format, "BLAKE3 lines must contain 64-char hex");

    assert!(output.contains("Similarities: 4"));
}

// ── --all and -s combinations ────────────────────────────────

#[test]
fn sample_and_hash_combined() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b, "-s", "10", "--all"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SAMPLE, HASH]:")
                .and(predicate::str::contains("Missing/different: 1 (50.00%)")),
        );
}

#[test]
fn sample_on_identical_content() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-s", "10"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE")
                .not()
                .and(predicate::str::contains("Similarities: 4")),
        );
}

// ── --ignore ─────────────────────────────────────────────────

#[test]
fn ignore_nonexistent_path_errors() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-i", "/nonexistent/path/that/does/not/exist"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does not exist").or(
            predicate::str::contains("not found").or(predicate::str::contains("Cannot resolve")),
        ));
}

#[test]
fn ignore_path_outside_trees_errors() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-i", "/tmp"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not within"));
}

#[test]
fn ignore_works_in_backup_tree() {
    let (a, b) = testdata("extras");
    let base = testdata_base("extras");
    let ignore_path = base.join("b").join("extra_dir").to_str().unwrap().to_string();

    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "EXTRA-DIR:", "extra_dir"),
        "extra_dir should be skipped via --ignore"
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1 for ignored extra_dir, got:\n{}",
        output
    );
}

#[test]
fn ignore_works_in_original_tree() {
    let (a, b) = testdata("nested");
    let base = testdata_base("nested");
    // Ignore sub3 in the original tree — it's a missing dir with nested contents
    let ignore_path = base.join("a").join("sub3").to_str().unwrap().to_string();

    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped via --ignore"
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1 for ignored sub3, got:\n{}",
        output
    );
    // sub3/ had 3 missing items (sub3/ + deep/ + file.txt); ignoring it should reduce counts
    // Original: root + sub1/ + ok.txt + missing.txt = 4 (sub3 skipped)
    // Missing: missing.txt = 1
    assert!(
        output.contains("Original items processed: 4"),
        "Expected 4 original items with sub3 ignored, got:\n{}",
        output
    );
}

#[test]
fn ignore_a_file_not_directory() {
    // --ignore on a specific file (not a directory)
    let (a, b) = testdata("missing");
    let base = testdata_base("missing");
    let ignore_path = base
        .join("a")
        .join("also_here.txt")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);

    // also_here.txt should be skipped, not reported as MISSING-FILE
    assert!(
        !some_line_has(&output, "MISSING-FILE:", "also_here.txt"),
        "also_here.txt should be skipped via --ignore, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
    // Root dir + exists.txt remain as original items
    assert!(
        output.contains("Original items processed: 2"),
        "got:\n{}",
        output
    );
}

#[test]
fn ignore_multiple_paths() {
    // -i path1 -i path2
    let (a, b) = testdata("nested");
    let base = testdata_base("nested");
    let ignore1 = base
        .join("a")
        .join("sub3")
        .to_str()
        .unwrap()
        .to_string();
    let ignore2 = base
        .join("b")
        .join("sub2")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd()
        .args([&a, &b, "-i", &ignore1, "-i", &ignore2])
        .assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "EXTRA-DIR:", "sub2"),
        "sub2 should be skipped, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 2"),
        "Expected Skipped: 2, got:\n{}",
        output
    );
}

#[test]
fn ignore_extra_file_in_backup() {
    // B1: --ignore on an extra FILE in backup tree should skip it
    let (a, b) = testdata("extras");
    let base = testdata_base("extras");
    let ignore_path = base
        .join("b")
        .join("extra.txt")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);

    // extra.txt should be skipped, not reported as EXTRA-FILE
    assert!(
        !some_line_has(&output, "EXTRA-FILE:", "extra.txt"),
        "extra.txt should be skipped via --ignore, got:\n{}",
        output
    );
    assert!(
        some_line_has(&output, "SKIP:", "extra.txt"),
        "Expected SKIP: for ignored extra.txt, got:\n{}",
        output
    );
    // extra_dir still reported, so Extras: 2 (extra_dir/ + extra_dir/file.txt)
    assert!(
        output.contains("Extras: 2"),
        "Expected Extras: 2 (extra_dir + its file), got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

#[test]
fn ignore_subdir_inside_missing_dir() {
    // If sub3/ is missing from backup and sub3/deep/ is ignored,
    // count_recursive should skip deep/ (SKIP: + skipped count)
    // but still count sub3/ itself and sub3/deep/file.txt is NOT counted
    // because its parent dir deep/ is skipped entirely.
    let (a, b) = testdata("nested");
    let base = testdata_base("nested");
    let ignore_path = base
        .join("a")
        .join("sub3")
        .join("deep")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd()
        .args([&a, &b, "-v", "-v", "-i", &ignore_path])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    // sub3/ itself should be MISSING-DIR (it's not ignored, only deep/ inside it is)
    assert!(
        some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be reported as MISSING-DIR, got:\n{}",
        output
    );
    // deep/ should be skipped, not counted as missing
    assert!(
        some_line_has(&output, "SKIP:", "deep"),
        "deep/ should be SKIP'd via --ignore, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "MISSING-DIR:", "deep"),
        "deep/ should not be MISSING-DIR (it's ignored), got:\n{}",
        output
    );
    // file.txt inside deep/ should NOT appear (parent skipped)
    assert!(
        !some_line_has(&output, "MISSING-FILE:", "file.txt"),
        "file.txt inside ignored deep/ should not appear, got:\n{}",
        output
    );

    // Counts: root(1) + sub1(1) + ok.txt(1) + missing.txt(1) + sub3(1) = 5 original
    // (deep/ and deep/file.txt are NOT counted because deep/ is skipped)
    assert!(
        output.contains("Original items processed: 5"),
        "Expected 5 original items (deep/ skipped), got:\n{}",
        output
    );
    // Missing: missing.txt(1) + sub3(1) = 2
    assert!(
        output.contains("Missing/different: 2"),
        "Expected Missing/different: 2, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1 for ignored deep/, got:\n{}",
        output
    );
}

#[test]
fn ignore_symlink_to_file() {
    // C3: --ignore on a symlink should use the symlink's own path, not the resolved target.
    // Both sides have a symlink "link" -> "target.txt". Ignoring "link" should skip it.
    let tmp = std::env::temp_dir().join("bv_test_ignore_symlink_file");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("target.txt"), "hello\n").unwrap();
    std::fs::write(b.join("target.txt"), "hello\n").unwrap();
    // Symlinks with different targets → would normally produce DIFFERENT-SYMLINK-TARGET
    std::os::unix::fs::symlink("target.txt", a.join("link")).unwrap();
    std::os::unix::fs::symlink("other.txt", b.join("link")).unwrap();

    let ignore_path = a.join("link").to_str().unwrap().to_string();
    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "-i", &ignore_path])
        .assert()
        .success();
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // The symlink should be skipped, not reported as DIFFERENT-SYMLINK-TARGET
    assert!(
        !some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "link"),
        "link should be skipped via --ignore, got:\n{}",
        output
    );
    assert!(
        some_line_has(&output, "SKIP:", "link"),
        "Expected SKIP: for ignored symlink, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

#[test]
fn ignore_path_through_symlink_parent() {
    // C3 regression: if the path to the ignored entry goes through a symlink parent,
    // canonicalizing the parent resolves the symlink, producing a path that won't
    // match what compare_recursive builds (which keeps symlink names intact).
    //
    // Setup: both sides have symdir -> realdir/ with differing file.txt inside.
    // With --follow, symdir is traversed. Ignoring symdir/file.txt should skip
    // the file even though symdir is itself a symlink.
    // realdir doesn't exist in either tree — only accessed through symdir.
    let tmp = std::env::temp_dir().join("bv_test_ignore_through_symlink");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    let target_a = tmp.join("real_a");
    let target_b = tmp.join("real_b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::create_dir_all(&target_a).unwrap();
    std::fs::create_dir_all(&target_b).unwrap();

    std::fs::write(target_a.join("file.txt"), "aaa\n").unwrap();
    std::fs::write(target_b.join("file.txt"), "bbb\n").unwrap();

    // symdir -> external target dirs (not inside a/ or b/)
    std::os::unix::fs::symlink(&target_a, a.join("symdir")).unwrap();
    std::os::unix::fs::symlink(&target_b, b.join("symdir")).unwrap();

    // Ignore file.txt reached through symdir (symlink parent)
    let ignore_path = a.join("symdir").join("file.txt").to_str().unwrap().to_string();
    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow", "--all", "-i", &ignore_path])
        .assert();
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // file.txt via symdir should be skipped, not reported as DIFFERENT-FILE
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "file.txt"),
        "file.txt through symlink parent should be skipped, got:\n{}",
        output
    );
    // The SKIP should show the symdir path, not the resolved path
    assert!(
        some_line_has(&output, "SKIP:", "symdir"),
        "Expected SKIP: with symdir in path, got:\n{}",
        output
    );
}

#[test]
fn ignore_symlink_to_dir_with_follow() {
    // C3: --ignore on a symlink-to-dir with --follow should skip the entire subtree.
    let tmp = std::env::temp_dir().join("bv_test_ignore_symlink_dir");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    let target_a = tmp.join("real_dir_a");
    let target_b = tmp.join("real_dir_b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::create_dir_all(&target_a).unwrap();
    std::fs::create_dir_all(&target_b).unwrap();

    std::fs::write(target_a.join("file.txt"), "short\n").unwrap();
    std::fs::write(target_b.join("file.txt"), "this is longer content\n").unwrap();

    // Both sides have symlink "linked" -> their respective real dirs
    std::os::unix::fs::symlink(&target_a, a.join("linked")).unwrap();
    std::os::unix::fs::symlink(&target_b, b.join("linked")).unwrap();

    // Also add a normal matching file so we can verify counts
    std::fs::write(a.join("ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("ok.txt"), "ok\n").unwrap();

    let ignore_path = a.join("linked").to_str().unwrap().to_string();
    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow", "-i", &ignore_path])
        .assert()
        .success();
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // The symlink-to-dir should be skipped entirely, no traversal
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "file.txt"),
        "file.txt inside ignored symlink dir should not be compared, got:\n{}",
        output
    );
    assert!(
        some_line_has(&output, "SKIP:", "linked"),
        "Expected SKIP: for ignored symlink dir, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
    // root(1) + ok.txt(1) = 2 original items (linked is skipped)
    assert!(
        output.contains("Original items processed: 2"),
        "Expected 2 original items, got:\n{}",
        output
    );
    assert!(
        output.contains("Similarities: 2"),
        "Expected Similarities: 2, got:\n{}",
        output
    );
}

#[test]
fn ignore_root_directory() {
    // C1: If the root directory itself is ignored, it should not be counted
    // as original/backup/similarity — only as skipped.
    let (a, b) = testdata("identical");
    let assert = cmd()
        .args([&a, &b, "-i", &a])
        .assert()
        .success();
    let output = stdout_of(&assert);

    assert!(
        output.contains("SKIP:"),
        "Expected SKIP for root, got:\n{}",
        output
    );
    // Root should NOT be counted as a processed item or similarity
    assert!(
        output.contains("Original items processed: 0"),
        "Ignored root should not be counted as original item, got:\n{}",
        output
    );
    assert!(
        output.contains("Backup items processed: 0"),
        "Ignored root should not be counted as backup item, got:\n{}",
        output
    );
    assert!(
        output.contains("Similarities: 0"),
        "Ignored root should not be counted as similarity, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

#[test]
fn all_with_ignore_skips_hashing() {
    // --all combined with --ignore: ignored entries should not produce BLAKE3 lines
    let (a, b) = testdata("nested");
    let base = testdata_base("nested");
    let ignore_path = base
        .join("a")
        .join("sub3")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd()
        .args([&a, &b, "--all", "-v", "-v", "-i", &ignore_path])
        .assert();
    let output = stdout_of(&assert);

    // sub3 should not appear in any BLAKE3 line
    assert!(
        !some_line_has(&output, "BLAKE3", "sub3"),
        "sub3 should be skipped, no BLAKE3 hashing, got:\n{}",
        output
    );
    assert!(
        output.contains("SKIP:"),
        "Expected SKIP: for ignored path, got:\n{}",
        output
    );
}
