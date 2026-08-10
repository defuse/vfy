#!/bin/bash
set -euo pipefail

# Update the help output in README.md by running the built binary.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
README="$REPO_ROOT/README.md"

# Build first so the binary is up to date.
cargo build --manifest-path "$REPO_ROOT/Cargo.toml"

# Find the built binary.
VFY="$(cargo metadata --manifest-path "$REPO_ROOT/Cargo.toml" --format-version 1 \
    | jq -r '.target_directory')/debug/vfy"

# Run vfy via PATH so the output shows "CMD: vfy" not a full path.
HELP_OUTPUT="$(PATH="$(dirname "$VFY"):$PATH" vfy 2>&1 || true)"

# Build the replacement block: ``` + help output + ```
REPLACEMENT='```
$ vfy
'"$HELP_OUTPUT"'
```'

# Replace the fenced block that starts with "$ vfy". Anchoring on that line
# rather than on the first fence in the file keeps us from overwriting the
# install instructions, which also live in a fenced block above this one.
REPLACEMENT="$REPLACEMENT" perl -0777 -i -pe '
    my $r = $ENV{REPLACEMENT};
    s/^```\n\$ vfy\n.*?^```/$r/ms or die "help block not found in README\n";
' "$README"

echo "README.md updated."
