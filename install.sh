#!/bin/bash
set -euo pipefail

REPO="ianzepp/skyfi-cli"
BINARY="skyfi"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Darwin) OS_TARGET="apple-darwin" ;;
    Linux)  OS_TARGET="unknown-linux-gnu" ;;
    *)
        echo "Error: unsupported OS: $OS"
        exit 1
        ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    arm64|aarch64) ARCH="aarch64" ;;
    x86_64)        ARCH="x86_64" ;;
    *)
        echo "Error: unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Linux ARM is not supported
if [ "$OS_TARGET" = "unknown-linux-gnu" ] && [ "$ARCH" = "aarch64" ]; then
    echo "Error: Linux ARM (aarch64) is not currently supported"
    exit 1
fi

TARGET="${ARCH}-${OS_TARGET}"
ASSET="${BINARY}-${TARGET}.tar.gz"

# Get latest release tag
echo "Finding latest release..."
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$TAG" ]; then
    echo "Error: could not find latest release"
    exit 1
fi
echo "Latest release: ${TAG}"

# Download
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading ${ASSET}..."
curl -fsSL "$URL" -o "${TMPDIR}/${ASSET}"

# Extract
tar xzf "${TMPDIR}/${ASSET}" -C "$TMPDIR"

# Install
if [ -w "$INSTALL_DIR" ]; then
    mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
fi

chmod +x "${INSTALL_DIR}/${BINARY}"

echo "Installed ${BINARY} ${TAG} to ${INSTALL_DIR}/${BINARY}"
