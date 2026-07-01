#!/usr/bin/env bash
# Rewrite Formula/skillsdash.rb with the release version and per-target sha256 sums.
# Usage: update-formula.sh <version> <dist-dir>
set -euo pipefail

VERSION="${1:?version required}"
DIST="${2:?dist dir required}"
FORMULA="Formula/skillsdash.rb"

sha_for() {
  local target="$1"
  local file="${DIST}/skillsdash-${VERSION}-${target}.tar.gz.sha256"
  if [ ! -f "$file" ]; then
    echo "::error::missing checksum for ${target} (${file})" >&2
    exit 1
  fi
  awk '{print $1}' "$file"
}

MAC_ARM="$(sha_for aarch64-apple-darwin)"
MAC_X86="$(sha_for x86_64-apple-darwin)"
LIN_ARM="$(sha_for aarch64-unknown-linux-gnu)"
LIN_X86="$(sha_for x86_64-unknown-linux-gnu)"

python3 - "$FORMULA" "$VERSION" "$MAC_ARM" "$MAC_X86" "$LIN_ARM" "$LIN_X86" <<'PY'
import re
import sys

formula, version, mac_arm, mac_x86, lin_arm, lin_x86 = sys.argv[1:]
text = open(formula).read()

text = re.sub(r'version "[^"]*"', f'version "{version}"', text, count=1)

# Replace the four sha256 lines in declaration order: mac arm, mac intel, linux arm, linux intel.
order = [mac_arm, mac_x86, lin_arm, lin_x86]
idx = 0

def repl(_m):
    global idx
    val = order[idx]
    idx += 1
    return f'sha256 "{val}"'

text = re.sub(r'sha256 "[0-9a-fA-F]{64}"', repl, text)

if idx != 4:
    sys.exit(f"expected 4 sha256 fields, replaced {idx}")

open(formula, "w").write(text)
print(f"formula updated to {version}")
PY
