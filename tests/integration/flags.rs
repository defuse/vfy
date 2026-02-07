use super::harness::{setup_legacy_test_dirs, Entry, Entry::*};
use super::{cmd, some_line_has, stdout_of};
use predicates::prelude::*;

// ===========================================================================
// Entry arrays for testdata scenarios
// ===========================================================================

const IDENTICAL: &[Entry] = &[
    File("hello.txt", "hello world\n"),
    Dir("sub"),
    File("sub/nested.txt", "nested file\n"),
];

const NESTED_ORIG: &[Entry] = &[
    Dir("sub1"),
    File("sub1/ok.txt", "ok\n"),
    File("sub1/missing.txt", "missing\n"),
    Dir("sub3"),
    Dir("sub3/deep"),
    File("sub3/deep/file.txt", "deep\n"),
];

const NESTED_BACKUP: &[Entry] = &[
    Dir("sub1"),
    File("sub1/ok.txt", "ok\n"),
    Dir("sub2"),
    File("sub2/extra.txt", "extra\n"),
];

const EXTRAS_ORIG: &[Entry] = &[File("base.txt", "base content\n")];

const EXTRAS_BACKUP: &[Entry] = &[
    File("base.txt", "base content\n"),
    File("extra.txt", "I'm extra\n"),
    Dir("extra_dir"),
    File("extra_dir/file.txt", "extra dir file\n"),
];

const MISSING_ORIG: &[Entry] = &[
    File("exists.txt", "I exist\n"),
    File("also_here.txt", "me too\n"),
];

const MISSING_BACKUP: &[Entry] = &[File("exists.txt", "I exist\n")];

const DIFF_CONTENT_ORIG: &[Entry] = &[File("file.txt", "aaaa")];
const DIFF_CONTENT_BACKUP: &[Entry] = &[File("file.txt", "bbbb")];

// ── CMD line output ─────────────────────────────────────────

#[test]
fn cmd_line_printed_no_flags() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().expect("expected CMD: line");
    assert!(cmd_line.starts_with("CMD: "), "first line should be CMD:, got: {}", cmd_line);
    assert!(cmd_line.contains(&a), "CMD should contain original path");
    assert!(cmd_line.contains(&b), "CMD should contain backup path");
}

#[test]
fn cmd_line_includes_all_flag() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "--all"]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.contains("--all"), "CMD should contain --all, got: {}", cmd_line);
}

#[test]
fn cmd_line_includes_short_a_flag() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "-a"]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.contains("-a"), "CMD should contain -a, got: {}", cmd_line);
}

#[test]
fn cmd_line_includes_verbose_flags() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.contains("-v"), "CMD should contain -v, got: {}", cmd_line);
    // Should have two -v flags
    let v_count = cmd_line.split_whitespace().filter(|&w| w == "-v").count();
    assert_eq!(v_count, 2, "CMD should contain two -v flags, got: {}", cmd_line);
}

#[test]
fn cmd_line_includes_samples_flag() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "-s", "5"]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.contains("-s") || cmd_line.contains("--samples"),
        "CMD should contain -s or --samples, got: {}", cmd_line);
    assert!(cmd_line.contains("5"), "CMD should contain sample count 5, got: {}", cmd_line);
}

#[test]
fn cmd_line_includes_follow_flag() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "--follow"]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.contains("--follow"), "CMD should contain --follow, got: {}", cmd_line);
}

#[test]
fn cmd_line_includes_ignore_flag() {
    let (_tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);
    let ignore_path = format!("{}/sub3", a);
    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.contains("-i") || cmd_line.contains("--ignore"),
        "CMD should contain -i or --ignore, got: {}", cmd_line);
    assert!(cmd_line.contains("sub3"), "CMD should contain ignore path, got: {}", cmd_line);
}

#[test]
fn cmd_line_includes_one_filesystem_flag() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "--one-filesystem"]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.contains("--one-filesystem"),
        "CMD should contain --one-filesystem, got: {}", cmd_line);
}

