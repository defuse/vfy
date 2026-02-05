//! Symlink-related integration tests using the case! macro infrastructure.
//!
//! Tests cover: symlink matching, target mismatches, type mismatches,
//! dangling symlinks, --follow behavior, and various edge cases.
//!
//! Note: Many symlink scenarios are also covered comprehensively in matrix.rs.
//! Tests here focus on specific symlink behaviors and edge cases.

use super::harness::Entry::*;
use crate::case;

// ===========================================================================
// Basic symlink matching and mismatches
// ===========================================================================

// Both sides have identical symlinks pointing to the same target file
// Symlinks match → similarity, but without --follow they're marked SYMLINK-SKIPPED
case!(matching_symlinks_are_similar {
    orig: [
        File("real.txt", "data\n"),
        Sym("link", "real.txt"),
    ],
    backup: [
        File("real.txt", "data\n"),
        Sym("link", "real.txt"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link",
    ],
    // root + real.txt + link = 3 items
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 1,
    errors: 0,
});

// Symlinks point to different targets → DIFFERENT-SYMLINK-TARGET
case!(symlink_target_mismatch {
    orig: [
        File("real_a.txt", "aaa\n"),
        File("real_b.txt", "bbb\n"),
        Sym("link", "real_a.txt"),
    ],
    backup: [
        File("real_a.txt", "aaa\n"),
        File("real_b.txt", "bbb\n"),
        Sym("link", "real_b.txt"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link",
        "SYMLINK-SKIPPED: a/link",
    ],
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 1,
    errors: 0,
});

// One side has symlink, other has regular file → DIFFERENT-SYMLINK-STATUS
case!(symlink_type_mismatch {
    orig: [
        File("real.txt", "aaa\n"),
        Sym("entry", "real.txt"),
    ],
    backup: [
        File("real.txt", "aaa\n"),
        File("entry", "regular\n"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-SYMLINK: a/entry",
        "EXTRA-FILE: b/entry",
    ],
    original_processed: 3,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// Symlink in orig, absent from backup → MISSING-SYMLINK
case!(symlink_missing_from_backup {
    orig: [
        File("real.txt", "aaa\n"),
        Sym("missing_link", "real.txt"),
    ],
    backup: [
        File("real.txt", "aaa\n"),
    ],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/missing_link",
    ],
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// Extra symlink in backup → EXTRA-SYMLINK
case!(extra_symlink_in_backup {
    orig: [
        File("file.txt", "data\n"),
    ],
    backup: [
        File("file.txt", "data\n"),
        Sym("extra_link", "file.txt"),
    ],
    flags: [],
    lines: [
        "EXTRA-SYMLINK: b/extra_link",
    ],
    original_processed: 2,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 1,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Symlinks to directories
// ===========================================================================

// Both sides have symlink to directory with identical content
// Without --follow, symlinks are skipped (not traversed)
case!(symlink_dir_no_follow {
    orig: [
        Dir("real_dir"),
        File("real_dir/file.txt", "in real dir\n"),
        Sym("link_dir", "real_dir"),
    ],
    backup: [
        Dir("real_dir"),
        File("real_dir/file.txt", "in real dir\n"),
        Sym("link_dir", "real_dir"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link_dir",
    ],
    // root + real_dir + real_dir/file.txt + link_dir = 4 items
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 1,
    errors: 0,
});

// With --follow, symlink to directory is traversed and content compared
case!(symlink_dir_with_follow {
    orig: [
        Dir("real_dir"),
        File("real_dir/file.txt", "in real dir\n"),
        Sym("link_dir", "real_dir"),
    ],
    backup: [
        Dir("real_dir"),
        File("real_dir/file.txt", "in real dir\n"),
        Sym("link_dir", "real_dir"),
    ],
    flags: ["--follow", "-vv"],
    lines: [],
    // Verify traversal into symlinked directory
    debug_contains: ["Comparing", "link_dir"],
    debug_excludes: [],
    // With --follow, link_dir is traversed as if it were real_dir
    // Items: root + real_dir + real_dir/file.txt + link_dir + link_dir/file.txt + link_dir resolved = 6
    original_processed: 6,
    backup_processed: 6,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 6,
    skipped: 0,
    errors: 0,
});

// Symlinks to different directories with different targets
// Without --follow: DIFFERENT-SYMLINK-TARGET + SYMLINK-SKIPPED
case!(symlink_dir_different_targets_no_follow {
    orig: [
        Dir("dir1"),
        File("dir1/file.txt", "from dir1\n"),
        Sym("link", "dir1"),
    ],
    backup: [
        Dir("dir2"),
        File("dir2/file.txt", "from dir2\n"),
        Sym("link", "dir2"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link",
        "SYMLINK-SKIPPED: a/link",
        "MISSING-DIR: a/dir1",
        "EXTRA-DIR: b/dir2",
    ],
    original_processed: 4,
    backup_processed: 4,
    missing: 2,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

// Symlinks to different directories with --follow: traverse and compare content
case!(symlink_dir_different_targets_with_follow {
    orig: [
        Dir("dir1"),
        File("dir1/shared.txt", "same content\n"),
        File("dir1/only_orig.txt", "only in dir1\n"),
        Sym("link", "dir1"),
    ],
    backup: [
        Dir("dir2"),
        File("dir2/shared.txt", "same content\n"),
        File("dir2/only_backup.txt", "only in dir2\n"),
        Sym("link", "dir2"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link",
        "MISSING-FILE: a/link/only_orig.txt",
        "EXTRA-FILE: b/link/only_backup.txt",
        "MISSING-DIR: a/dir1",
        "EXTRA-DIR: b/dir2",
    ],
    // With --follow: link is traversed
    // orig: root + dir1 + dir1/shared.txt + dir1/only_orig.txt + link + link/shared.txt + link/only_orig.txt + link resolved = 8
    // backup: root + dir2 + dir2/shared.txt + dir2/only_backup.txt + link + link/shared.txt + link/only_backup.txt + link resolved = 8
    original_processed: 8,
    backup_processed: 8,
    // Missing: dir1(1) + dir1/shared.txt(1) + dir1/only_orig.txt(1) + link/only_orig.txt(1) = 4
    missing: 4,
    different: 1,
    // Extras: dir2(1) + dir2/shared.txt(1) + dir2/only_backup.txt(1) + link/only_backup.txt(1) = 4
    extras: 4,
    special_files: 0,
    // Similarities: root + link + link/shared.txt = 3
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// --follow with differences inside symlinked directory
case!(symlink_follow_finds_differences {
    orig: [
        Dir("real_dir"),
        File("real_dir/ok.txt", "ok\n"),
        File("real_dir/only_orig.txt", "only\n"),
        Sym("link_dir", "real_dir"),
    ],
    backup: [
        Dir("real_dir"),
        File("real_dir/ok.txt", "ok\n"),
        Sym("link_dir", "real_dir"),
    ],
    flags: ["--follow", "-vv"],
    lines: [
        "MISSING-FILE: a/real_dir/only_orig.txt",
        "MISSING-FILE: a/link_dir/only_orig.txt",
    ],
    // orig: root + real_dir + real_dir/ok.txt + real_dir/only_orig.txt + link_dir + link_dir resolved + link_dir/ok.txt + link_dir/only_orig.txt = 8
    // backup: root + real_dir + real_dir/ok.txt + link_dir + link_dir resolved + link_dir/ok.txt = 6
    original_processed: 8,
    backup_processed: 6,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 6,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Symlink status mismatch with directories
// ===========================================================================

// Original has real directory, backup has symlink
case!(symlink_status_mismatch_orig_dir {
    orig: [
        Dir("entry"),
        File("entry/child.txt", "content\n"),
    ],
    backup: [
        File("target.txt", "target\n"),
        Sym("entry", "target.txt"),
    ],
    flags: ["-vv"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "MISSING-FILE: a/entry/child.txt",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target.txt",
    ],
    original_processed: 3,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Original has symlink, backup has real directory
case!(symlink_status_mismatch_backup_dir {
    orig: [
        File("target.txt", "target\n"),
        Sym("entry", "target.txt"),
    ],
    backup: [
        Dir("entry"),
        File("entry/child.txt", "content\n"),
    ],
    flags: ["-vv"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target.txt",
        "EXTRA-DIR: b/entry",
        "EXTRA-FILE: b/entry/child.txt",
    ],
    original_processed: 3,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Dangling symlinks
// ===========================================================================

// Both sides have dangling symlinks to the same nonexistent target
case!(dangling_symlinks_same_target {
    orig: [
        File("good.txt", "data\n"),
        Sym("dangling", "nonexistent_target"),
    ],
    backup: [
        File("good.txt", "data\n"),
        Sym("dangling", "nonexistent_target"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/dangling",
    ],
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 1,
    errors: 0,
});

// Dangling symlinks with different targets → DIFFERENT-SYMLINK-TARGET
case!(dangling_symlinks_different_targets {
    orig: [
        Sym("dangling", "target_a"),
    ],
    backup: [
        Sym("dangling", "target_b"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/dangling",
        "SYMLINK-SKIPPED: a/dangling",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

// Dangling symlinks with --follow: both dangling → errors reported
case!(dangling_symlinks_same_target_with_follow {
    orig: [
        Sym("dangling", "nonexistent_target"),
    ],
    backup: [
        Sym("dangling", "nonexistent_target"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: a/dangling",
        "DANGLING-SYMLINK: b/dangling",
    ],
    // With --follow, attempt to resolve → error for each
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 2,
});

// Orig dangling, backup resolves to file
case!(dangling_orig_resolving_backup_file_with_follow {
    orig: [
        Sym("link", "nonexistent"),
    ],
    backup: [
        File("target.txt", "content\n"),
        Sym("link", "target.txt"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link",
        "DANGLING-SYMLINK: a/link",
        "EXTRA-SYMLINK: b/link",
        "EXTRA-FILE: b/target.txt",
    ],
    // orig: root + link + (dangling resolution attempt) = 3
    // backup: root + target.txt + link + link-symlink + link-resolved = 5
    original_processed: 3,
    backup_processed: 5,
    missing: 0,
    different: 1,
    // EXTRA-SYMLINK (link) + EXTRA-FILE (link resolved, silent) + EXTRA-FILE (target.txt) = 3
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// Orig resolves to dir, backup dangling
case!(dangling_backup_resolving_orig_dir_with_follow {
    orig: [
        Dir("real_dir"),
        File("real_dir/child.txt", "content\n"),
        Sym("link", "real_dir"),
    ],
    backup: [
        Sym("link", "nonexistent"),
    ],
    flags: ["--follow", "-vv"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/link",
        "DANGLING-SYMLINK: b/link",
        "MISSING-DIR: a/real_dir",
        "MISSING-FILE: a/real_dir/child.txt",
        "MISSING-SYMLINK: a/link",
        "MISSING-DIR: a/link",
        "MISSING-FILE: a/link/child.txt",
    ],
    // orig: root + real_dir + real_dir/child.txt + link + link-symlink + link-resolved (as dir) + link/child.txt = 7
    // backup: root + link + (dangling resolution attempt) = 3
    original_processed: 7,
    backup_processed: 3,
    // MISSING-SYMLINK (link) + MISSING-DIR (link resolved) + MISSING-FILE (link/child.txt) + MISSING-DIR (real_dir) + MISSING-FILE (real_dir/child.txt) = 5
    missing: 5,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// ===========================================================================
// File symlinks with --follow
// ===========================================================================

// Both symlinks point to same target but content differs
// With --follow, resolved content is compared
case!(file_symlink_with_follow_compares_content {
    orig: [
        File("target.txt", "original content\n"),
        Sym("link", "target.txt"),
    ],
    backup: [
        File("target.txt", "different backup content\n"),
        Sym("link", "target.txt"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/target.txt",
        "DIFFERENT-FILE [SIZE]: a/link",
    ],
    // With --follow: link resolved to file, content compared
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

// Without --follow, matching symlinks are skipped (content not verified)
case!(file_symlink_without_follow_reports_skip {
    orig: [
        File("target.txt", "original content\n"),
        Sym("link", "target.txt"),
    ],
    backup: [
        File("target.txt", "different backup content\n"),
        Sym("link", "target.txt"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/target.txt",
        "SYMLINK-SKIPPED: a/link",
    ],
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

// ===========================================================================
// Cross-type symlink resolution
// ===========================================================================

// Both symlinks, but one resolves to dir and other to file (different targets)
case!(symlinks_one_resolves_to_dir_other_to_file {
    orig: [
        Dir("target_dir"),
        Sym("entry", "target_dir"),
    ],
    backup: [
        File("target_file.txt", "content\n"),
        Sym("entry", "target_file.txt"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-SYMLINK: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "MISSING-DIR: a/target_dir",
        "EXTRA-FILE: b/target_file.txt",
    ],
    // orig: root + target_dir + entry + entry-symlink + entry-resolved (as dir) = 5
    // backup: root + target_file.txt + entry + entry-symlink + entry-resolved (as file) = 5
    original_processed: 5,
    backup_processed: 5,
    // MISSING-SYMLINK (entry) + MISSING-DIR (entry resolved, silent) + MISSING-DIR (target_dir) = 3
    missing: 3,
    different: 2,
    // EXTRA-SYMLINK (entry) + EXTRA-FILE (entry resolved, silent) + EXTRA-FILE (target_file.txt) = 3
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// Same symlink target but resolves to dir on orig, file on backup
// Without --follow: targets match, so symlink is similarity
case!(symlink_same_target_dir_vs_file_no_follow {
    orig: [
        Dir("target"),
        File("target/inside.txt", "content\n"),
        Sym("link", "target"),
    ],
    backup: [
        File("target", "I'm a file\n"),
        Sym("link", "target"),
    ],
    flags: [],
    lines: [
        "SYMLINK-SKIPPED: a/link",
        "FILE-DIR-MISMATCH: a/target",
        "MISSING-DIR: a/target",
        "EXTRA-FILE: b/target",
    ],
    // Without --follow, symlinks match (same target "target")
    // But target itself differs (dir vs file)
    original_processed: 4,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

// Same target with --follow: orig resolves to dir, backup to file
case!(symlink_same_target_orig_dir_backup_file_follow {
    orig: [
        Dir("target"),
        File("target/inside.txt", "content\n"),
        Sym("link", "target"),
    ],
    backup: [
        File("target", "I'm a file\n"),
        Sym("link", "target"),
    ],
    flags: ["--follow", "-vv"],
    lines: [
        "FILE-DIR-MISMATCH: a/link",
        "MISSING-SYMLINK: a/link",
        "MISSING-DIR: a/link",
        "MISSING-FILE: a/link/inside.txt",
        "EXTRA-SYMLINK: b/link",
        "EXTRA-FILE: b/link",
        "FILE-DIR-MISMATCH: a/target",
        "MISSING-DIR: a/target",
        "MISSING-FILE: a/target/inside.txt",
        "EXTRA-FILE: b/target",
    ],
    // orig: root + target + target/inside.txt + link + link-symlink + link-resolved (as dir) + link/inside.txt = 7
    // backup: root + target + link + link-symlink + link-resolved (as file) = 5
    original_processed: 7,
    backup_processed: 5,
    // MISSING-SYMLINK (link) + MISSING-DIR (link resolved) + MISSING-FILE (link/inside.txt) + MISSING-DIR (target) + MISSING-FILE (target/inside.txt) = 5
    missing: 5,
    different: 2,
    // EXTRA-SYMLINK (link) + EXTRA-FILE (link resolved) + EXTRA-FILE (target) = 3
    extras: 3,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// Same target with --follow: orig resolves to file, backup to dir
case!(symlink_same_target_orig_file_backup_dir_follow {
    orig: [
        File("target", "I'm a file\n"),
        Sym("link", "target"),
    ],
    backup: [
        Dir("target"),
        File("target/inside.txt", "content\n"),
        Sym("link", "target"),
    ],
    flags: ["--follow", "-vv"],
    lines: [
        "FILE-DIR-MISMATCH: a/link",
        "MISSING-SYMLINK: a/link",
        "MISSING-FILE: a/link",
        "EXTRA-SYMLINK: b/link",
        "EXTRA-DIR: b/link",
        "EXTRA-FILE: b/link/inside.txt",
        "FILE-DIR-MISMATCH: a/target",
        "MISSING-FILE: a/target",
        "EXTRA-DIR: b/target",
        "EXTRA-FILE: b/target/inside.txt",
    ],
    // orig: root + target + link + link-symlink + link-resolved (as file) = 5
    // backup: root + target + target/inside.txt + link + link-symlink + link-resolved (as dir) + link/inside.txt = 7
    original_processed: 5,
    backup_processed: 7,
    // MISSING-SYMLINK (link) + MISSING-FILE (link resolved) + MISSING-FILE (target) = 3
    missing: 3,
    different: 2,
    // EXTRA-SYMLINK (link) + EXTRA-DIR (link resolved) + EXTRA-FILE (link/inside.txt) + EXTRA-DIR (target) + EXTRA-FILE (target/inside.txt) = 5
    extras: 5,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Missing symlink to directory
// ===========================================================================

// Symlink pointing to directory should be MISSING-SYMLINK, not MISSING-DIR
case!(missing_symlink_to_dir {
    orig: [
        Dir("real_dir"),
        File("real_dir/inside.txt", "content\n"),
        Sym("link_to_dir", "real_dir"),
    ],
    backup: [
        Dir("real_dir"),
        File("real_dir/inside.txt", "content\n"),
    ],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/link_to_dir",
    ],
    original_processed: 4,
    backup_processed: 3,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// ===========================================================================
// Complex symlink scenarios (combined from symlink_mismatch testdata)
// ===========================================================================

// Multiple symlink scenarios in one test (mirrors symlink_mismatch testdata)
// orig: missing_link -> real_a.txt, ok.txt, real_a.txt, real_b.txt, target_diff -> real_a.txt, type_mis -> real_a.txt
// backup: ok.txt, real_a.txt, real_b.txt, target_diff -> real_b.txt, type_mis (regular file)
case!(symlink_mismatch_summary {
    orig: [
        File("ok.txt", "ok\n"),
        File("real_a.txt", "aaa\n"),
        File("real_b.txt", "bbb\n"),
        Sym("missing_link", "real_a.txt"),
        Sym("target_diff", "real_a.txt"),
        Sym("type_mis", "real_a.txt"),
    ],
    backup: [
        File("ok.txt", "ok\n"),
        File("real_a.txt", "aaa\n"),
        File("real_b.txt", "bbb\n"),
        Sym("target_diff", "real_b.txt"),
        File("type_mis", "regular\n"),
    ],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/missing_link",
        "DIFFERENT-SYMLINK-TARGET: a/target_diff",
        "SYMLINK-SKIPPED: a/target_diff",
        "DIFFERENT-SYMLINK-STATUS: a/type_mis",
        "MISSING-SYMLINK: a/type_mis",
        "EXTRA-FILE: b/type_mis",
    ],
    // orig: root + ok.txt + real_a.txt + real_b.txt + missing_link + target_diff + type_mis = 7
    // backup: root + ok.txt + real_a.txt + real_b.txt + target_diff + type_mis = 6
    original_processed: 7,
    backup_processed: 6,
    // Missing: missing_link(1) + type_mis(1) = 2
    missing: 2,
    // Different: target_diff(1) + type_mis(1) = 2
    different: 2,
    // Extras: type_mis file(1) = 1
    extras: 1,
    special_files: 0,
    // Similarities: root + ok.txt + real_a.txt + real_b.txt = 4
    similarities: 4,
    // Skipped: target_diff = 1
    skipped: 1,
    errors: 0,
});
