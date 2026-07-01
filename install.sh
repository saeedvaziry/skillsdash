#!/usr/bin/env sh
# skillsdash installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/saeedvaziry/skillsdash/main/install.sh | sh
# Environment:
#   SKILLSDASH_VERSION  install a specific version (e.g. 1.2.0); default: latest
#   SKILLSDASH_BIN_DIR  install directory; default: ~/.local/bin (or /usr/local/bin if writable via sudo)
set -eu

REPO="saeedvaziry/skillsdash"
BIN="skillsdash"

err() { printf 'error: %s\n' "$1" >&2; exit 1; }
info() { printf '%s\n' "$1" >&2; }

need() { command -v "$1" >/dev/null 2>&1 || err "missing required command: $1"; }

need uname
need tar

if command -v curl >/dev/null 2>&1; then
  DL="curl -fsSL"
  DL_O="curl -fsSL -o"
elif command -v wget >/dev/null 2>&1; then
  DL="wget -qO-"
  DL_O="wget -qO"
else
  err "need curl or wget"
fi

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) plat="apple-darwin" ;;
  Linux) plat="unknown-linux-gnu" ;;
  *) err "unsupported OS: $os (supported: Linux, macOS)" ;;
esac

case "$arch" in
  x86_64 | amd64) cpu="x86_64" ;;
  arm64 | aarch64) cpu="aarch64" ;;
  *) err "unsupported architecture: $arch (supported: x86_64, arm64)" ;;
esac

# musl fallback: if on Linux and glibc looks absent, prefer the static musl build.
if [ "$os" = "Linux" ] && [ "$cpu" = "x86_64" ]; then
  if ! ldd --version >/dev/null 2>&1 && [ ! -e /lib/x86_64-linux-gnu/libc.so.6 ]; then
    plat="unknown-linux-musl"
  fi
fi

target="${cpu}-${plat}"

version="${SKILLSDASH_VERSION:-}"
if [ -z "$version" ]; then
  info "resolving latest release..."
  version="$($DL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -n1 | sed -E 's/.*"tag_name": *"v?([^"]+)".*/\1/')"
  [ -n "$version" ] || err "could not determine latest version"
fi
version="${version#v}"

archive="${BIN}-${version}-${target}.tar.gz"
base="https://github.com/${REPO}/releases/download/v${version}"
url="${base}/${archive}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

info "downloading ${archive}..."
$DL_O "${tmp}/${archive}" "$url" || err "download failed: $url"

info "verifying checksum..."
$DL_O "${tmp}/${archive}.sha256" "${url}.sha256" 2>/dev/null || err "could not fetch checksum"
expected="$(awk '{print $1}' "${tmp}/${archive}.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "${tmp}/${archive}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "${tmp}/${archive}" | awk '{print $1}')"
else
  err "need sha256sum or shasum to verify download"
fi
[ "$expected" = "$actual" ] || err "checksum mismatch (expected $expected, got $actual)"

tar -C "$tmp" -xzf "${tmp}/${archive}"
extracted="${tmp}/${BIN}-${version}-${target}/${BIN}"
[ -f "$extracted" ] || err "binary not found in archive"
chmod +x "$extracted"

bindir="${SKILLSDASH_BIN_DIR:-}"
if [ -z "$bindir" ]; then
  if [ -w "/usr/local/bin" ]; then
    bindir="/usr/local/bin"
  else
    bindir="${HOME}/.local/bin"
  fi
fi
mkdir -p "$bindir"

if mv "$extracted" "${bindir}/${BIN}" 2>/dev/null; then
  :
elif command -v sudo >/dev/null 2>&1; then
  info "elevating to install into ${bindir}..."
  sudo mv "$extracted" "${bindir}/${BIN}"
else
  err "cannot write to ${bindir}; set SKILLSDASH_BIN_DIR to a writable path"
fi

info "installed ${BIN} ${version} -> ${bindir}/${BIN}"
case ":${PATH}:" in
  *":${bindir}:"*) ;;
  *) info "note: ${bindir} is not on your PATH; add it to use '${BIN}' directly" ;;
esac
