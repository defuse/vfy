# Type Comparison Matrix

Comprehensive analysis of all possible entry types in original vs backup,
what the code does for each case, and whether we have test coverage.

## Entry types

- **File** — regular file
- **Dir** — real directory
- **Sym→file** — symlink whose target resolves to a regular file
- **Sym→dir** — symlink whose target resolves to a directory
- **Sym→dangling** — symlink whose target doesn't exist
- **Special** — device, FIFO, socket (not a symlink)
- **Absent** — entry doesn't exist on that side

---

## Row: Orig = File

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `compare_file` → `DIFFERENT-FILE [SIZE/SAMPLE/HASH]:` or similarity | `basic::identical`, `basic::different_size`, `basic::different_content_hash`, `basic::different_content_sample` | Yes |
| **Dir** | `DIFFERENT-FILE [TYPE]: (file vs dir)` + `count_recursive(backup, Extra)` | `errors::file_in_original_dir_in_backup` | Yes |
| **Sym→file** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — one is symlink, other isn't | No (reverse tested in `symlink_type_mismatch`) | **Questionable — see TODO #5, #6** |
| **Sym→dir** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` | No | **Questionable — see TODO #5, #6** |
| **Sym→dangling** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — one is symlink, other isn't | No | **Questionable — see TODO #6** |
| **Special** | `NOT_A_FILE_OR_DIR:` | No | Yes — but only the special side is unusual; the file is fine |
| **Absent** | `MISSING-FILE:` | `basic::missing_file` | Yes |

## Row: Orig = Dir

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `DIFFERENT-FILE [TYPE]: (dir vs file)` + `count_recursive(orig, Missing)` | `errors::dir_in_original_file_in_backup` | Yes |
| **Dir** | `compare_recursive` (recurse into both) | `basic::identical`, `basic::nested`, `edge_cases::deep_identical_tree` | Yes |
| **Sym→file** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — no recursive counting of dir contents | No | **Bug — see TODO #5, #6** |
| **Sym→dir** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — no recursive counting of dir contents | No | **Bug — see TODO #5, #6** |
| **Sym→dangling** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — no recursive counting of dir contents | No | **Bug — see TODO #5, #6** |
| **Special** | `NOT_A_FILE_OR_DIR:` | No | Yes |
| **Absent** | `MISSING-DIR:` + `count_recursive` | `basic::nested` (sub3) | Yes |

## Row: Orig = Sym→file

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` | `symlinks::symlink_type_mismatch` | **Questionable — see TODO #6** |
| **Dir** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — no recursive counting of dir contents | No | **Bug — see TODO #5, #6** |
| **Sym→file** (same target) | Without `--follow`: similarity. With `--follow`: `compare_file` on resolved content | `symlinks::matching_symlinks_are_similar`, `symlinks::file_symlink_with_follow_compares_content`, `symlinks::file_symlink_without_follow_checks_target_only` | Yes |
| **Sym→file** (diff target) | `DIFFERENT-SYMLINK-TARGET: (targets differ)` | `symlinks::symlink_target_mismatch` | Yes |
| **Sym→dir** (same target) | Not both dirs. Same target. Without `--follow`: similarity. With `--follow`: `DIFFERENT-FILE [TYPE]: (file vs dir)` + `count_recursive(backup, Extra)` | `symlinks::symlink_same_target_orig_file_backup_dir_follow` | Yes |
| **Sym→dir** (diff target) | `DIFFERENT-SYMLINK-TARGET: (targets differ)` | `symlinks::symlinks_one_resolves_to_dir_other_to_file` | Yes |
| **Sym→dangling** (diff target) | `DIFFERENT-SYMLINK-TARGET: (targets differ)` | No | Yes |
| **Sym→dangling** (same target) | Same target string but orig resolves, backup doesn't (relative path, file exists in orig tree but not backup tree). Without `--follow`: similarity. With `--follow`: `orig_is_file=true`, `backup_is_file=false` → `NOT_A_FILE_OR_DIR:` | No | **Questionable** — with `--follow`, a resolvable-on-one-side symlink gets `NOT_A_FILE_OR_DIR` which is misleading |
| **Special** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — orig is symlink, backup is not | No | **Questionable — see TODO #4, #6** |
| **Absent** | `MISSING-SYMLINK:` | `symlinks::symlink_missing_from_backup` | Yes |

