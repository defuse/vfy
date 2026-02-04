This is a tool for verifying your backups completed successfully.

```
$ vfy
CMD: vfy
Verify backup integrity by comparing directory trees

Usage: vfy [OPTIONS] <ORIGINAL> <BACKUP>

Arguments:
  <ORIGINAL>  Original directory
  <BACKUP>    Backup directory

Options:
  -v, --verbose...         Verbose output (-v for dirs, -vv for files)
  -s, --samples <SAMPLES>  Number of random samples to compare per file [default: 0]
  -a, --all                Full BLAKE3 hash comparison
  -f, --follow             Follow symlinks into directories
  -o, --one-filesystem     Stay on one filesystem
  -i, --ignore <IGNORE>    Directories to ignore (can be specified multiple times)
  -h, --help               Print help

Output prefixes (grep-friendly):
  MISSING-FILE:                  File in original missing from backup
  MISSING-DIR:                   Directory in original missing from backup
  MISSING-SYMLINK:               Symlink in original missing from backup
  EXTRA-FILE:                    File in backup not in original
  EXTRA-DIR:                     Directory in backup not in original
  EXTRA-SYMLINK:                 Symlink in backup not in original
  DIFFERENT-FILE [reason]:       File differs (reason: first mismatch of SIZE, SAMPLE, HASH)
  DIFFERENT-TYPE:                One side is a file, the other is a directory
  DIFFERENT-SYMLINK-TARGET:      Both sides are symlinks but point to different targets
  DIFFERENT-SYMLINK-STATUS:      One side is a symlink, the other is not
  NOT_A_FILE_OR_DIR:             Entry is a device, FIFO, socket, etc.
  SYMLINK:                       Symlink skipped (use --follow to compare content)
  DANGLING-SYMLINK:              Symlink target does not exist (with --follow)
  DIFFERENT-FS:                  Different filesystem skipped (--one-filesystem)
  SKIP:                          Entry skipped via --ignore
  ERROR:                         I/O or permission error
  DEBUG:                         Verbose logging (-v dirs, -vv files and hashes)
  SUMMARY:                       Final counts
```
