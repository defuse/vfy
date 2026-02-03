use super::{cmd, some_line_has, stdout_of, testdata, testdata_base};

#[test]
fn symlink_dir_no_follow() {
    let (a, b) = testdata("symlink_dirs");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);

    // Without --follow, symlink-to-dir → SYMLINK: and not traversed
    assert!(
        some_line_has(&output, "SYMLINK:", "/link_dir"),
        "Expected SYMLINK: for link_dir without --follow, got:\n{}",
        output
    );
    // Symlink-to-dir should be counted as skipped
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1 for symlink-to-dir without --follow, got:\n{}",
        output
    );
}

#[test]
fn symlink_dir_with_follow() {
    let (a, b) = testdata("symlink_dirs");
    let assert = cmd()
        .args([&a, &b, "--follow", "-v", "-v"])
        .assert()
        .success();
    let output = stdout_of(&assert);

    assert!(
        !output.contains("SYMLINK:"),
        "Should not produce SYMLINK: with --follow"
    );
    // Files inside the symlinked dir should be traversed
    assert!(
        some_line_has(&output, "DEBUG: Comparing file", "/link_dir/"),
        "Expected traversal into link_dir with --follow, got:\n{}",
        output
    );
}

#[test]
fn symlink_target_mismatch() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // target_diff: both symlinks but point to different files
    assert!(
        some_line_has(&output, "SYMMIS:", "target_diff"),
        "Expected SYMMIS for target_diff, got:\n{}",
        output
    );
    assert!(
        output.contains("targets differ"),
        "Expected 'targets differ' message"
    );
}

#[test]
fn symlink_type_mismatch() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // type_mis: symlink in a, regular file in b
    assert!(
        some_line_has(&output, "SYMMIS:", "type_mis"),
        "Expected SYMMIS for type_mis (symlink vs regular), got:\n{}",
        output
    );
}

#[test]
fn symlink_missing_from_backup() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // missing_link: symlink in a, absent from b → MISSING-FILE
    assert!(
        some_line_has(&output, "MISSING-FILE:", "missing_link"),
        "Expected MISSING-FILE for missing_link, got:\n{}",
        output
    );
}

#[test]
fn symlink_mismatch_summary() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // a/ has: root + missing_link, ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 7 items
    // In backup: root + ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 6 backup items
    // Missing: missing_link = 1
    // Different: target_diff (SYMMIS) + type_mis (SYMMIS) = 2
    // Similarities: root + ok.txt + real_a.txt + real_b.txt = 4
    assert!(output.contains("Original items processed: 7"),
        "got:\n{}", output);
    assert!(output.contains("Backup items processed: 6"),
        "got:\n{}", output);
    assert!(output.contains("Similarities: 4"),
        "got:\n{}", output);

    // Verify no ERROR lines — these are all expected conditions
    assert!(
        !output.contains("ERROR:"),
        "Should have no errors, got:\n{}",
        output
    );
}

// ── Matching symlinks ───────────────────────────────────────

#[test]
fn matching_symlinks_are_similar() {
    // Both sides have a symlink pointing to the same target → should count as similarity
    let (a, b) = testdata("symlink_matching");
    let assert = cmd().args([&a, &b]).assert().success();
    let output = stdout_of(&assert);

    assert!(
        !output.contains("SYMMIS:"),
        "Matching symlinks should not produce SYMMIS, got:\n{}",
        output
    );
    assert!(
        !output.contains("DIFFERENT-FILE"),
        "Matching symlinks should not produce DIFFERENT-FILE, got:\n{}",
        output
    );
    // 3 items: root + real.txt + link
    assert!(
        output.contains("Original items processed: 3"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Similarities: 3"),
        "got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}

// ── --follow with differences inside symlinked dir ──────────

#[test]
fn symlink_follow_finds_differences() {
    let (a, b) = testdata("symlink_follow_diff");
    let assert = cmd()
        .args([&a, &b, "--follow", "-v", "-v"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    // Traversal should find missing file inside the symlinked dir
    assert!(
        some_line_has(&output, "MISSING-FILE:", "only_orig.txt"),
        "Expected MISSING-FILE for only_orig.txt inside followed symlink dir, got:\n{}",
        output
    );
    // No SYMLINK: lines since we used --follow
    assert!(
        !output.contains("SYMLINK:"),
        "Should not produce SYMLINK: with --follow, got:\n{}",
        output
    );
}

// ── Dangling symlinks ───────────────────────────────────────

#[test]
fn dangling_symlinks_same_target() {
    // Both sides have dangling symlinks to the same nonexistent target
    let (a, b) = testdata("dangling_symlink");
    let assert = cmd().args([&a, &b]).assert();
    let output = stdout_of(&assert);

    // Both dangling symlinks point to same target → no SYMMIS
    assert!(
        !some_line_has(&output, "SYMMIS:", "dangling"),
        "Dangling symlinks with same target should not SYMMIS, got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}

#[test]
fn dangling_symlinks_different_targets() {
    // Dangling symlinks point to different targets
    let (a, b) = testdata("dangling_symlink_diff");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    assert!(
        some_line_has(&output, "SYMMIS:", "dangling"),
        "Dangling symlinks with different targets should SYMMIS, got:\n{}",
        output
    );
    assert!(
        output.contains("targets differ"),
        "Expected 'targets differ' message, got:\n{}",
        output
    );
}

// ── Extra symlink in backup ─────────────────────────────────

#[test]
fn extra_symlink_in_backup() {
    let (a, b) = testdata("extra_symlink");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // Extra symlink should be reported as EXTRA-FILE (symlink_metadata.is_dir() = false)
    assert!(
        some_line_has(&output, "EXTRA-FILE:", "extra_link"),
        "Expected EXTRA-FILE for extra symlink, got:\n{}",
        output
    );
    assert!(output.contains("Extras: 1"), "got:\n{}", output);
}

// ── File symlinks with --follow ─────────────────────────────

#[test]
fn file_symlink_with_follow_compares_content() {
    // Both sides have link -> target.txt (same symlink target),
    // but the actual file content at target.txt differs.
    // With --follow, the symlink should compare resolved content too.
    let tmp = std::env::temp_dir().join("bv_test_file_symlink_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("target.txt"), "original content\n").unwrap();
    std::fs::write(b.join("target.txt"), "different backup content\n").unwrap();

    std::os::unix::fs::symlink("target.txt", a.join("link")).unwrap();
    std::os::unix::fs::symlink("target.txt", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str, "--follow"]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // target.txt is caught as different (regular file comparison)
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "target.txt"),
        "Expected DIFFERENT-FILE for target.txt, got:\n{}",
        output
    );
    // With --follow, link should ALSO be caught as different (resolved content differs)
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "/link"),
        "Expected DIFFERENT-FILE for link with --follow, got:\n{}",
        output
    );
}

#[test]
fn file_symlink_without_follow_checks_target_only() {
    // Same setup but WITHOUT --follow.
    // link should be similarity (targets match), only target.txt different.
    let tmp = std::env::temp_dir().join("bv_test_file_symlink_no_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("target.txt"), "original content\n").unwrap();
    std::fs::write(b.join("target.txt"), "different backup content\n").unwrap();

    std::os::unix::fs::symlink("target.txt", a.join("link")).unwrap();
    std::os::unix::fs::symlink("target.txt", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // target.txt is caught as different (regular file)
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "target.txt"),
        "Expected DIFFERENT-FILE for target.txt, got:\n{}",
        output
    );
    // Without --follow, link should NOT be different (targets match → similarity)
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "/link"),
        "link should be similarity without --follow (targets match), got:\n{}",
        output
    );
}