## Row: Orig = Sym→dir

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` | No | **Questionable — see TODO #6** |
| **Dir** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — no recursive counting of dir contents | No | **Bug — see TODO #5, #6** |
| **Sym→file** (same target) | Not both dirs. Same target. Without `--follow`: similarity. With `--follow`: `DIFFERENT-FILE [TYPE]: (dir vs file)` + `count_recursive(orig, Missing)` | `symlinks::symlink_same_target_orig_dir_backup_file_follow`, `symlinks::symlink_same_target_dir_vs_file_no_follow` | Yes |
| **Sym→file** (diff target) | `DIFFERENT-SYMLINK-TARGET: (targets differ)` | `symlinks::symlinks_one_resolves_to_dir_other_to_file` | Yes |
| **Sym→dir** (same target) | Both dirs. Without `--follow`: `SYMLINK:` + skipped. With `--follow`: `compare_recursive` | `symlinks::symlink_dir_no_follow`, `symlinks::symlink_dir_with_follow` | Yes |
| **Sym→dir** (diff target) | Both resolve to dirs → enters `orig_is_dir && backup_is_dir` branch. **Targets are never compared.** With `--follow`: `compare_recursive`. Without `--follow`: `SYMLINK:` + skipped. | No | **Bug — see TODO #1** |
| **Sym→dangling** (diff target) | orig_is_dir=true, backup_is_dir=false. Not both dirs. Compare targets → differ → `DIFFERENT-SYMLINK-TARGET: (targets differ)` | No | Yes |
| **Sym→dangling** (same target) | Same target string but orig resolves to dir, backup doesn't. orig_is_dir=true, backup_is_dir=false. Not both dirs. Same target. Without `--follow`: similarity. With `--follow`: `DIFFERENT-FILE [TYPE]: (dir vs file)` + `count_recursive(orig, Missing)` | No | **Questionable** — without `--follow`, a dir-on-one-side vs dangling-on-other is silently a similarity |
| **Special** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` | No | **Questionable — see TODO #4, #6** |
| **Absent** | `MISSING-SYMLINK:` | `symlinks::missing_symlink_to_dir` | Yes |

## Row: Orig = Sym→dangling

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` | No | **Questionable — see TODO #6** |
| **Dir** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — no recursive counting of dir contents | No | **Bug — see TODO #5, #6** |
| **Sym→file** (diff target) | `DIFFERENT-SYMLINK-TARGET: (targets differ)` | No | Yes |
| **Sym→file** (same target) | Same target string but backup resolves, orig doesn't. Without `--follow`: similarity. With `--follow`: `orig_is_file=false`, `backup_is_file=true` → `NOT_A_FILE_OR_DIR:` | No | **Questionable** — mirrors the Sym→file vs Sym→dangling case |
| **Sym→dir** (diff target) | orig_is_dir=false, backup_is_dir=true. Not both dirs. Compare targets → differ → `DIFFERENT-SYMLINK-TARGET: (targets differ)` | No | Yes |
| **Sym→dir** (same target) | Same target string but backup resolves to dir, orig doesn't. orig_is_dir=false, backup_is_dir=true. Not both dirs. Same target. Without `--follow`: similarity. With `--follow`: `DIFFERENT-FILE [TYPE]: (file vs dir)` + `count_recursive(backup, Extra)` | No | **Questionable** — mirrors the Sym→dir vs Sym→dangling case |
| **Sym→dangling** (same target) | Not both dirs. Same target. Without `--follow`: similarity. With `--follow`: both `is_file=false` → `NOT_A_FILE_OR_DIR:` | `symlinks::dangling_symlinks_same_target` (no `--follow` only) | **Questionable** — with `--follow`, matching dangling symlinks get `NOT_A_FILE_OR_DIR` instead of similarity |
| **Sym→dangling** (diff target) | `DIFFERENT-SYMLINK-TARGET: (targets differ)` | `symlinks::dangling_symlinks_different_targets` | Yes |
| **Special** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` | No | **Questionable — see TODO #4, #6** |
| **Absent** | `MISSING-SYMLINK:` | No (same code path as other symlink missing tests) | Yes |

