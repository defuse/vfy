# vfy

`vfy` is a directory comparison tool, useful for checking if backups have been
completed or restored successfully.

By default, it compares only by file size, but it also supports checking random
samples within files (with `--samples N`) or full BLAKE3 hash-based comparison
(with `--all`).

To install `vfy`, run...

```
cargo install vfy
```

...and make sure `~/.cargo/bin` is in your `$PATH`. You can also clone the
repository and run `cargo install --path .`

```
$ vfy
CMD: vfy
Verify backup integrity by comparing directory trees. By default, only compares file sizes.

Usage: vfy [OPTIONS] <ORIGINAL> <BACKUP>

Arguments:
  <ORIGINAL>  Original directory
  <BACKUP>    Backup directory

Options:
  -v, --verbose...         Verbose output (-v for dirs, -vv for files, hashes with --all, see below)
  -s, --samples <SAMPLES>  Number of random samples to compare per file [default: 0]
  -a, --all                Full BLAKE3 hash comparison
  -f, --follow             Compare symlinked-to contents (symlink target paths are always compared, even without --follow)
  -o, --one-filesystem     Stay on one filesystem (only supported on Unix-like OSes)
  -i, --ignore <IGNORE>    Ignore one directory or file. Must exist. Ignoring one side also ignores the other.
  -h, --help               Print help

WARNING: By default, it only compares by file size; files themselves are not read.
WARNING: Only officially supported on Linux, but seems to work on Windows/Mac.
WARNING: Output behavior is currently NOT STABLE between releases.

ORIGINAL and BACKUP are not interchangeable:
  EXTRA-* suggests data can be deleted from the backup, so a wrong EXTRA-* is
  more dangerous than a wrong MISSING-*. When one side cannot be checked (an
  error, or a symlink loop), we report the side we can still see, and which
  side failed decides what we say about it:
    - Backup could not be checked: the original is reported MISSING-*, since
      we cannot confirm it was backed up.
    - Original could not be checked: the backup is only reported SKIP, since
      we cannot confirm the original lacks it, and EXTRA-* would invite
      deleting it.
  Swapping the two arguments does not simply mirror the output.

Verbosity levels:
  (default)  Show differences only. For missing/extra directories, only the
             top-level directory is listed; children are counted but not shown.
  -v         Add DEBUG lines showing each directory comparison.
  -vv        Add DEBUG lines for file comparisons. Show all individual entries
             inside missing/extra directories. With --all, show BLAKE3 hashes.

Output prefixes (grep-friendly):
  MISSING-FILE:                  File in original missing from backup
  MISSING-DIR:                   Directory in original missing from backup
  MISSING-SYMLINK:               Symlink in original missing from backup
  MISSING-SPECIAL:               Special file in original missing from backup
  MISSING-ERROR:                 Something (that errored) in original missing from backup
  EXTRA-FILE:                    File in backup not in original
  EXTRA-DIR:                     Directory in backup not in original
  EXTRA-SYMLINK:                 Symlink in backup not in original
  EXTRA-SPECIAL:                 Extra special file in backup not in original
  EXTRA-ERROR:                   Extra something (that errored) in backup not in original
  DIFFERENT-FILE [reason]:       File differs (reason: first mismatch of SIZE, SAMPLE, HASH)
  FILE-DIR-MISMATCH:             One side is a file, the other is a directory
  DIFFERENT-SYMLINK-TARGET:      Both sides are symlinks but point to different targets
  DIFFERENT-SYMLINK-STATUS:      One side is a symlink, the other is not
  SPECIAL-FILE:                  Entry is a device, FIFO, socket, etc.
  SYMLINK-SKIPPED:               Symlink skipped (use --follow to compare resolved content)
  DANGLING-SYMLINK:              Symlink target does not exist (with --follow)
  SYMLINK-LOOP:                  Symlink resolves into a directory already being walked (--follow)
  DIFFERENT-FS:                  Different filesystem skipped (--one-filesystem)
  SKIP:                          Entry skipped via --ignore or error/FS/type mismatch between sides
  ERROR:                         I/O or permission error
  DEBUG:                         Verbose logging (-v dirs, -vv files and hashes)
  SUMMARY:                       Final counts (not guaranteed to add up to 100%)

Symlink handling with --follow:
  When both sides are symlinks with different targets:
    - Reports DIFFERENT-SYMLINK-TARGET as a warning.
    - Continues comparing resolved contents (may find similarities).

  When one side is a symlink and the other is a regular file/directory:
    - Reports DIFFERENT-SYMLINK-STATUS for the type mismatch.
    - Reports original as MISSING-*, backup symlink as EXTRA-* (or vice-versa).
    - Does NOT compare contents.

  Rationale: A symlink replacing a directory is a structural failure--the backup
  holds a pointer where the data should be. Comparing through it would resolve
  to whatever it points at, possibly the original's own files, and report a
  match, so a backup that only points back at the original would look correct
  while holding no copy at all. We do not try to tell that case apart from a
  harmless one, so we never compare through, and report the data as missing
  instead. Two symlinks with different targets is a metadata difference rather
  than a structural one--each tree still has whatever its own link points at--so
  there the resolved contents are worth comparing.

  When a symlink resolves to a directory that is already being walked
  (e.g. `latest -> .` or `sub/up -> ..`):
    - Reports SYMLINK-LOOP, stops descending, and counts an error.
    - No data goes uncompared: that directory's contents get compared where it
      is really walked, so following the loop would only repeat that work.
    - If only one side loops, the other still points at data that never got
      checked, and the rule above applies to it: if that data is in the
      original it is reported MISSING-*, if it is in the backup it is only
      reported SKIP.
```

Note: The `--one-filesystem` tests assume your development environment is a
Linux system with `/dev/shm/` writable; they fail on Windows/Mac. Most of the
other tests are broken on Windows as well due to the use of a Unix-specific
filesystem library. As such, those platforms are not officially supported, but
it builds and seems to work fine.

**AI Use Policy:** AI tools were used to assist with writing this utility. All
code in the core utility has been fully reviewed, and rewritten for clarity when
necessary, by myself (a human). If you would like to submit a PR, using AI is
fine, but you must stand by the correctness of your submission as strongly as
you would if you had written the code yourself.
