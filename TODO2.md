# TODO: Full Review Findings

## Bugs

- [x] B1. `--ignore` on extra FILES in backup tree is silently ignored
  - Fixed: removed `meta.is_dir()` gate on ignore check in `handle_extra`; ignore now applies to all extra entry types
  - Also fixed: `count_recursive` now checks ignore list per entry, so ignored subdirs inside missing/extra dirs are skipped
  - Tests: `flags::ignore_extra_file_in_backup`, `flags::ignore_subdir_inside_missing_dir`

- [x] B2. Type mismatch messages are misleading for special file types
  - Fixed: added `NOT_A_FILE_OR_DIR:` check before dir/file branching for both non-symlink and symlink `--follow` paths
  - Special files (FIFO, socket, device, etc.) are now detected early and reported as differences
  - Test: `edge_cases::special_file_via_symlink_follow`

- [x] B3. Sample size off-by-one vs Ruby — wontfix
  - Ruby (vfy.rb:135): `length = [aBytes, start + SampleSize].min - start + 1` reads up to SampleSize+1 = 33 bytes
  - Rust (compare.rs:409): `sample_size = 32`, reads exactly 32 bytes
  - Very low probability of impact; 32 bytes is a reasonable sample size

## Missing Test Coverage

- [x] T1. Nonexistent backup directory -> exit 2
  - Test: `errors::nonexistent_backup_exits_2`

- [x] T2. Backup path is a file, not a directory -> exit 2
  - Test: `errors::backup_is_file_not_dir`

- [x] T3. Cannot read symlink targets -> ERROR — wontfix
  - compare.rs handles this with explicit error reporting and error count increment
  - Untestable: read_link fails only via race conditions (symlink deleted or parent permissions changed between readdir and read_link); lchmod not supported on Linux

- [x] T4. Both symlinks, one resolves to dir, other resolves to file
  - Fixed: symlink --follow path now detects resolved type mismatch → DIFFERENT-FILE [TYPE] with recursive counting
  - Also fixed: non-symlink type mismatch now counts dir contents as missing/extras via count_recursive
  - Tests: `symlinks::symlink_same_target_orig_dir_backup_file_follow`, `symlinks::symlink_same_target_orig_file_backup_dir_follow`, `symlinks::symlink_same_target_dir_vs_file_no_follow`, `errors::type_mismatch_dir_orig_counts_missing_contents`, `errors::type_mismatch_dir_backup_counts_extra_contents`

- [x] T5. `--ignore` on extra file in backup tree (related to B1)
  - Test: `flags::ignore_extra_file_in_backup`

## Correctness Concerns

- [x] C1. Root dir counted before ignore check
  - Fixed: moved similarity counting into `compare_recursive` (after ignore check) via `is_root` parameter; root original/backup counting also deferred
  - Test: `flags::ignore_root_directory`

- [x] C2. If both dirs unreadable, only orig error reported — wontfix
  - compare_recursive:34-50: if orig dir read fails, returns immediately; backup dir read failure never checked
  - The item was already counted as similarity (from parent recurse)
  - Ruby (vfy.rb:245-251): only catches EACCES/EINVAL for original dir read — same behavior

- [x] C3. `--ignore` path canonicalization follows symlinks
  - Fixed: ignore path resolution now canonicalizes only the parent directory and appends the final component, preserving the symlink's own name
  - Existence verified via `symlink_metadata` (doesn't follow symlinks)
  - Tests: `flags::ignore_symlink_to_file`, `flags::ignore_symlink_to_dir_with_follow`

- [x] C4. Ignore paths not checked inside count_recursive
  - Fixed: `count_recursive` now checks ignore list per entry; skipped entries get `SKIP:` output and increment skipped counter
  - Test: `flags::ignore_subdir_inside_missing_dir`

- [x] C5. `handle_extra` stat failure: item not counted at all
  - Prints ERROR and increments error count — user sees the failure
  - Not counted as backup_items/extras, but error count is sufficient
  - No test: requires race condition (file deleted between readdir and stat), untestable without fault injection

## Design Observations

- [x] D1. `-f` short flag for `--follow` missing
  - Fixed: added `#[arg(short, long)]` to `follow` in cli.rs

- [x] D2. No `--machine` output format — wontfix
  - Ruby has `-m`/`--machine` for machine-readable summary (vfy.rb:43-45)
  - Not planned for Rust port

- [x] D3. `handle_extra` increment-then-undo pattern
  - Fixed: moved ignore check before counter increments in `handle_extra`; removed unused `dec_extras()` and `dec_backup_items()` from Stats

- [x] D4. `Ordering::Relaxed` is correct for all atomics
  - Single-threaded comparison; only ctrlc handler reads from another thread and only needs a best-effort snapshot
