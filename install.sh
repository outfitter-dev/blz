#!/usr/bin/env sh
# Simplified installer for the blz CLI.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/outfitter-dev/blz/main/install.sh | sh
# or download locally and run `sh install.sh`.

set -eu

REPO="outfitter-dev/blz"
BINARY="blz"
INSTALL_DIR_DEFAULT="$HOME/.local/bin"
LATEST_URL="https://github.com/$REPO/releases/latest"
EXPANDED_ASSETS_BASE="https://github.com/$REPO/releases/expanded_assets"
DOWNLOAD_BASE="https://github.com/$REPO/releases/download"

if [ -n "${NO_COLOR:-}" ] || [ ! -t 2 ]; then
    COLOR_INFO="=>"
    COLOR_WARN="!!"
    COLOR_ERR="xx"
else
    COLOR_INFO="\033[1;34m=>\033[0m"
    COLOR_WARN="\033[1;33m!!\033[0m"
    COLOR_ERR="\033[1;31mxx\033[0m"
fi

info() { printf "%s %s\n" "$COLOR_INFO" "$*" >&2; }
warn() { printf "%s %s\n" "$COLOR_WARN" "$*" >&2; }
err() { printf "%s %s\n" "$COLOR_ERR" "$*" >&2; exit 1; }

usage() {
    cat <<'USAGE'
blz install script

Options:
  --version <vX.Y.Z>  Install a specific release (defaults to latest)
  --dir <path>        Installation directory (default: ~/.local/bin)
  --skip-check        Skip SHA-256 verification (not recommended)
  --dry-run           Download and unpack, but do not install
  -h, --help          Show this help message
USAGE
}

VERSION=""
INSTALL_DIR="${BLZ_INSTALL_DIR:-$INSTALL_DIR_DEFAULT}"
SKIP_CHECK=0
DRY_RUN=0

while [ "${#}" -gt 0 ]; do
    case "$1" in
        --version)
            shift || err "--version requires an argument"
            VERSION="$1"
            ;;
        --dir|--install-dir)
            shift || err "--dir requires an argument"
            INSTALL_DIR="$1"
            ;;
        --skip-check)
            SKIP_CHECK=1
            ;;
        --dry-run)
            DRY_RUN=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            err "Unknown option: $1"
            ;;
    esac
    shift || break
done

command -v curl >/dev/null 2>&1 || err "curl is required"
command -v tar >/dev/null 2>&1 || err "tar is required"

# Determine version/tag
if [ -n "$VERSION" ]; then
    case "$VERSION" in
        v*) TAG="$VERSION" ;;
        *) TAG="v$VERSION" ;;
    esac
else
    info "Detecting latest release"
    redirect_url=$(curl -fsSL -o /dev/null -w '%{url_effective}' "$LATEST_URL") || err "Unable to resolve latest release"
    TAG=$(printf '%s' "$redirect_url" | sed -n 's#.*/tag/\(v[^/]*\)#\1#p')
    [ -n "$TAG" ] || err "Failed to determine latest version"
fi
VERSION_TRIMMED=${TAG#v}
info "Installing blz $TAG"

# OS / architecture detection
OS=$(uname -s 2>/dev/null || echo "")
ARCH=$(uname -m 2>/dev/null || echo "")
case "$OS" in
    Darwin) PLATFORM="darwin" ;;
    Linux) PLATFORM="linux" ;;
    *) err "Unsupported operating system: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64) ARCH_TOKEN="x64" ;;
    arm64|aarch64)
        if [ "$PLATFORM" = "darwin" ]; then
            ARCH_TOKEN="arm64"
        else
            err "No prebuilt binary for architecture: $ARCH"
        fi
        ;;
    *) err "Unsupported architecture: $ARCH" ;;
esac

ARCHIVE_NAME="$BINARY-$VERSION_TRIMMED-$PLATFORM-$ARCH_TOKEN.tar.gz"
DOWNLOAD_URL="$DOWNLOAD_BASE/$TAG/$ARCHIVE_NAME"
info "Selected asset: $ARCHIVE_NAME"

TMPDIR=$(mktemp -d)
cleanup() { rm -rf "$TMPDIR"; }
trap cleanup EXIT INT TERM
ARCHIVE_PATH="$TMPDIR/$ARCHIVE_NAME"

info "Downloading binary"
curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE_PATH" || err "Failed to download $DOWNLOAD_URL"

if [ "$SKIP_CHECK" -eq 0 ]; then
    info "Fetching checksum"
    assets_html=$(curl -fsSL "$EXPANDED_ASSETS_BASE/$TAG") || err "Failed to load assets page"
    SHA256=$(printf '%s\n' "$assets_html" | awk -v name="$ARCHIVE_NAME" '
        index($0, name) {found=1}
        found && $0 ~ /sha256:/ {
            gsub(/.*sha256:/, ""); gsub(/<.*/, ""); gsub(/[^0-9a-f]/, "");
            if (length($0) == 64) {print; exit}
        }
    ')
    [ -n "$SHA256" ] || err "Could not locate sha256 for $ARCHIVE_NAME"

    if command -v sha256sum >/dev/null 2>&1; then
        CALC=$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        CALC=$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')
    else
        err "Need sha256sum or shasum for verification (or rerun with --skip-check)"
    fi

    if [ "${CALC}" != "$SHA256" ]; then
        err "Checksum mismatch (expected $SHA256, got $CALC)"
    fi
    info "Checksum verified"
else
    warn "Skipping checksum verification"
fi

info "Extracting archive"
tar -xf "$ARCHIVE_PATH" -C "$TMPDIR"
BIN_PATH=""
for candidate in "$TMPDIR/$BINARY" $(find "$TMPDIR" -type f -name "$BINARY" -perm -111 2>/dev/null); do
    if [ -f "$candidate" ]; then
        BIN_PATH="$candidate"
        break
    fi
done
[ -n "$BIN_PATH" ] || err "Failed to locate extracted binary"
chmod +x "$BIN_PATH"

if [ "$DRY_RUN" -eq 1 ]; then
    info "Dry-run: extracted binary at $BIN_PATH"
    info "Skipping install (binary removed when script exits)"
    exit 0
fi

info "Installing to $INSTALL_DIR"
if mkdir -p "$INSTALL_DIR" 2>/dev/null; then
    install -m 0755 "$BIN_PATH" "$INSTALL_DIR/$BINARY" || err "Failed to install binary"
else
    if command -v sudo >/dev/null 2>&1; then
        warn "Elevated permissions required for $INSTALL_DIR"
        sudo mkdir -p "$INSTALL_DIR" || err "Failed to create $INSTALL_DIR"
        sudo install -m 0755 "$BIN_PATH" "$INSTALL_DIR/$BINARY" || err "Failed to install binary"
    else
        err "Cannot write to $INSTALL_DIR (try specifying --dir or run with sudo)"
    fi
fi

info "Installed $BINARY to $INSTALL_DIR"

case ":$PATH:" in
    *:"$INSTALL_DIR":*) ;;
    *)
        warn "$INSTALL_DIR is not on your PATH"
        printf '   Add the following to your shell profile, e.g. ~/.bashrc:\n'
        printf '     export PATH="%s:$PATH"\n' "$INSTALL_DIR"
        ;;
esac

info "blz installation complete"
printf '\nRun "blz --help" to get started.\n'
