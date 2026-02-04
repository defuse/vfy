# Comparison algorithm v2

## Requirements

1. **Item counting**: Every non-ignored filesystem entry is counted as
   an item exactly once.
2. **Completeness**: Every entry and all descendants are fully
   processed — nothing silently dropped.
3. **Self-counting**: Each function counts the entries it processes.
   The caller never pre-counts.
4. **Error recovery**: If one side errors (stat fails, directory
   unreadable), the other side is reported as missing/extra.
5. **Symmetry**: Missing and extra use identical logic parameterized
   by `Direction`.
6. **Uniform entry point**: `compare` accepts any path type. No
   special-casing of the root.

## Meta

All I/O happens during metadata loading. The result is one of:

    Error | Dangling | Special | File | Dir(entries) | Symlink

`load_meta(path)` uses `symlink_metadata`.
`load_meta(path, follow=true)` uses `metadata` (follows symlinks),
plus `readdir` for dirs. Returns `Dangling` if the target doesn't exist.

A directory that stats OK but can't be read is `Error`.

## `compare(orig, backup, follow=false)`

**Pre:** Neither path counted as an item yet.
**Post:** Both counted. Fully categorized. Descendants processed.

The `follow` parameter controls metadata loading: `follow=false` uses
`symlink_metadata` (default), `follow=true` uses `metadata` (follows
symlinks). Only `compare_symlinks` passes `follow=true`.

```
compare(orig, backup, follow=false):
    if ignored(orig or backup): inc_skipped, return

    meta_orig = load_meta(orig, follow)
    meta_back = load_meta(backup, follow)

    // --- Errors / Dangling ---
    if meta_orig is Error or Dangling:
        inc_original_items
        if Error: print ERROR, inc_errors
        if Dangling: print DANGLING-SYMLINK, inc_errors

    if meta_back is Error or Dangling:
        inc_backup_items
        if Error: print ERROR, inc_errors
        if Dangling: print DANGLING-SYMLINK, inc_errors

    if both (Error or Dangling): return

    // --- Special files ---
    if meta_orig is Special:
        inc_original_items, print NOT_A_FILE_OR_DIR, inc_nfd

    if meta_back is Special:
        inc_backup_items, print NOT_A_FILE_OR_DIR, inc_nfd

    if both Special: return

    // --- Same type (helpers count items) ---
    if both File:    compare_files(orig, backup, meta_orig, meta_back); return
    if both Dir:     compare_directories(orig, backup, meta_orig, meta_back); return
    if both Symlink: compare_symlinks(orig, backup); return

    // --- Type mismatch ---
    // Error/Dangling/Special sides already counted above.
    // File/Dir/Symlink sides counted by report() below.

    if one is Symlink and other is (File or Dir):
        print DIFFERENT-SYMLINK-STATUS, inc_different
    elif one is File and other is Dir:
        print DIFFERENT-TYPE, inc_different

    if meta_orig is File, Dir, or Symlink: report(orig, Missing)
    if meta_back is File, Dir, or Symlink: report(backup, Extra)
```

## `compare_files(orig, backup, meta_orig, meta_back)`

**Pre:** Both are files. Neither counted. Metadata loaded.
**Post:** Both counted. Content compared.

```
compare_files(orig, backup, meta_orig, meta_back):
    inc_original_items
    inc_backup_items
    compare content (size, samples, BLAKE3 hash)
    // Each side can independently error (e.g. can't open for reading).
    // Report both errors if both fail — don't let one hide the other.
    if orig error: print ERROR for orig, inc_errors
    if backup error: print ERROR for backup, inc_errors
    if any error: return
    if same: inc_similarities
    if different: print DIFFERENT-FILE, inc_different
```

## `compare_directories(orig, backup, meta_orig, meta_back)`

**Pre:** Both are dirs. Entries pre-loaded. Neither counted.
**Post:** Both counted. All children processed.

```
compare_directories(orig, backup, meta_orig, meta_back):
    if one-filesystem and different device:
        inc_original_items, inc_backup_items
        print DIFFERENT-FS, inc_skipped
        return

    inc_original_items
    inc_backup_items
    inc_similarities

    for name in orig_entries (sorted):
        in_backup = backup_entries.remove(name)
        if in_backup:
            compare(orig/name, backup/name)
        else:
            report(orig/name, Missing)

    for name in remaining backup_entries (sorted):
        report(backup/name, Extra)
```

