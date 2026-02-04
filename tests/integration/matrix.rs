//! Comprehensive type-comparison matrix tests.
//!
//! Each `case!` invocation generates a test that:
//!   1. Creates filesystem entries in temp orig/backup dirs
//!   2. Runs `vfy` and checks output lines + summary counts
//!   3. Automatically runs the reversed direction (orig↔backup swapped)
//!
//! Entry types: File, Dir, Sym (symlink), Fifo (named pipe).
//! Entries are composable: `Sym("entry","target"), File("target","x")`
//! creates a symlink with its target file.

use assert_cmd::Command;
use std::path::Path;
use std::process::Command as StdCommand;

#[allow(unused_imports)]
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Entry DSL
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Entry {
    File(&'static str, &'static str), // (name, content)
    Dir(&'static str),                 // (name) — empty directory
    Sym(&'static str, &'static str),  // (name, target) — symlink only
    Fifo(&'static str),               // (name) — named pipe
}

use Entry::*;

// ---------------------------------------------------------------------------
// Expected counts
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Counts {
    original_processed: u64,
    backup_processed: u64,
    missing: u64,
    different: u64,
    extras: u64,
    special_files: u64,
    similarities: u64,
    skipped: u64,
    errors: u64,
}

// ---------------------------------------------------------------------------
// case! macro
// ---------------------------------------------------------------------------

macro_rules! case {
    ($name:ident {
        orig: [ $($orig:expr),* $(,)? ],
        backup: [ $($backup:expr),* $(,)? ],
        flags: [ $($flag:expr),* $(,)? ],
        lines: [ $($line:expr),* $(,)? ],
        original_processed: $op:expr,
        backup_processed: $bp:expr,
        missing: $mis:expr,
        different: $diff:expr,
        extras: $ext:expr,
        special_files: $nfd:expr,
        similarities: $sim:expr,
        skipped: $skip:expr,
        errors: $err:expr $(,)?
    }) => {
        #[test]
        fn $name() {
            check(
                stringify!($name),
                &[$($orig),*],
                &[$($backup),*],
                &[$($flag),*],
                &[$($line),*],
                Counts {
                    original_processed: $op,
                    backup_processed: $bp,
                    missing: $mis,
                    different: $diff,
                    extras: $ext,
                    special_files: $nfd,
                    similarities: $sim,
                    skipped: $skip,
                    errors: $err,
                },
            );
        }
    };
}

// ---------------------------------------------------------------------------
// Test infrastructure
// ---------------------------------------------------------------------------

fn create_entries(dir: &Path, entries: &[Entry]) {
    for entry in entries {
        match entry {
            Entry::File(name, content) => {
                let path = dir.join(name);
                if let Some(parent) = path.parent() {
                    if parent != dir {
                        std::fs::create_dir_all(parent).unwrap();
                    }
                }
                std::fs::write(&path, content).unwrap();
            }
            Entry::Dir(name) => {
                std::fs::create_dir_all(dir.join(name)).unwrap();
            }
            Entry::Sym(name, target) => {
                std::os::unix::fs::symlink(target, dir.join(name)).unwrap();
            }
            Entry::Fifo(name) => {
                let path = dir.join(name);
                StdCommand::new("mkfifo")
                    .arg(&path)
                    .status()
                    .expect("mkfifo failed");
            }
        }
    }
}

/// Transform an expected line for the reversed (orig↔backup swapped) test.
///
/// Rules:
///   - MISSING-X ↔ EXTRA-X  (prefix swap + path a/↔b/)
///   - DANGLING-SYMLINK:     (path a/↔b/ only)
///   - Everything else:      unchanged (DIFFERENT-*, SPECIAL-FILE, SYMLINK
///                           always report the orig path, which is always a/)
fn reverse_expected_line(line: &str) -> String {
    // Split "PREFIX: path" at the first ": "
    let (prefix, path) = match line.split_once(": ") {
        Some((p, r)) => (p, r),
        None => return line.to_string(),
    };

    let (new_prefix, swap_path) = match prefix {
        "MISSING-FILE"    => ("EXTRA-FILE",       true),
        "MISSING-DIR"     => ("EXTRA-DIR",        true),
        "MISSING-SYMLINK" => ("EXTRA-SYMLINK",    true),
        "EXTRA-FILE"      => ("MISSING-FILE",     true),
        "EXTRA-DIR"       => ("MISSING-DIR",      true),
        "EXTRA-SYMLINK"   => ("MISSING-SYMLINK",  true),
        "DANGLING-SYMLINK"  => ("DANGLING-SYMLINK",  true),
        "SPECIAL-FILE" => ("SPECIAL-FILE", true),
        _                   => (prefix,              false),
    };

    let new_path = if swap_path {
        if path.starts_with("a/") {
            format!("b/{}", &path[2..])
        } else if path.starts_with("b/") {
            format!("a/{}", &path[2..])
        } else {
            panic!(
                "Expected line path must start with a/ or b/, got: {:?}",
                line
            );
        }
    } else {
        path.to_string()
    };

    format!("{}: {}", new_prefix, new_path)
}

/// Return true if this output line is a "diagnostic" line (not summary/CMD/DEBUG).
fn is_diagnostic_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    // Summary block: "SUMMARY:" header and indented lines
    if line.starts_with("SUMMARY:") || line.starts_with("    ") {
        return false;
    }
    // CMD line at the top
    if line.starts_with("CMD:") {
        return false;
    }
    // DEBUG lines (verbose output)
    if line.starts_with("DEBUG:") {
        return false;
    }
    true
}

fn run_and_check(
    label: &str,
    tmp: &Path,
    orig_entries: &[Entry],
    backup_entries: &[Entry],
    flags: &[&str],
    expected_lines: &[&str],
    counts: &Counts,
) {
    let a = tmp.join("a");
    let b = tmp.join("b");

    // Clean and recreate
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    create_entries(&a, orig_entries);
    create_entries(&b, backup_entries);

    let a_str = a.to_str().unwrap();
    let b_str = b.to_str().unwrap();

    let mut args: Vec<&str> = vec![a_str, b_str];
    args.extend_from_slice(flags);

    let output = Command::cargo_bin("vfy")
        .unwrap()
        .args(&args)
        .output()
        .expect("failed to run vfy");

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Collect diagnostic lines
    let mut diag_lines: Vec<&str> = stdout.lines().filter(|l| is_diagnostic_line(l)).collect();

    // Match expected lines (each must match exactly one diagnostic line).
    // Expected format: "PREFIX: relative/path" — checks that the output line
    // starts with PREFIX and contains the relative path.
    let mut unmatched_expected: Vec<&str> = Vec::new();
    for expected in expected_lines {
        let matches = |line: &&str| -> bool {
            if let Some((prefix, path)) = expected.split_once(": ") {
                line.starts_with(prefix) && line.contains(path)
            } else {
                line.contains(expected)
            }
        };
        if let Some(pos) = diag_lines.iter().position(matches) {
            diag_lines.remove(pos);
        } else {
            unmatched_expected.push(expected);
        }
    }

    if !unmatched_expected.is_empty() || !diag_lines.is_empty() {
        panic!(
            "[{}] Line mismatch.\n\
             Expected lines not found: {:?}\n\
             Unexpected lines in output: {:?}\n\
             Full output:\n{}",
            label, unmatched_expected, diag_lines, stdout
        );
    }

    // Check summary counts
    let expect_exit_0 = counts.missing == 0
        && counts.different == 0
        && counts.extras == 0
        && counts.special_files == 0
        && counts.errors == 0;

    let checks = [
        format!("Original items processed: {}", counts.original_processed),
        format!("Backup items processed: {}", counts.backup_processed),
        format!("Missing: {}", counts.missing),
        format!("Different: {}", counts.different),
        format!("Extras: {}", counts.extras),
        format!("Special files: {}", counts.special_files),
        format!("Similarities: {}", counts.similarities),
        format!("Skipped: {}", counts.skipped),
        format!("Errors: {}", counts.errors),
    ];

    for check_str in &checks {
        if !stdout.contains(check_str) {
            panic!(
                "[{}] Summary mismatch: expected {:?} in output.\nFull output:\n{}",
                label, check_str, stdout
            );
        }
    }

    // Check exit code
    let actual_exit = output.status.code().unwrap_or(-1);
    let expected_exit = if expect_exit_0 { 0 } else { 1 };
    if actual_exit != expected_exit {
        panic!(
            "[{}] Exit code mismatch: expected {}, got {}.\nFull output:\n{}",
            label, expected_exit, actual_exit, stdout
        );
    }
}

fn check(
    name: &str,
    orig_entries: &[Entry],
    backup_entries: &[Entry],
    flags: &[&str],
    expected_lines: &[&str],
    counts: Counts,
) {
    let tmp = std::env::temp_dir().join(format!("bv_matrix_{}", name));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // Forward direction
    run_and_check(
        &format!("{} (forward)", name),
        &tmp,
        orig_entries,
        backup_entries,
        flags,
        expected_lines,
        &counts,
    );

    // Reversed direction: swap orig↔backup, swap missing↔extras,
    // swap original_processed↔backup_processed, transform MISSING↔EXTRA in lines
    let reversed_lines: Vec<String> = expected_lines
        .iter()
        .map(|l| reverse_expected_line(l))
        .collect();
    let reversed_line_refs: Vec<&str> = reversed_lines.iter().map(|s| s.as_str()).collect();

    let reversed_counts = Counts {
        original_processed: counts.backup_processed,
        backup_processed: counts.original_processed,
        missing: counts.extras,
        different: counts.different,
        extras: counts.missing,
        special_files: counts.special_files,
        similarities: counts.similarities,
        skipped: counts.skipped,
        errors: counts.errors,
    };

    run_and_check(
        &format!("{} (reversed)", name),
        &tmp,
        backup_entries,
        orig_entries,
        flags,
        &reversed_line_refs,
        &reversed_counts,
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ===========================================================================
// Test cases
// ===========================================================================

// === Orig = File ===

case!(file_x_file_same {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        File("entry", "contents"),
    ],
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

case!(file_x_file_diff {
    orig: [
        File("entry", "original-contents"),
    ],
    backup: [
        File("entry", "different-contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/entry",
    ],
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

case!(file_x_dir {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    flags: [],
    lines: [
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
    ],
    // Both sides reported: MISSING-FILE for orig file, EXTRA-DIR for backup dir
    // child counted by report but only printed at -vv
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_dir_vv {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    flags: ["-vv"],
    lines: [
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
        "EXTRA-FILE: b/entry/child",
    ],
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_fifo {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-FILE: a/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_absent {
    orig: [
        File("entry", "contents"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-FILE: a/entry",
    ],
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

// === Orig = Dir ===

case!(dir_x_dir {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    flags: [],
    lines: [],
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

case!(dir_x_fifo {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-DIR: a/entry",
    ],
    // Backup is special → SPECIAL-FILE for the fifo.
    // Orig is a dir → MISSING-DIR + count_recursive counts the child as missing too.
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_absent {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-DIR: a/entry",
    ],
    original_processed: 3,
    backup_processed: 1,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// === Orig = Fifo ===

case!(fifo_x_fifo {
    orig: [
        Fifo("entry"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: a/entry",
        "SPECIAL-FILE: b/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 2,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(fifo_x_absent {
    orig: [
        Fifo("entry"),
    ],
    backup: [],
    flags: [],
    lines: [
        "SPECIAL-FILE: a/entry",
    ],
    original_processed: 2,
    backup_processed: 1,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// === One-side symlink: File x Sym (no --follow) ===

case!(file_x_symlink_to_file {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_to_dir {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_dangling {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// === One-side symlink: File x Sym (--follow) ===
// Currently --follow has no effect on one-side-symlink mismatch.
// Same expected results as without --follow for now.

case!(file_x_symlink_to_file_follow {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved file counted as item (+1 backup, +1 extras)
    original_processed: 2,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_to_dir_follow {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved dir + child counted as items
    // "target" dir + child only in backup → EXTRA-DIR + child
    original_processed: 2,
    backup_processed: 6,
    missing: 1,
    different: 1,
    extras: 5,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(file_x_symlink_dangling_follow {
    orig: [
        File("entry", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → dangling → error
    original_processed: 2,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === One-side symlink: Dir x Sym (no --follow) ===

case!(dir_x_symlink_to_file {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    // orig is a real dir → report counts dir + child as missing
    // symlink side also reported as EXTRA-SYMLINK
    // "target" only in backup → EXTRA-FILE
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

case!(dir_x_symlink_to_file_follow {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-FILE: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved file counted (+1 backup, +1 extras)
    // orig dir → report: dir(1) + child(1) = 2 missing
    original_processed: 3,
    backup_processed: 4,
    missing: 2,
    different: 1,
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_to_dir {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child2", "data2"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    // orig dir → report: dir(1) + child(1) = 2 missing
    // symlink side also reported as EXTRA-SYMLINK
    // "target" dir + child2 only in backup → EXTRA-DIR(1) + child2(1) = 2+1 extras
    original_processed: 3,
    backup_processed: 4,
    missing: 2,
    different: 1,
    extras: 3,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_to_dir_follow {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
        File("target/child2", "data2"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "EXTRA-DIR: b/target",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows symlink → resolved dir(1) + child(1) + child2(1) = 3 more extras
    // "target" dir + children also in backup = EXTRA-DIR + child + child2
    original_processed: 3,
    backup_processed: 8,
    missing: 2,
    different: 1,
    extras: 7,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_dangling {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
    ],
    // orig dir → report: dir(1) + child(1) = 2 missing
    // symlink side also reported as EXTRA-SYMLINK
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(dir_x_symlink_dangling_follow {
    orig: [
        Dir("entry"),
        File("entry/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-STATUS: a/entry",
        "MISSING-DIR: a/entry",
        "EXTRA-SYMLINK: b/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Cat1: symlink side reported as EXTRA-SYMLINK
    // Cat3: report follows dangling symlink → error
    original_processed: 3,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === One-side symlink: Sym x Fifo ===

case!(symlink_to_file_x_fifo {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    // "target": only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_dir_x_fifo {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    // "target": only in orig → MISSING-DIR
    original_processed: 3,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_dangling_x_fifo {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: [],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
    ],
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    original_processed: 2,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// --follow doesn't change Sym x Fifo (special takes priority)

case!(symlink_to_file_x_fifo_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: ["--follow"],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Cat3: report follows symlink → resolved file counted as separate item (+1 orig, +1 missing)
    // "target" only in orig → MISSING-FILE
    original_processed: 4,
    backup_processed: 2,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_dir_x_fifo_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: ["--follow"],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Cat3: report follows symlink → resolved dir counted as separate item (+1 orig, +1 missing)
    // "entry": backup FIFO → SPECIAL-FILE, orig symlink → MISSING-SYMLINK
    // "target": only in orig → MISSING-DIR
    original_processed: 4,
    backup_processed: 2,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_dangling_x_fifo_follow {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Fifo("entry"),
    ],
    flags: ["--follow"],
    lines: [
        "SPECIAL-FILE: b/entry",
        "MISSING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: a/entry",
    ],
    // Cat3: report follows symlink → dangling counted as separate item (+1 orig)
    // "entry": backup FIFO → SPECIAL-FILE, orig dangling symlink → MISSING-SYMLINK + DANGLING-SYMLINK (error)
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 1,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === One-side symlink: Sym x Absent ===

case!(symlink_to_file_x_absent {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // "entry" is a symlink only in orig → MISSING-SYMLINK
    // "target" is a file only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 1,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_file_x_absent_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Cat3: report follows symlink → resolved file counted as separate item (+1 orig, +1 missing)
    // Resolved MISSING-FILE not printed at default verbosity
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_to_dir_x_absent {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // "target/child" counted in missing but not printed without -vv
    original_processed: 4,
    backup_processed: 1,
    missing: 3,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// With --follow, the symlink resolves to a dir, so its contents are also counted as missing.
case!(symlink_to_dir_x_absent_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Cat3: report follows symlink → resolved dir+child counted as separate items
    // entry + entry/child + target + target/child = 5 missing
    // entry/child and target/child not printed without -vv
    original_processed: 6,
    backup_processed: 1,
    missing: 5,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symlink_dangling_x_absent {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [],
    flags: [],
    lines: [
        "MISSING-SYMLINK: a/entry",
    ],
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

// With --follow, the dangling symlink is reported as an error.
case!(symlink_dangling_x_absent_follow {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [],
    flags: ["--follow"],
    lines: [
        "MISSING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: a/entry",
    ],
    // Cat3: report follows symlink → dangling counted as separate item (+1 orig)
    original_processed: 3,
    backup_processed: 1,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === Both symlinks: same type, same target ===

case!(symfile_x_symfile_same {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: [],
    lines: [
        "SYMLINK: a/entry",
    ],
    // Both sides have identical symlinks → SYMLINK (skipped)
    // "target" files match → similarity
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

case!(symfile_x_symfile_same_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    flags: ["--follow"],
    lines: [],
    // Cat2: resolved file counted as separate item (+1 orig, +1 backup)
    // Cat4: same-target symlink pair counted as similarity (+1 sim)
    // entry resolves to file with same contents → similarity
    // "target" files also match → similarity
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 0,
    errors: 0,
});

case!(symdir_x_symdir_same {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: [],
    lines: [
        "SYMLINK: a/entry",
    ],
    // Both sides have identical symlinks → SYMLINK (skipped)
    // "target" dirs match, "target/child" files match → similarities
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

case!(symdir_x_symdir_same_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: ["--follow"],
    lines: [],
    // Cat2: resolved dir+child counted as separate items (+1 orig, +1 backup each)
    // With --follow, entry resolves to target dir → compare_recursive
    // entry/child discovered via followed symlink
    // Cat4: same-target symlink pair counted as similarity
    // target dir also walked independently → target/child discovered
    // All match → similarities
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

case!(symdangling_x_symdangling_same {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "SYMLINK: a/entry",
    ],
    // Both symlinks point to same target → SYMLINK (skipped)
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symdangling_x_symdangling_same_follow {
    orig: [
        Sym("entry", "nonexistent"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Cat2: resolved (dangling) content counted as separate items (+1 orig, +1 backup)
    // Both dangling with same target — symlinks match, both dangling = 2 errors
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

// === Both symlinks: same type, different target ===

case!(symfile_x_symfile_diff {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "target-b"),
        File("target-b", "contents"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK: a/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-FILE: b/target-b",
    ],
    // "entry" symlinks differ in target → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" only in orig → MISSING-FILE
    // "target-b" only in backup → EXTRA-FILE
    original_processed: 3,
    backup_processed: 3,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symfile_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "same-content"),
    ],
    backup: [
        Sym("entry", "target-b"),
        File("target-b", "same-content"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-FILE: b/target-b",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved files counted as separate items (+1 orig, +1 backup)
    // With --follow, resolved files have same content → similarity for entry
    // "target-a" only in orig → MISSING-FILE
    // "target-b" only in backup → EXTRA-FILE
    original_processed: 4,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 1,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

case!(symdir_x_symdir_diff {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK: a/entry",
        "MISSING-DIR: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // "entry" symlinks differ in target → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" dir only in orig → MISSING-DIR (+ child counted)
    // "target-b" dir only in backup → EXTRA-DIR (+ child counted)
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

case!(symdir_x_symdir_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
        File("target-a/child2", "original"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
        File("target-b/child2", "different"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DIFFERENT-FILE [SIZE]: a/entry/child2",
        "MISSING-DIR: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved dir+children counted as separate items (+1 orig, +1 backup each)
    // With --follow, entry resolves to dir → compare_recursive
    //   entry/child matches → similarity
    //   entry/child2 differs → different
    // "target-a" only in orig → MISSING-DIR (+ 2 children counted)
    // "target-b" only in backup → EXTRA-DIR (+ 2 children counted)
    original_processed: 8,
    backup_processed: 8,
    missing: 3,
    different: 2,
    extras: 3,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

case!(symdangling_x_symdangling_diff {
    orig: [
        Sym("entry", "nonexistent-a"),
    ],
    backup: [
        Sym("entry", "nonexistent-b"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK: a/entry",
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

case!(symdangling_x_symdangling_diff_follow {
    orig: [
        Sym("entry", "nonexistent-a"),
    ],
    backup: [
        Sym("entry", "nonexistent-b"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DANGLING-SYMLINK: a/entry",
        "DANGLING-SYMLINK: b/entry",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET, both dangling = 2 errors
    // Cat2: resolved (dangling) content counted as separate items (+1 orig, +1 backup)
    original_processed: 3,
    backup_processed: 3,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 2,
});

// === Both symlinks: cross-type, different targets ===

case!(symfile_x_symdir_diff {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK: a/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // "entry" symlinks differ → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" file only in orig → MISSING-FILE
    // "target-b" dir only in backup → EXTRA-DIR (+ child counted)
    original_processed: 3,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdir_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "target-b"),
        Dir("target-b"),
        File("target-b/child", "data"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
        "MISSING-FILE: a/target-a",
        "EXTRA-DIR: b/target-b",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved content counted as separate items
    // Cat1: both sides reported → MISSING-FILE + EXTRA-DIR
    // With --follow, entry resolves to file vs dir → FILE-DIR-MISMATCH + MISSING-FILE + EXTRA-DIR (+ child)
    // "target-a" only in orig → MISSING-FILE
    // "target-b" dir only in backup → EXTRA-DIR (+ child counted)
    original_processed: 4,
    backup_processed: 6,
    missing: 2,
    different: 2,
    extras: 4,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

case!(symfile_x_symdangling_diff {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK: a/entry",
        "MISSING-FILE: a/target-a",
    ],
    // "entry" symlinks differ → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdangling_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        File("target-a", "contents"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DANGLING-SYMLINK: b/entry",
        "MISSING-FILE: a/entry",
        "MISSING-FILE: a/target-a",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved content counted as separate items
    // Backup dangling → DANGLING-SYMLINK (error), inc_backup_items
    // Orig entry resolves to file → report(entry, Missing): MISSING-FILE
    // "target-a" only in orig → MISSING-FILE
    original_processed: 4,
    backup_processed: 3,
    missing: 2,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

case!(symdir_x_symdangling_diff {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "SYMLINK: a/entry",
        "MISSING-DIR: a/target-a",
    ],
    // "entry" symlinks differ → DIFFERENT-SYMLINK-TARGET + SYMLINK (skipped)
    // "target-a" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 4,
    backup_processed: 2,
    missing: 2,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 1,
    errors: 0,
});

case!(symdir_x_symdangling_diff_follow {
    orig: [
        Sym("entry", "target-a"),
        Dir("target-a"),
        File("target-a/child", "data"),
    ],
    backup: [
        Sym("entry", "nonexistent"),
    ],
    flags: ["--follow"],
    lines: [
        "DIFFERENT-SYMLINK-TARGET: a/entry",
        "DANGLING-SYMLINK: b/entry",
        "MISSING-DIR: a/entry",
        "MISSING-DIR: a/target-a",
    ],
    // Targets differ → DIFFERENT-SYMLINK-TARGET
    // Cat2: resolved content counted as separate items
    // Backup dangling → DANGLING-SYMLINK (error), inc_backup_items
    // Orig resolves to dir → report(entry, Missing): dir(1) + child(1) counted as missing
    // "target-a" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 6,
    backup_processed: 3,
    missing: 4,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 1,
});

// === Both symlinks: cross-type, same target (different resolution per side) ===

case!(symfile_x_symdir_same_target {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: [],
    lines: [
        "SYMLINK: a/entry",
        "FILE-DIR-MISMATCH: a/target",
        "MISSING-FILE: a/target",
        "EXTRA-DIR: b/target",
    ],
    // Same symlink target → SYMLINK (skipped), similarity
    // "target" is file in orig vs dir in backup → FILE-DIR-MISMATCH
    //   Both sides reported: MISSING-FILE + EXTRA-DIR (+ child counted)
    original_processed: 3,
    backup_processed: 4,
    missing: 1,
    different: 1,
    extras: 2,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdir_same_target_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    flags: ["--follow"],
    lines: [
        "FILE-DIR-MISMATCH: a/entry",
        "MISSING-FILE: a/entry",
        "EXTRA-DIR: b/entry",
        "FILE-DIR-MISMATCH: a/target",
        "MISSING-FILE: a/target",
        "EXTRA-DIR: b/target",
    ],
    // Cat2: resolved content counted as separate items
    // Cat1: both sides reported in type mismatch → MISSING-FILE + EXTRA-DIR
    // With --follow, entry resolves: file vs dir → FILE-DIR-MISMATCH + MISSING-FILE + EXTRA-DIR (+ child)
    // "target" also file vs dir → FILE-DIR-MISMATCH + MISSING-FILE + EXTRA-DIR (+ child)
    // Cat4: same-target symlink pair counted as similarity
    original_processed: 4,
    backup_processed: 6,
    missing: 2,
    different: 2,
    extras: 4,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

case!(symfile_x_symdangling_same_target {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: [],
    lines: [
        "SYMLINK: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Same target → SYMLINK (skipped), similarity
    // "target" file only in orig → MISSING-FILE
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symfile_x_symdangling_same_target_follow {
    orig: [
        Sym("entry", "target"),
        File("target", "contents"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: b/entry",
        "MISSING-FILE: a/entry",
        "MISSING-FILE: a/target",
    ],
    // Cat2: resolved content counted as separate items
    // Cat4: same-target symlink pair counted as similarity
    // Same target → similarity. Backup dangling → DANGLING-SYMLINK (error)
    // Orig entry resolves to file → report(entry, Missing): MISSING-FILE
    // "target" file only in orig → MISSING-FILE
    original_processed: 4,
    backup_processed: 3,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 1,
});

case!(symdir_x_symdangling_same_target {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: [],
    lines: [
        "SYMLINK: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Same target → SYMLINK (skipped), similarity
    // "target" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 4,
    backup_processed: 2,
    missing: 2,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 1,
    errors: 0,
});

case!(symdir_x_symdangling_same_target_follow {
    orig: [
        Sym("entry", "target"),
        Dir("target"),
        File("target/child", "data"),
    ],
    backup: [
        Sym("entry", "target"),
    ],
    flags: ["--follow"],
    lines: [
        "DANGLING-SYMLINK: b/entry",
        "MISSING-DIR: a/entry",
        "MISSING-DIR: a/target",
    ],
    // Cat2: resolved content counted as separate items
    // Cat4: same-target symlink pair counted as similarity
    // Same target → similarity. Backup dangling → DANGLING-SYMLINK (error)
    // Orig entry resolves to dir → report(entry, Missing): dir(1) + child(1) counted as missing
    // "target" dir only in orig → MISSING-DIR (+ child counted)
    original_processed: 6,
    backup_processed: 3,
    missing: 4,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 1,
});
