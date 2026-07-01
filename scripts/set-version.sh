#!/usr/bin/env bash
# Set the [package] version in Cargo.toml. Only the first `version = "..."`
# under [package] is changed; dependency versions are untouched.
# Usage: set-version.sh <version>
set -euo pipefail

VERSION="${1:?version required}"

python3 - "$VERSION" <<'PY'
import re
import sys

version = sys.argv[1]
path = "Cargo.toml"
text = open(path).read()

# Replace the version line inside the [package] table only.
def repl(match):
    return f'{match.group(1)}version = "{version}"'

new, n = re.subn(
    r'(\[package\][^\[]*?\n)version = "[^"]*"',
    repl,
    text,
    count=1,
    flags=re.S,
)
if n != 1:
    sys.exit("could not find [package] version in Cargo.toml")

open(path, "w").write(new)
print(f"Cargo.toml version set to {version}")
PY