No ignore checks here — `compare` and `report` check for themselves.

## `compare_symlinks(orig, backup)`

**Pre:** Both are symlinks. Neither counted.
**Post:** Both counted. Targets compared. If `--follow`, resolved
content compared via `compare`.

```
compare_symlinks(orig, backup):
    orig_target = readlink(orig)
    if error:
        inc_original_items, print ERROR, inc_errors
        report(backup, Extra)           // report counts backup
        return

    backup_target = readlink(backup)
    if error:
        inc_backup_items, print ERROR, inc_errors
        report(orig, Missing)           // report counts orig
        return

    inc_original_items
    inc_backup_items

    if targets differ:
        print DIFFERENT-SYMLINK-TARGET, inc_different

    if not --follow:
        print SYMLINK (skipped), inc_skipped
        if same targets: inc_similarities
        return

    // --follow: compare resolved content as additional items.
    // Symlinks are already counted above. The resolved content is
    // counted separately by compare (via its helpers or report).
    if same targets: inc_similarities
    compare(orig, backup, follow=true)
```

With `follow=true`, `compare` uses `metadata` (follows symlinks), so
the resolved types are never `Symlink` — no infinite loop. Dangling
symlinks become `Dangling` in the meta enum, handled by compare's
error/dangling block. Resolved type mismatches (e.g., symlink-to-file
vs symlink-to-dir) fall through to compare's type mismatch logic.

## `report(path, direction, follow=false)`

Report a path and all descendants as missing or extra.

**Pre:** Entry has NOT been counted as an item.
**Post:** Entry counted. Classified. Descendants processed.

The `follow` parameter works the same as in `compare`: `follow=false`
uses `symlink_metadata` (default), `follow=true` uses `metadata`
(follows symlinks). Only used when reporting the resolved content of a
symlink with `--follow`.

```
report(path, direction, follow=false):
    if ignored(path): inc_skipped, return

    direction.inc_items()               // inc_original_items or inc_backup_items

    meta = load_meta(path, follow)
    if meta is Error: print ERROR, inc_errors, return
    if meta is Dangling: print DANGLING-SYMLINK, inc_errors, return

    if meta is Special:
        print NOT_A_FILE_OR_DIR, inc_nfd
        return

    kind = classify(meta)               // File, Dir, or Symlink
    print "{direction.prefix(kind)}: path"
    direction.inc_count()               // inc_missing or inc_extras

    if meta is Dir:
        for child in entries:
            report(child, direction)
    elif meta is Symlink and --follow:
        report(path, direction, follow=true)
```

### Verbosity

`report` has a `print` parameter (omitted above):
- Top-level calls (from `compare` / `compare_directories`): `print=true`.
- Recursive calls (children): `print = (verbosity >= Files)`.

## Entry point

```
compare_dirs(config):
    compare(config.original, config.backup)
```

## Counting rules

1. `compare` counts Error/Dangling/Special sides inline.
   File/Dir/Symlink sides are counted by their helpers or by `report`.
2. `compare_files`, `compare_directories` count both items at their top.
3. `compare_symlinks` counts both items after successful readlink.
   On readlink error, counts the errored side and reports the other.
4. `report` counts its entry as an item immediately (before load_meta),
   so entries are counted even if metadata loading fails.
   No `already_counted` parameter needed.
5. For followed symlinks: the symlink pair is counted by
   `compare_symlinks` / `report`, and the resolved content is counted
   separately by `compare(follow=true)` / `report(follow=true)`.
   Same-target symlinks always count as a similarity.
6. No function relies on the caller to pre-count.

## Ignore handling

Ignore is checked in exactly two places:
1. `compare` — at the top, for the pair.
2. `report` — at the top, for the entry.

## Direction enum

```
Direction { Missing, Extra }
    .prefix(kind) → "MISSING-FILE" / "EXTRA-DIR" / etc.
    .inc_count(stats) → inc_missing or inc_extras
    .inc_items(stats) → inc_original_items or inc_backup_items
```