#[test]
fn cmd_line_includes_multiple_flags() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "-v", "-v", "--all", "-s", "3", "--follow"]).assert().success();
    let output = stdout_of(&assert);
    let cmd_line = output.lines().next().unwrap();
    assert!(cmd_line.starts_with("CMD: "), "first line should be CMD:");
    assert!(cmd_line.contains("--all"), "CMD should contain --all, got: {}", cmd_line);
    assert!(cmd_line.contains("--follow"), "CMD should contain --follow, got: {}", cmd_line);
    assert!(cmd_line.contains("-s"), "CMD should contain -s, got: {}", cmd_line);
    assert!(cmd_line.contains("3"), "CMD should contain sample count, got: {}", cmd_line);
    let v_count = cmd_line.split_whitespace().filter(|&w| w == "-v").count();
    assert_eq!(v_count, 2, "CMD should have two -v flags, got: {}", cmd_line);
}

#[test]
fn cmd_line_quotes_paths_with_spaces() {
    let tmp = std::env::temp_dir().join("bv_test_cmd_spaces");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("orig dir");
    let b = tmp.join("backup dir");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().success();
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    let cmd_line = output.lines().next().unwrap();
    // Paths with spaces should be single-quoted
    let a_quoted = format!("'{}'", a_str);
    let b_quoted = format!("'{}'", b_str);
    assert!(cmd_line.contains(&a_quoted),
        "CMD should quote path with spaces, got: {}", cmd_line);
    assert!(cmd_line.contains(&b_quoted),
        "CMD should quote path with spaces, got: {}", cmd_line);
}

#[test]
fn cmd_line_escapes_single_quotes_in_paths() {
    let tmp = std::env::temp_dir().join("bv_test_cmd_quotes");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("it's orig");
    let b = tmp.join("it's backup");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().success();
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    let cmd_line = output.lines().next().unwrap();
    // Single quotes in paths should be escaped as '\''
    assert!(cmd_line.contains("'\\''"),
        "CMD should escape single quotes with '\\'' idiom, got: {}", cmd_line);
    // The escaped path should still be parseable — check both dir names appear
    assert!(cmd_line.contains("it") && cmd_line.contains("s orig"),
        "CMD should contain original path content, got: {}", cmd_line);
    assert!(cmd_line.contains("it") && cmd_line.contains("s backup"),
        "CMD should contain backup path content, got: {}", cmd_line);
}

// ── Verbosity ────────────────────────────────────────────────

#[test]
fn verbose_dirs_only() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
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
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);

    assert!(output.contains("DEBUG: Comparing file"));
    assert!(output.contains("Similarities: 4"));
}

#[test]
fn triple_verbose_errors() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    cmd()
        .args([&a, &b, "-vvv"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("at most twice"));
}

#[test]
fn quadruple_verbose_errors() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    cmd()
        .args([&a, &b, "-v", "-v", "-v", "-v"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("specified 4 times"));
}

#[test]
fn verbose_blake3_known_hashes() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
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
    let (_tmp, a, b) = setup_legacy_test_dirs(DIFF_CONTENT_ORIG, DIFF_CONTENT_BACKUP);
    cmd()
        .args([&a, &b, "-s", "10", "--all"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SAMPLE]:")
                .and(predicate::str::contains("Missing: 0 (0.00%)"))
                .and(predicate::str::contains("Different: 1 (50.00%)")),
        );
}

