use super::{cmd, some_line_has, stdout_of, testdata};

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

    // a/ has: missing_link, ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 6 items
    // In backup: ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 5 backup items
    // Missing: missing_link = 1
    // Different: target_diff (SYMMIS) + type_mis (SYMMIS) = 2
    // Similarities: ok.txt + real_a.txt + real_b.txt = 3
    assert!(output.contains("Original items processed: 6"),
        "got:\n{}", output);
    assert!(output.contains("Backup items processed: 5"),
        "got:\n{}", output);
    assert!(output.contains("Similarities: 3"),
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
    // 2 items: real.txt + link
    assert!(
        output.contains("Original items processed: 2"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Similarities: 2"),
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