## Row: Orig = Special

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `NOT_A_FILE_OR_DIR:` | No | Yes |
| **Dir** | `NOT_A_FILE_OR_DIR:` | No | Yes |
| **Sym→file** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — backup is symlink, orig is not | No | **Questionable — see TODO #4, #6** |
| **Sym→dir** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` | No | **Questionable — see TODO #4, #6** |
| **Sym→dangling** | `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` — backup is symlink, orig is not | No | **Questionable — see TODO #4, #6** |
| **Special** | `NOT_A_FILE_OR_DIR:` | `edge_cases::symlink_to_dev_dir_with_follow` (via `--follow` into /dev) | Yes |
| **Absent** | `MISSING-FILE:` | No | **Questionable — see TODO #2** |

## Row: Orig = Absent

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `EXTRA-FILE:` | `basic::extras` | Yes |
| **Dir** | `EXTRA-DIR:` + `count_recursive` | `basic::extras` | Yes |
| **Sym→file** | `EXTRA-SYMLINK:` | `symlinks::extra_symlink_in_backup` | Yes |
| **Sym→dir** | `EXTRA-SYMLINK:` | `edge_cases::special_files_extra_in_backup` | Yes |
| **Sym→dangling** | `EXTRA-SYMLINK:` | No (same code path as other extra symlink tests) | Yes |
| **Special** | `EXTRA-FILE:` | No | **Questionable — see TODO #3** |
| **Absent** | N/A — entry doesn't exist in either side, never visited | N/A | N/A |

---

## TODO: Potential bugs and questionable behavior

### TODO 1: Sym→dir vs Sym→dir with different targets — targets never compared

**Severity: Bug**

When both sides are symlinks that resolve to directories, the code enters the
`orig_is_dir && backup_is_dir` branch (`compare.rs` line 175) and **never
compares the symlink targets**. With `--follow` it silently traverses both
directories (which may have completely different content). Without `--follow`
it just says `SYMLINK:` and skips. The different symlink targets are silently
ignored.

The user would want to know the symlinks point to different places. At minimum,
a `DIFFERENT-SYMLINK:` should be emitted before traversal/skip.

**No test exists for this case.**

### TODO 2: Special (orig) vs Absent (backup) → reports `MISSING-FILE:`

**Severity: Minor / cosmetic**

A missing device node, FIFO, or socket is reported as `MISSING-FILE:`, which is
misleading. The `handle_missing` function checks for symlink and dir but falls
through to `MISSING-FILE:` for everything else, including special files.

Could warrant a `MISSING-SPECIAL:` or `MISSING-FILE:` with a note, though this
is a rare edge case (special files almost never appear in normal directory trees
outside `/dev`).

**No test exists for this case.**

### TODO 3: Absent (orig) vs Special (backup) → reports `EXTRA-FILE:`

**Severity: Minor / cosmetic**

Same issue as TODO 2 but in reverse. An extra device node in the backup is
reported as `EXTRA-FILE:`. The `handle_extra` function checks for symlink and
dir but falls through to `EXTRA-FILE:` for everything else.

**No test exists for this case.**

### TODO 4: Special vs Sym(any) → `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)`

**Severity: Minor / cosmetic**

When one side is a special file (device, FIFO, socket) and the other is a
symlink, the code reports `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)`. The
prefix is confusing when one side is a device node — the real issue is a
fundamental type mismatch, not a symlink target problem. Will be partially
addressed by TODO #6 (rename to `DIFFERENT-SYMLINK:`).

**No test exists for this case.**

### TODO 5: Dir vs Sym mismatch — directory contents not recursively counted

**Severity: Bug**

When one side is a real directory and the other is a symlink (any kind), the
code reports `DIFFERENT-SYMLINK-TARGET: (symlink mismatch)` and returns
immediately. It does not recursively count the directory's contents as missing
or extra. This affects 6 cells in the matrix:

- Orig=Dir, Backup=Sym→file
- Orig=Dir, Backup=Sym→dir
- Orig=Dir, Backup=Sym→dangling
- Orig=Sym→file, Backup=Dir
- Orig=Sym→dir, Backup=Dir
- Orig=Sym→dangling, Backup=Dir

Expected behavior:
- Orig=Dir, Backup=Sym: emit `DIFFERENT-SYMLINK:` + recursively count all
  contents of the original directory as missing.
- Orig=Sym, Backup=Dir: emit `DIFFERENT-SYMLINK:` + recursively count all
  contents of the backup directory as extra.

This mirrors the existing behavior for Dir vs File type mismatches (which do
call `count_recursive`).

**No test exists for any of these cases.**

### TODO 6: Rename `DIFFERENT-SYMLINK-TARGET:` → `DIFFERENT-SYMLINK:`

**Severity: Minor / cosmetic**

The prefix `DIFFERENT-SYMLINK-TARGET:` is misleading when the issue is not a
target difference but a type mismatch (e.g., one side is a symlink and the
other is a regular file or directory). Rename to `DIFFERENT-SYMLINK:` which
covers all symlink-related mismatches: different targets, one-side-not-a-symlink,
and type mismatches.

Affects: `src/compare.rs` (2 println! calls), `src/cli.rs` (help text),
`README.md`, and all test files referencing the old prefix.

### TODO 7: Dangling symlinks with `--follow` and same target → `NOT_A_FILE_OR_DIR:`

**Severity: Minor**

When both sides are dangling symlinks with the same target, and `--follow` is
used, the code reaches the `!orig_is_file || !backup_is_file` branch (both
`fs::metadata` calls fail with NotFound → `unwrap_or(false)`) and reports
`NOT_A_FILE_OR_DIR:`. Without `--follow`, the same case correctly counts as
a similarity.

This also affects cross-type cases where one side is dangling and the other
resolves (same target string, relative path, different filesystem contexts):
the resolvable side gets `is_file=true` or `is_dir=true` while the dangling
side gets `false`, leading to `NOT_A_FILE_OR_DIR:` or `DIFFERENT-FILE [TYPE]:`.

The `dangling_symlinks_same_target` test only covers the no-`--follow` case.
