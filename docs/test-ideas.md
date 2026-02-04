# Test cases to add

- FIFO inside missing directory
- --follow with many nested levels of symlink directories (verify counts are correct)
- Add a dangling symlink counter to summary output and test it
- Invariant: DANGLING-SYMLINK messages should never appear without --follow
- Make sure all tests with Dir have children in the dir
- Update symdangling_x_symdangling_same_follow when dangling symlink counter is added


we verified up to and not including symfile_x_symfile_diff_follow