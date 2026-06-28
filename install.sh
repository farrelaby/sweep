#!/usr/bin/env sh
set -eu

VERSION="${VERSION:-latest}"
REPO="farrelaby/dirsweep"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

if [ "$VERSION" = "latest" ]; then
  if command -v jq > /dev/null 2>&1; then
    VERSION=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | jq -r '.tag_name' | sed 's/^v//')
  else
    VERSION=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | grep -o '"tag_name": *"v[^"]*"' | sed 's/.*"v\(.*\)"/\1/')
  fi
  if [ -z "$VERSION" ]; then
    echo "Error: Failed to fetch latest version from GitHub"
    exit 1
  fi
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64 | amd64) ARCH="x86_64" ;;
  aarch64 | arm64) ARCH="aarch64" ;;
  *)
    echo "Error: Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

case "$OS" in
  linux) TARGET="x86_64-unknown-linux-gnu"
    if [ "$ARCH" = "aarch64" ]; then
      TARGET="aarch64-unknown-linux-gnu"
    fi
    ;;
  darwin)
    if [ "$ARCH" != "aarch64" ]; then
      echo "Error: Only Apple Silicon (aarch64) macOS is supported. For Intel Macs, use 'cargo install dirsweep'."
      exit 1
    fi
    TARGET="aarch64-apple-darwin"
    ;;
  *)
    echo "Error: Unsupported OS: $OS"
    exit 1
    ;;
esac

URL="https://github.com/$REPO/releases/download/v$VERSION/dirsweep-v$VERSION-$TARGET.tar.gz"
TMP_DIR=$(mktemp -d)

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

echo "Downloading dirsweep v$VERSION for $TARGET..."
if command -v curl > /dev/null 2>&1; then
  if ! curl -sL -f "$URL" -o "$TMP_DIR/dirsweep.tar.gz"; then
    echo "Error: Download failed — check your network or the release URL"
    exit 1
  fi
elif command -v wget > /dev/null 2>&1; then
  if ! wget -q "$URL" -O "$TMP_DIR/dirsweep.tar.gz"; then
    echo "Error: Download failed — check your network or the release URL"
    exit 1
  fi
else
  echo "Error: Need curl or wget to download"
  exit 1
fi

if ! tar -xzf "$TMP_DIR/dirsweep.tar.gz" -C "$TMP_DIR" 2>/dev/null; then
  echo "Error: Failed to extract archive — download may be corrupt"
  exit 1
fi

if [ ! -f "$TMP_DIR/dirsweep" ]; then
  echo "Error: Extracted binary not found in archive"
  exit 1
fi

chmod +x "$TMP_DIR/dirsweep"
mv "$TMP_DIR/dirsweep" "$INSTALL_DIR/dirsweep"

echo "dirsweep v$VERSION installed to $INSTALL_DIR/dirsweep"
