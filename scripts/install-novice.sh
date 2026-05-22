#!/bin/sh
# Install the latest "novice herdr" release into $HOME/.local/bin (or
# wherever NOVICE_HERDR_INSTALL_DIR points). Detects platform; falls
# back to a clear error on anything we don't ship a binary for.
set -eu

REPO="alstrup/herdr"
BIN="herdr"
INSTALL_DIR="${NOVICE_HERDR_INSTALL_DIR:-$HOME/.local/bin}"

log() { printf '\033[1m%s\033[0m\n' "$*"; }
err() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    *)      err "unsupported OS: $OS (novice herdr ships Linux + macOS)" ;;
esac
case "$ARCH" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)             err "unsupported architecture: $ARCH" ;;
esac

asset="${BIN}-${os}-${arch}"
url="https://github.com/${REPO}/releases/latest/download/${asset}"
target="${INSTALL_DIR}/${BIN}"

command -v curl >/dev/null 2>&1 || err "curl is required"
mkdir -p "$INSTALL_DIR"

log "fetching ${url}"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
curl -fL --retry 3 --connect-timeout 10 --max-time 120 "$url" -o "$tmp" \
    || err "download failed; the release may not have published binaries yet"

chmod +x "$tmp"
mv "$tmp" "$target"

log "installed ${target}"
"$target" --version

case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo
       echo "note: ${INSTALL_DIR} is not on your PATH. Add this to your shell rc:"
       echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
       ;;
esac
