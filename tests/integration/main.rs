mod basic;
mod edge_cases;
mod errors;
mod flags;
mod harness;
mod matrix;
mod symlinks;

use assert_cmd::Command;

pub fn cmd() -> Command {
    Command::cargo_bin("vfy").unwrap()
}

/// Check that at least one line contains both `prefix` and `needle`.
pub fn some_line_has(output: &str, prefix: &str, needle: &str) -> bool {
    output
        .lines()
        .any(|l| l.contains(prefix) && l.contains(needle))
}

/// Check that no line matching `prefix` also contains `needle`.
#[allow(dead_code)]
pub fn no_line_has(output: &str, prefix: &str, needle: &str) -> bool {
    !some_line_has(output, prefix, needle)
}

/// Extract stdout from an assert_cmd Assert.
pub fn stdout_of(a: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(a.get_output().stdout.clone()).unwrap()
}
