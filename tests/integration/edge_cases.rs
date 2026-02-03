use super::{cmd, stdout_of, testdata, testdata_base};
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
            predicate::str::contains("Original items processed: 0")
                .and(predicate::str::contains("Backup items processed: 0"))
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 0"))
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