#[test]
fn sample_on_identical_content() {
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
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
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
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
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
    cmd()
        .args([&a, &b, "-i", "/tmp"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not within"));
}

#[test]
fn ignore_works_in_backup_tree() {
    let (_tmp, a, b) = setup_legacy_test_dirs(EXTRAS_ORIG, EXTRAS_BACKUP);
    let ignore_path = format!("{}/extra_dir", b);

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
    let (_tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);
    // Ignore sub3 in the original tree — it's a missing dir with nested contents
    let ignore_path = format!("{}/sub3", a);

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
    let (_tmp, a, b) = setup_legacy_test_dirs(MISSING_ORIG, MISSING_BACKUP);
    let ignore_path = format!("{}/also_here.txt", a);

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
    let (_tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);
    let ignore1 = format!("{}/sub3", a);
    let ignore2 = format!("{}/sub2", b);

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
    let (_tmp, a, b) = setup_legacy_test_dirs(EXTRAS_ORIG, EXTRAS_BACKUP);
    let ignore_path = format!("{}/extra.txt", b);

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
    let (_tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);
    let ignore_path = format!("{}/sub3/deep", a);

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
        output.contains("Missing: 2"),
        "Expected Missing: 2, got:\n{}",
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
        output.contains("Skipped: 2"),
        "Expected Skipped: 2, got:\n{}",
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
        output.contains("Skipped: 2"),
        "Expected Skipped: 2, got:\n{}",
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
    let (_tmp, a, b) = setup_legacy_test_dirs(IDENTICAL, IDENTICAL);
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
        output.contains("Skipped: 2"),
        "Expected Skipped: 2, got:\n{}",
        output
    );
}

#[test]
fn all_with_ignore_skips_hashing() {
    // --all combined with --ignore: ignored entries should not produce BLAKE3 lines
    let (_tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);
    let ignore_path = format!("{}/sub3", a);

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

#[test]
fn ignore_symlink_skips_symlink_but_not_target_dir() {
    // original/foo -> bar (symlink), original/bar/ (real dir with differing content)
    // --ignore original/foo should skip the symlink entry but still traverse bar/
    // --ignore original/bar should skip the real dir but still check the symlink
    let tmp = std::env::temp_dir().join("bv_test_ignore_symlink_vs_target");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(a.join("bar")).unwrap();
    std::fs::create_dir_all(b.join("bar")).unwrap();

    // bar/ has differing content so we can tell if it's traversed
    std::fs::write(a.join("bar").join("file.txt"), "original\n").unwrap();
    std::fs::write(b.join("bar").join("file.txt"), "different content here\n").unwrap();

    // foo -> bar (symlink in both sides, same target)
    std::os::unix::fs::symlink("bar", a.join("foo")).unwrap();
    std::os::unix::fs::symlink("bar", b.join("foo")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();

    // Test 1: --ignore foo → symlink skipped, bar/ still traversed (diff detected)
    let ignore_foo = a.join("foo").to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "-i", &ignore_foo])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    assert!(
        some_line_has(&output, "SKIP:", "foo"),
        "foo symlink should be skipped, got:\n{}",
        output
    );
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "file.txt"),
        "bar/file.txt should still be compared and differ, got:\n{}",
        output
    );

    // Test 2: --ignore bar → real dir skipped, foo symlink still checked
    let ignore_bar = a.join("bar").to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "-i", &ignore_bar])
        .assert()
        .success();
    let output = stdout_of(&assert);

    assert!(
        some_line_has(&output, "SKIP:", "bar"),
        "bar dir should be skipped, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "file.txt"),
        "bar/file.txt should not be compared (bar skipped), got:\n{}",
        output
    );
    // foo symlink should still be checked — both have same target so it's a similarity
    assert!(
        !some_line_has(&output, "SKIP:", "foo"),
        "foo should NOT be skipped (only bar is ignored), got:\n{}",
        output
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ignore_path_through_symlinked_root_works() {
    // Roots and --ignore path all go through a symlink alias.
    // canonicalize() resolves the roots but the ignore path is normalized
    // against the as-typed root, so strip_prefix works and the suffix is
    // rejoined onto the canonical root.
    let tmp = std::env::temp_dir().join("bv_test_ignore_symlinked_root");
    let _ = std::fs::remove_dir_all(&tmp);
    let real_dir = tmp.join("real");
    std::fs::create_dir_all(real_dir.join("a").join("sub")).unwrap();
    std::fs::create_dir_all(real_dir.join("b").join("sub")).unwrap();
    std::fs::write(real_dir.join("a").join("sub").join("f.txt"), "a\n").unwrap();
    std::fs::write(real_dir.join("b").join("sub").join("f.txt"), "a\n").unwrap();

    // Create a symlink alias to real_dir
    let alias = tmp.join("alias");
    std::os::unix::fs::symlink(&real_dir, &alias).unwrap();

    // Pass roots and --ignore all via the symlink alias
    let a_str = alias.join("a").to_str().unwrap().to_string();
    let b_str = alias.join("b").to_str().unwrap().to_string();
    let ignore_path = alias.join("a").join("sub").to_str().unwrap().to_string();

    let assert = cmd()
        .args([&a_str, &b_str, "-i", &ignore_path])
        .assert()
        .success();
    let output = stdout_of(&assert);

    assert!(
        some_line_has(&output, "SKIP:", "sub"),
        "sub should be skipped via --ignore, got:\n{}",
        output
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// Reproduces the macOS /var -> /private/var bug on Linux:
// When the temp directory path goes through a symlink, canonicalize()
// resolves it for orig/backup roots but normalize_path() doesn't for
// --ignore paths, so the starts_with check fails.
#[test]
fn ignore_fails_when_tempdir_behind_symlink() {
    let tmp = std::env::temp_dir().join("bv_test_ignore_symlink_tmpdir");
    let _ = std::fs::remove_dir_all(&tmp);
    let real = tmp.join("real");
    std::fs::create_dir_all(real.join("a").join("sub")).unwrap();
    std::fs::create_dir_all(real.join("b").join("sub")).unwrap();
    std::fs::write(real.join("a").join("sub").join("f.txt"), "a\n").unwrap();
    std::fs::write(real.join("b").join("sub").join("f.txt"), "a\n").unwrap();

    // Symlink: tmp/link -> tmp/real (simulates /var -> /private/var)
    let link = tmp.join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    // Pass roots through the symlink — canonicalize() will resolve to real/
    let a_via_link = link.join("a").to_str().unwrap().to_string();
    let b_via_link = link.join("b").to_str().unwrap().to_string();
    // Ignore path also through the symlink — normalize_path() won't resolve it
    let ignore_via_link = link.join("a").join("sub").to_str().unwrap().to_string();

    // Should succeed: the ignore path IS within the original tree.
    // Currently fails because canonicalize resolves the symlink for roots
    // while normalize_path leaves it unresolved for ignore paths.
    // On macOS this happens naturally because /var -> /private/var.
    let assert = cmd()
        .args([&a_via_link, &b_via_link, "-i", &ignore_via_link])
        .assert()
        .success();
    let output = stdout_of(&assert);

    // sub/ should be ignored, not reported as missing or similar
    assert!(
        some_line_has(&output, "SKIP:", "sub"),
        "sub should be skipped via --ignore, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "MISSING", "sub"),
        "sub should not be reported as missing, got:\n{}",
        output
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ignore_canonical_form_when_root_typed_through_symlink() {
    // Root typed through symlink, but ignore path uses the canonical/resolved form.
    // Exercises the fallback strip_prefix(&original) branch.
    let tmp = std::env::temp_dir().join("bv_test_ignore_canonical_form");
    let _ = std::fs::remove_dir_all(&tmp);
    let real = tmp.join("real");
    std::fs::create_dir_all(real.join("a").join("sub")).unwrap();
    std::fs::create_dir_all(real.join("b").join("sub")).unwrap();
    std::fs::write(real.join("a").join("sub").join("f.txt"), "a\n").unwrap();
    std::fs::write(real.join("b").join("sub").join("f.txt"), "a\n").unwrap();

    let link = tmp.join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    // Roots through symlink, but ignore via the truly canonical path.
    // Must canonicalize real/ because on macOS /tmp itself is a symlink
    // to /private/tmp, so real/ alone isn't fully canonical.
    let a_via_link = link.join("a").to_str().unwrap().to_string();
    let b_via_link = link.join("b").to_str().unwrap().to_string();
    let real_canonical = real.canonicalize().unwrap();
    let ignore_via_real = real_canonical.join("a").join("sub").to_str().unwrap().to_string();

    let assert = cmd()
        .args([&a_via_link, &b_via_link, "-i", &ignore_via_real])
        .assert()
        .success();
    let output = stdout_of(&assert);

    assert!(
        some_line_has(&output, "SKIP:", "sub"),
        "sub should be skipped via canonical --ignore path, got:\n{}",
        output
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ignore_through_different_symlink_alias_errors() {
    // Root typed through one symlink, ignore through a different symlink to
    // the same place. This should error — we can't resolve arbitrary aliases.
    let tmp = std::env::temp_dir().join("bv_test_ignore_third_alias");
    let _ = std::fs::remove_dir_all(&tmp);
    let real = tmp.join("real");
    std::fs::create_dir_all(real.join("a").join("sub")).unwrap();
    std::fs::create_dir_all(real.join("b").join("sub")).unwrap();
    std::fs::write(real.join("a").join("sub").join("f.txt"), "a\n").unwrap();
    std::fs::write(real.join("b").join("sub").join("f.txt"), "a\n").unwrap();

    let link1 = tmp.join("link1");
    let link2 = tmp.join("link2");
    std::os::unix::fs::symlink(&real, &link1).unwrap();
    std::os::unix::fs::symlink(&real, &link2).unwrap();

    // Roots through link1, ignore through link2
    let a_str = link1.join("a").to_str().unwrap().to_string();
    let b_str = link1.join("b").to_str().unwrap().to_string();
    let ignore_path = link2.join("a").join("sub").to_str().unwrap().to_string();

    cmd()
        .args([&a_str, &b_str, "-i", &ignore_path])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not within"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ignore_symlink_with_follow_does_not_ignore_target() {
    // Ignoring symlink "thelink" should NOT also ignore its target "realdir"
    // with --follow. Both sides have realdir/ with differing content.
    // --ignore original/thelink with --follow: thelink is skipped, realdir
    // is still compared.
    let tmp = std::env::temp_dir().join("bv_test_ign_link_not_tgt");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(a.join("realdir")).unwrap();
    std::fs::create_dir_all(b.join("realdir")).unwrap();

    std::fs::write(a.join("realdir").join("file.txt"), "original\n").unwrap();
    std::fs::write(b.join("realdir").join("file.txt"), "different content\n").unwrap();

    // thelink -> realdir in both sides
    std::os::unix::fs::symlink("realdir", a.join("thelink")).unwrap();
    std::os::unix::fs::symlink("realdir", b.join("thelink")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let ignore_path = a.join("thelink").to_str().unwrap().to_string();

    let assert = cmd()
        .args([&a_str, &b_str, "--follow", "-i", &ignore_path])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    // thelink should be skipped
    assert!(
        some_line_has(&output, "SKIP:", "thelink"),
        "thelink should be skipped via --ignore, got:\n{}",
        output
    );
    // realdir should NOT be skipped — it's the target, not the ignored path
    assert!(
        !some_line_has(&output, "SKIP:", "realdir"),
        "realdir should NOT be skipped (only thelink is ignored), got:\n{}",
        output
    );
    // realdir/file.txt should still show a difference
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "file.txt"),
        "realdir/file.txt should still be compared and differ, got:\n{}",
        output
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ignore_dangling_symlink() {
    // Ignoring a dangling symlink (target doesn't exist) should work.
    // The symlink itself exists, so the existence check passes.
    let tmp = std::env::temp_dir().join("bv_test_ignore_dangling");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Dangling symlinks in both sides (target doesn't exist)
    std::os::unix::fs::symlink("nonexistent_target", a.join("dangling")).unwrap();
    std::os::unix::fs::symlink("nonexistent_target", b.join("dangling")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let ignore_path = a.join("dangling").to_str().unwrap().to_string();

    let assert = cmd()
        .args([&a_str, &b_str, "-i", &ignore_path])
        .assert()
        .success();
    let output = stdout_of(&assert);

    assert!(
        some_line_has(&output, "SKIP:", "dangling"),
        "dangling symlink should be skipped via --ignore, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "DANGLING", "dangling"),
        "dangling symlink should not be reported as dangling (it's ignored), got:\n{}",
        output
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── --ignore with relative paths (exercises normalize_path) ──

#[test]
fn ignore_relative_path_with_dot_component() {
    // Relative path with ./ should be normalized and work correctly.
    // Run from the temp directory, ignore "./a/sub3"
    let (tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);

    let assert = cmd()
        .current_dir(tmp.path())
        .args([&a, &b, "-i", "./a/sub3"])
        .assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped via relative --ignore, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

#[test]
fn ignore_relative_path_with_parent_component() {
    // Relative path with ../ should be normalized and resolved correctly.
    // Run from tmp/a, ignore "../a/sub3" (goes up then back down)
    let (tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);

    let assert = cmd()
        .current_dir(tmp.path().join("a"))
        .args([&a, &b, "-i", "../a/sub3"])
        .assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped via ../ relative --ignore, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

#[test]
fn ignore_relative_path_complex_normalization() {
    // Path with lots of redundant ./ and ../ components:
    // ./a/../a/./sub3/../sub3/./
    // Should normalize to <cwd>/a/sub3
    let (tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);

    let assert = cmd()
        .current_dir(tmp.path())
        .args([&a, &b, "-i", "./a/../a/./sub3/../sub3/."])
        .assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped via complex relative --ignore, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

#[test]
fn ignore_relative_path_deeply_nested_parent_refs() {
    // Go deep then climb back out: a/sub3/deep/../../sub3
    // From tmp dir, this should resolve to <tmp>/a/sub3
    let (tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);

    let assert = cmd()
        .current_dir(tmp.path())
        .args([&a, &b, "-i", "a/sub3/deep/../../sub3"])
        .assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped via deeply nested ../ path, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

#[test]
fn ignore_relative_path_bare_name() {
    // Just a bare relative name, no ./ or ../ — still triggers the relative branch
    // Run from tmp/a so "sub3" resolves to tmp/a/sub3
    let (tmp, a, b) = setup_legacy_test_dirs(NESTED_ORIG, NESTED_BACKUP);

    let assert = cmd()
        .current_dir(tmp.path().join("a"))
        .args([&a, &b, "-i", "sub3"])
        .assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped via bare relative --ignore, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
}

// ===========================================================================
// BUG EXPOSURE: --ignore SKIP not printed without -vv
// ===========================================================================

/// Control test: with -vv, SKIP should appear for ignored nested files (current behavior works)
#[test]
fn ignore_nested_in_missing_dir_with_vv_prints_skip() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    std::fs::create_dir_all(a.join("missing_dir")).unwrap();
    std::fs::write(a.join("missing_dir/ignored.txt"), "x").unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "-i", &a.join("missing_dir/ignored.txt").to_string_lossy(),
            "-vv",  // With verbose, SKIP should appear
        ])
        .assert()
        .code(1);

    let output = stdout_of(&assert);

    // This should pass - with -vv, SKIP is printed (when ignoring same-side path)
    assert!(output.contains("SKIP:"),
        "SKIP should appear with -vv\nOutput:\n{}", output);

    // Verify ALL counts
    // Skipped items are NOT counted in "items processed"
    // Structure: root(1) + missing_dir(1) = 2 original items (ignored.txt skipped)
    assert!(output.contains("Original items processed: 2"),
        "Expected Original: 2\nOutput:\n{}", output);
    assert!(output.contains("Backup items processed: 1"),    // root only
        "Expected Backup: 1\nOutput:\n{}", output);
    assert!(output.contains("Missing: 1"),                   // missing_dir only (NOT ignored.txt)
        "Expected Missing: 1\nOutput:\n{}", output);
    assert!(output.contains("Different: 0"),
        "Expected Different: 0\nOutput:\n{}", output);
    assert!(output.contains("Extras: 0"),
        "Expected Extras: 0\nOutput:\n{}", output);
    assert!(output.contains("Special files: 0"),
        "Expected Special: 0\nOutput:\n{}", output);
    assert!(output.contains("Similarities: 1"),              // root dir
        "Expected Similarities: 1\nOutput:\n{}", output);
    assert!(output.contains("Skipped: 1"),                   // ignored.txt
        "Expected Skipped: 1\nOutput:\n{}", output);
    assert!(output.contains("Errors: 0"),
        "Expected Errors: 0\nOutput:\n{}", output);
}

/// BUG EXPOSURE: Test that ignored paths inside missing directories print SKIP even without -vv
#[test]
fn ignore_nested_in_missing_dir_prints_skip() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    // Create structure in orig only (will be reported as missing)
    std::fs::create_dir_all(a.join("missing_dir")).unwrap();
    std::fs::write(a.join("missing_dir/ignored.txt"), "x").unwrap();
    std::fs::write(a.join("missing_dir/other.txt"), "y").unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "-i", &a.join("missing_dir/ignored.txt").to_string_lossy(),
            // NO -vv flag - testing default verbosity
        ])
        .assert()
        .code(1);

    let output = stdout_of(&assert);

    // BUG: Currently SKIP is NOT printed without -vv
    // Expected behavior: SKIP should always print for ignored paths
    // The Skipped: 1 count IS correct, but the SKIP: line is missing
    assert!(output.contains("SKIP:"),
        "SKIP should appear for ignored.txt even without -vv\nOutput:\n{}", output);
}

// ===========================================================================
// --ignore symmetry tests
// ===========================================================================
// Note: The tool requires ignore paths to exist, so we can only ignore
// paths that actually exist in one of the trees. The following tests verify
// that ignoring an existing backup-side path works correctly for extras,
// and ignoring an existing original-side path works correctly for missing.

/// Test that ignoring a backup extra file via backup path (b/...) works correctly
/// This is a control test - ignoring the path that exists should work.
#[test]
fn ignore_backup_path_for_backup_extra() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    // File only in backup (would be reported as extra)
    std::fs::write(b.join("should_skip.txt"), "backup only").unwrap();

    // Ignore using BACKUP path (which exists)
    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "-i", &b.join("should_skip.txt").to_string_lossy(),
        ])
        .assert()
        .success();  // Should be success since only item is skipped

    let output = stdout_of(&assert);

    // This should work - ignoring the path that exists
    assert!(output.contains("SKIP:"),
        "SKIP should appear when backup path ignored\nOutput:\n{}", output);
    assert!(!output.contains("EXTRA-FILE"),
        "should_skip.txt should not be reported as extra\nOutput:\n{}", output);

    // Verify counts
    assert!(output.contains("Original items processed: 1"),  // root only
        "Expected Original: 1\nOutput:\n{}", output);
    assert!(output.contains("Backup items processed: 1"),    // root only (skip not counted)
        "Expected Backup: 1\nOutput:\n{}", output);
    assert!(output.contains("Extras: 0"),                    // should_skip.txt ignored
        "Expected Extras: 0\nOutput:\n{}", output);
    assert!(output.contains("Similarities: 1"),              // root dir
        "Expected Similarities: 1\nOutput:\n{}", output);
    assert!(output.contains("Skipped: 1"),                   // should_skip.txt
        "Expected Skipped: 1\nOutput:\n{}", output);
}

/// Test that ignoring an original missing file via original path (a/...) works correctly
/// This is a control test - ignoring the path that exists should work.
#[test]
fn ignore_orig_path_for_orig_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    // File only in original (would be reported as missing)
    std::fs::write(a.join("should_skip.txt"), "orig only").unwrap();

    // Ignore using ORIGINAL path (which exists)
    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "-i", &a.join("should_skip.txt").to_string_lossy(),
        ])
        .assert()
        .success();  // Should be success since only item is skipped

    let output = stdout_of(&assert);

    // This should work - ignoring the path that exists
    assert!(output.contains("SKIP:"),
        "SKIP should appear when orig path ignored\nOutput:\n{}", output);
    assert!(!output.contains("MISSING-FILE"),
        "should_skip.txt should not be reported as missing\nOutput:\n{}", output);

    // Verify counts
    assert!(output.contains("Original items processed: 1"),  // root only (skip not counted)
        "Expected Original: 1\nOutput:\n{}", output);
    assert!(output.contains("Backup items processed: 1"),    // root only
        "Expected Backup: 1\nOutput:\n{}", output);
    assert!(output.contains("Missing: 0"),                   // should_skip.txt ignored
        "Expected Missing: 0\nOutput:\n{}", output);
    assert!(output.contains("Similarities: 1"),              // root dir
        "Expected Similarities: 1\nOutput:\n{}", output);
    assert!(output.contains("Skipped: 1"),                   // should_skip.txt
        "Expected Skipped: 1\nOutput:\n{}", output);
}

/// Test ignoring a nested file inside a missing directory via original path
#[test]
fn ignore_orig_path_for_nested_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    // Missing directory in original with files inside
    std::fs::create_dir_all(a.join("missing_dir")).unwrap();
    std::fs::write(a.join("missing_dir/should_skip.txt"), "nested in missing").unwrap();
    std::fs::write(a.join("missing_dir/other.txt"), "also nested").unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Ignore using ORIGINAL path (which exists)
    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "-i", &a.join("missing_dir/should_skip.txt").to_string_lossy(),
            "-vv",  // Need verbose to see individual children
        ])
        .assert()
        .code(1);

    let output = stdout_of(&assert);

    // should_skip.txt should be skipped
    assert!(output.contains("SKIP:"),
        "SKIP should appear for should_skip.txt\nOutput:\n{}", output);
    assert!(!some_line_has(&output, "MISSING-FILE", "should_skip.txt"),
        "should_skip.txt should not be reported as missing\nOutput:\n{}", output);
    // other.txt SHOULD still be reported as missing
    assert!(some_line_has(&output, "MISSING-FILE", "other.txt"),
        "other.txt should be reported as missing\nOutput:\n{}", output);

    // Verify counts - skipped items don't count toward "processed"
    assert!(output.contains("Original items processed: 3"),  // root + missing_dir + other.txt (skip not counted)
        "Expected Original: 3\nOutput:\n{}", output);
    assert!(output.contains("Backup items processed: 1"),    // root only
        "Expected Backup: 1\nOutput:\n{}", output);
    assert!(output.contains("Missing: 2"),                   // missing_dir + other.txt (NOT should_skip)
        "Expected Missing: 2\nOutput:\n{}", output);
    assert!(output.contains("Similarities: 1"),              // root dir
        "Expected Similarities: 1\nOutput:\n{}", output);
    assert!(output.contains("Skipped: 1"),                   // should_skip.txt
        "Expected Skipped: 1\nOutput:\n{}", output);
}

/// Test ignoring a nested file inside an extra directory via backup path
#[test]
fn ignore_backup_path_for_nested_extra() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");

    std::fs::create_dir_all(&a).unwrap();
    // Extra directory in backup with files inside
    std::fs::create_dir_all(b.join("extra_dir")).unwrap();
    std::fs::write(b.join("extra_dir/should_skip.txt"), "nested in extra").unwrap();
    std::fs::write(b.join("extra_dir/other.txt"), "also nested").unwrap();

    // Ignore using BACKUP path (which exists)
    let assert = cmd()
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "-i", &b.join("extra_dir/should_skip.txt").to_string_lossy(),
            "-vv",  // Need verbose to see individual children
        ])
        .assert()
        .code(1);

    let output = stdout_of(&assert);

    // should_skip.txt should be skipped
    assert!(output.contains("SKIP:"),
        "SKIP should appear for should_skip.txt\nOutput:\n{}", output);
    assert!(!some_line_has(&output, "EXTRA-FILE", "should_skip.txt"),
        "should_skip.txt should not be reported as extra\nOutput:\n{}", output);
    // other.txt SHOULD still be reported as extra
    assert!(some_line_has(&output, "EXTRA-FILE", "other.txt"),
        "other.txt should be reported as extra\nOutput:\n{}", output);

    // Verify counts - skipped items don't count toward "processed"
    assert!(output.contains("Original items processed: 1"),  // root only
        "Expected Original: 1\nOutput:\n{}", output);
    assert!(output.contains("Backup items processed: 3"),    // root + extra_dir + other.txt (skip not counted)
        "Expected Backup: 3\nOutput:\n{}", output);
    assert!(output.contains("Extras: 2"),                    // extra_dir + other.txt (NOT should_skip)
        "Expected Extras: 2\nOutput:\n{}", output);
    assert!(output.contains("Similarities: 1"),              // root dir
        "Expected Similarities: 1\nOutput:\n{}", output);
    assert!(output.contains("Skipped: 1"),                   // should_skip.txt
        "Expected Skipped: 1\nOutput:\n{}", output);
}
