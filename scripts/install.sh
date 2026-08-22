#!/usr/bin/env sh
set -e

REPO="josiah-mbao/pulse"
BIN_NAME="pulse"

echo "==> Pulse Linux Installer"

# 1. Detect Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Linux" ]; then
    echo "Error: Pulse installer currently supports Linux only." >&2
    exit 1
fi

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

# 2. Determine installation directory
if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ]; then
    INSTALL_DIR="$HOME/.local/bin"
else
    INSTALL_DIR="/usr/local/bin"
fi

# 3. Fetch latest release version from GitHub API
LATEST_TAG=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo "Error: Unable to determine latest release tag for ${REPO}." >&2
    exit 1
fi

echo "Found latest release: ${LATEST_TAG} (${TARGET_ARCH}-unknown-linux-gnu)"

TARBALL="pulse-${LATEST_TAG}-${TARGET_ARCH}-unknown-linux-gnu.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST_TAG}/${TARBALL}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ${DOWNLOAD_URL}..."
curl -sSL "$DOWNLOAD_URL" -o "${TMP_DIR}/${TARBALL}"

echo "Extracting binary..."
tar -xzf "${TMP_DIR}/${TARBALL}" -C "$TMP_DIR"

if [ ! -f "${TMP_DIR}/${BIN_NAME}" ]; then
    echo "Error: Extracted archive did not contain expected '${BIN_NAME}' executable." >&2
    exit 1
fi

echo "Installing ${BIN_NAME} to ${INSTALL_DIR}..."
if [ -w "$INSTALL_DIR" ]; then
    mv "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
else
    echo "Root privileges required to install into ${INSTALL_DIR}"
    sudo mv "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    sudo chmod +x "${INSTALL_DIR}/${BIN_NAME}"
fi

echo ""
echo "✅ Pulse successfully installed to ${INSTALL_DIR}/${BIN_NAME}"
echo ""
echo "Usage:"
echo "  pulse        # Unprivileged mode (/proc telemetry)"
echo "  sudo pulse   # eBPF mode (kernel process lifecycle tracing)"
