#!/usr/bin/env sh
# ROSQL installer — downloads the pre-built binary for your platform.
#
# Usage:
#   curl -fsSL https://rosql.org/install.sh | sh
#
# The binary is installed to ~/.local/bin/rosql by default.
# Set ROSQL_INSTALL_DIR to override the destination directory.
#
# Supported platforms:
#   Linux  x86_64     (x86_64-unknown-linux-gnu)
#   Linux  arm64      (aarch64-unknown-linux-gnu)
#   macOS  arm64      (aarch64-apple-darwin)      — Apple Silicon
#
# Unsupported platforms (build from source):
#   macOS  x86_64     (Intel Mac)
#   Windows            (any arch)
#
# To build from source on any platform:
#   cargo install rosql --features server,duckdb

set -eu

REPO="RobotOpsInc/rosql"
BINARY="rosql"
INSTALL_DIR="${ROSQL_INSTALL_DIR:-$HOME/.local/bin}"

# ── Detect OS ────────────────────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux*)
    case "$ARCH" in
      x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
      *)
        echo "Error: unsupported Linux architecture: $ARCH"
        echo "Build from source: cargo install rosql --features server,duckdb"
        exit 1
        ;;
    esac
    ;;
  Darwin*)
    case "$ARCH" in
      arm64)    TARGET="aarch64-apple-darwin" ;;
      x86_64)
        echo "Error: pre-built binaries are not available for Intel Macs."
        echo "Build from source: cargo install rosql --features server,duckdb"
        exit 1
        ;;
      *)
        echo "Error: unsupported macOS architecture: $ARCH"
        echo "Build from source: cargo install rosql --features server,duckdb"
        exit 1
        ;;
    esac
    ;;
  CYGWIN*|MINGW*|MSYS*|Windows*)
    echo "Error: Windows is not supported by this installer."
    echo "Build from source: cargo install rosql --features server,duckdb"
    exit 1
    ;;
  *)
    echo "Error: unsupported operating system: $OS"
    echo "Build from source: cargo install rosql --features server,duckdb"
    exit 1
    ;;
esac

# ── Fetch latest release version ────────────────────────────────────────────

echo "Fetching latest ROSQL release..."
LATEST_URL="https://api.github.com/repos/${REPO}/releases/latest"

if command -v curl >/dev/null 2>&1; then
  TAG="$(curl -fsSL "$LATEST_URL" | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
elif command -v wget >/dev/null 2>&1; then
  TAG="$(wget -qO- "$LATEST_URL" | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
else
  echo "Error: curl or wget is required to download ROSQL."
  exit 1
fi

if [ -z "$TAG" ]; then
  echo "Error: could not determine the latest release tag."
  echo "Check https://github.com/${REPO}/releases for the latest version."
  exit 1
fi

VERSION="${TAG#v}"
echo "Installing ROSQL ${TAG} for ${TARGET}..."

# ── Download and extract ─────────────────────────────────────────────────────

TARBALL="rosql-${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${TARBALL}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ${DOWNLOAD_URL}..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "${TMP_DIR}/${TARBALL}" "$DOWNLOAD_URL"
else
  wget -q -O "${TMP_DIR}/${TARBALL}" "$DOWNLOAD_URL"
fi

tar -xzf "${TMP_DIR}/${TARBALL}" -C "$TMP_DIR"

# ── Install ──────────────────────────────────────────────────────────────────

mkdir -p "$INSTALL_DIR"
mv "${TMP_DIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

# ── Verify ───────────────────────────────────────────────────────────────────

INSTALLED_VERSION="$("${INSTALL_DIR}/${BINARY}" --version 2>/dev/null | head -1 || true)"
echo ""
echo "ROSQL installed successfully!"
echo "  Location:  ${INSTALL_DIR}/${BINARY}"
if [ -n "$INSTALLED_VERSION" ]; then
  echo "  Version:   ${INSTALLED_VERSION}"
fi

# ── PATH hint ────────────────────────────────────────────────────────────────

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    # Already in PATH
    ;;
  *)
    echo ""
    echo "Add ROSQL to your PATH by adding this line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo ""
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then restart your shell or run: source ~/.bashrc"
    ;;
esac

echo ""
echo "Get started:"
echo "  rosql compile \"FROM traces WHERE status = 'ERROR' SINCE 1 hour ago\" --backend parquet"
echo "  rosql query  \"FROM traces WHERE status = 'ERROR' SINCE 1 hour ago\" \\"
echo "    --backend parquet --url ./path/to/telemetry/"
echo ""
echo "Documentation: https://rosql.org/docs"
