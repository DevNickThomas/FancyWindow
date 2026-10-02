#!/usr/bin/env bash
# Regenerates THIRD-PARTY-NOTICES.txt from the crates in the current build.
# Every dependency offers the MIT licence; its notice is reproduced in full.
set -euo pipefail
cd "$(dirname "$0")/.."
registry=$(ls -d ~/.cargo/registry/src/*/ | head -1)
out=THIRD-PARTY-NOTICES.txt
{
  echo "Fancy Window includes the following open-source components."
  echo "Each is used under the MIT licence; their notices follow."
  echo
  echo "It is built with the Rust standard library (MIT OR Apache-2.0,"
  echo "https://github.com/rust-lang/rust) and the MinGW-w64 runtime"
  echo "(https://www.mingw-w64.org), whose licences permit redistribution."
  cargo tree -e normal --prefix none --target x86_64-pc-windows-gnu \
    | sed 's/ (\*)//; s/ (proc-macro)//' | sort -u | grep -v '^fancy-window' \
    | while read -r name ver _; do
        dir="$registry$name-${ver#v}"
        licence=$(grep -m1 '^license *=' "$dir/Cargo.toml" | cut -d'"' -f2)
        mit=$(ls "$dir" | grep -iE '^licen[cs]e-mit' | head -1)
        echo
        echo "================================================================"
        echo "$name ${ver#v}  ($licence)"
        echo "================================================================"
        cat "$dir/$mit"
      done
} > "$out"
echo "wrote $out"
