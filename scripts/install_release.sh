#!/usr/bin/env bash
set -euo pipefail

# Installs Git Projects Manager from the latest GitHub release for the host platform:
#   macOS → /Applications/Git Projects Manager.app
#   Linux → system install via the released .deb (Debian/Ubuntu)
# Intended to be curlable:
#   curl -fsSL https://raw.githubusercontent.com/LeoManrique/git-projects-manager/master/scripts/install_release.sh | bash
#
# Unlike install_macos.sh / install_arch.sh (which install a locally built
# bundle), this script downloads a published release artifact — the same ones
# deploy_releases.sh uploads.

# ── Colors ──
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

TOTAL_STEPS=5
step()    { echo -e "\n${BLUE}[$1/$TOTAL_STEPS]${NC} ${CYAN}$2${NC}"; }
success() { echo -e "  ${GREEN}✓ $1${NC}"; }
warn()    { echo -e "  ${YELLOW}⚠ $1${NC}"; }
error()   { echo -e "  ${RED}✗ $1${NC}"; exit 1; }

REPO="LeoManrique/git-projects-manager"
API_URL="https://api.github.com/repos/$REPO/releases/latest"
TMP_DIR="/tmp/git-projects-manager-install"
PROC="git-projects-manager"           # bundle executable name on every platform
APP_NAME="Git Projects Manager.app"   # macOS bundle name inside the zip

# ── Step 1: Detect platform ──
step 1 "Detecting platform"

# Release artifacts use uname-style arch labels (x86_64 / aarch64) to match
# deploy_releases.sh — not the Go-style amd64/arm64 some installers use.
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  arm64|aarch64) ARCH="aarch64" ;;
  x86_64)        ARCH="x86_64" ;;
  *) error "Unsupported architecture: $ARCH" ;;
esac
case "$OS" in
  darwin) PLATFORM="macOS-$ARCH"; ARTIFACT_EXT="zip" ;;
  linux)  PLATFORM="linux-$ARCH"; ARTIFACT_EXT="deb" ;;
  *) error "Unsupported OS: $OS (this installer supports macOS and Linux)" ;;
esac
success "Platform: $PLATFORM"

# ── Step 2: Stop any running instance ──
# A running app holds an open file lock on its binary; replacing the bundle
# on disk while the old copy is running leaves you with a half-old/half-new
# install until the next launch. Kill it first.
step 2 "Stopping running instance"

if pgrep -x "$PROC" >/dev/null 2>&1; then
  pkill -TERM -x "$PROC" 2>/dev/null || true
  for _ in $(seq 1 16); do
    pgrep -x "$PROC" >/dev/null 2>&1 || break
    sleep 0.5
  done
  if pgrep -x "$PROC" >/dev/null 2>&1; then
    warn "Force-killing $PROC (graceful stop timed out)"
    pkill -KILL -x "$PROC" 2>/dev/null || true
  fi
  success "Stopped running instance"
else
  success "No running instances"
fi

# ── Step 3: Fetch latest release metadata ──
step 3 "Fetching latest release from GitHub"

RELEASE_JSON=$(curl -fsSL -H "Accept: application/vnd.github.v3+json" "$API_URL" 2>/dev/null) \
  || error "Failed to fetch release info from GitHub. Check your internet connection."

TAG=$(echo "$RELEASE_JSON" | { grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' || true; } | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
[ -z "$TAG" ] && error "Could not parse release tag from GitHub API"

VERSION="${TAG#v}"
success "Latest version: $VERSION (tag: $TAG)"

# ── Step 4: Download artifact ──
ARTIFACT="git-projects-manager-$VERSION-$PLATFORM.$ARTIFACT_EXT"
step 4 "Downloading $ARTIFACT"

# `|| true` keeps a no-match from tripping `set -o pipefail` and aborting the
# script silently before the explicit "not found" check below can run.
DOWNLOAD_URL=$(echo "$RELEASE_JSON" | { grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*'"$ARTIFACT"'"' || true; } | head -1 | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
[ -z "$DOWNLOAD_URL" ] && error "Could not find artifact $ARTIFACT in release $TAG (was it built for $PLATFORM?)"

rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

curl -fSL --progress-bar -o "$TMP_DIR/$ARTIFACT" "$DOWNLOAD_URL" \
  || error "Failed to download $ARTIFACT"
success "Downloaded $ARTIFACT"

# ── Step 5: Install ──
step 5 "Installing Git Projects Manager"

if [ "$OS" = "darwin" ]; then
  DEST="/Applications/$APP_NAME"

  # `ditto -x -k` unpacks the zip preserving the bundle structure (xattrs,
  # code signatures, symlinks) — the inverse of how deploy_releases.sh packs it.
  ditto -x -k "$TMP_DIR/$ARTIFACT" "$TMP_DIR"
  [ -d "$TMP_DIR/$APP_NAME" ] || error "Expected $APP_NAME inside $ARTIFACT, but didn't find it"

  if [ -d "$DEST" ]; then
    rm -rf "$DEST"
    warn "Replaced existing $DEST"
  fi
  mv "$TMP_DIR/$APP_NAME" "$DEST"

  # Strip the quarantine xattr Gatekeeper adds to anything downloaded via curl.
  # Without this, ad-hoc-signed bundles trigger a "developer cannot be verified"
  # dialog and won't open with a double-click. Stripping is the standard escape
  # hatch for open-source / unsigned tools.
  xattr -cr "$DEST" 2>/dev/null || true

  # Register with Launch Services so Spotlight, Launchpad, and `open -a` resolve
  # the freshly-unpacked bundle. Without this, LS only learns about the app the
  # first time Finder touches it.
  LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
  [ -x "$LSREGISTER" ] && "$LSREGISTER" -f "$DEST" >/dev/null 2>&1 || true

  success "Installed: $DEST"
else
  # Linux ships a .deb (Debian/Ubuntu). dpkg installs the binary to
  # /usr/bin/git-projects-manager and registers a .desktop launcher + icons,
  # so there's nothing to wire up by hand.
  command -v dpkg >/dev/null 2>&1 \
    || error "dpkg not found. The Linux release is a .deb (Debian/Ubuntu). On Arch, build locally with scripts/install_arch.sh instead."

  SUDO=""
  if [ "$(id -u)" -ne 0 ]; then
    command -v sudo >/dev/null 2>&1 || error "Need root to install the .deb, but sudo is not available. Re-run as root."
    SUDO="sudo"
  fi

  # `dpkg -i` fails if the package's runtime deps (webkit2gtk, etc.) are
  # missing; `apt-get install -f` then pulls them in and completes the install.
  if ! $SUDO dpkg -i "$TMP_DIR/$ARTIFACT" 2>/dev/null; then
    warn "Resolving dependencies"
    $SUDO apt-get install -f -y >/dev/null 2>&1 \
      || error "Failed to install dependencies. Try: sudo apt-get install -f"
  fi

  success "Installed: /usr/bin/$PROC"
fi

rm -rf "$TMP_DIR"

echo -e "\n${GREEN}═══ Git Projects Manager $VERSION installed ═══${NC}"
if [ "$OS" = "darwin" ]; then
  echo -e "  ${CYAN}Open from /Applications, Spotlight, or:  open -a 'Git Projects Manager'${NC}"
else
  echo -e "  ${CYAN}Launch from your app menu, or run:  $PROC${NC}"
fi
