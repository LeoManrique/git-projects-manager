#!/usr/bin/env bash
set -euo pipefail

# Installs the native SwiftUI Git Projects Manager (macos/) from a *local* build
# to /Applications/Git Projects Manager.app (requires macOS 26+).
#
# Unlike install_release.sh (which downloads a published .zip), this installs a
# bundle you build on this machine. Pass --build to (re)build it first; without
# --build it installs the existing Release build under macos/DerivedData.
#
#   scripts/install_macos.sh --build

# ── Colors ──
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

TOTAL_STEPS=4
step()    { echo -e "\n${BLUE}[$1/$TOTAL_STEPS]${NC} ${CYAN}$2${NC}"; }
success() { echo -e "  ${GREEN}✓ $1${NC}"; }
warn()    { echo -e "  ${YELLOW}⚠ $1${NC}"; }
error()   { echo -e "  ${RED}✗ $1${NC}"; exit 1; }

BUILD=0
for arg in "$@"; do
    case "$arg" in
        --build) BUILD=1 ;;
        *) error "Unknown argument: $arg (usage: install_macos.sh [--build])" ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS_DIR="$(dirname "$SCRIPT_DIR")/macos"
APP_NAME="Git Projects Manager"
APP_BUNDLE="$MACOS_DIR/DerivedData/Build/Products/Release/$APP_NAME.app"
INSTALL_PATH="/Applications/$APP_NAME.app"

# ── Step 1: Check platform ──
# The native SwiftUI app targets macOS 26+; refuse early on older systems.
step 1 "Checking platform"
[ "$(uname -s)" = "Darwin" ] || error "This installer only runs on macOS"
MACOS_VERSION=$(sw_vers -productVersion)
[ "${MACOS_VERSION%%.*}" -ge 26 ] 2>/dev/null \
  || error "The macOS app requires macOS 26 or later (this Mac runs $MACOS_VERSION)"
success "macOS $MACOS_VERSION"

# ── Step 2: Build ──
step 2 "Building $APP_NAME"
if [ "$BUILD" -eq 1 ]; then
    # Same sequence as deploy_releases.sh / `just build-macos`: build the Rust
    # FFI staticlib + Swift bindings, regenerate the Xcode project, then build.
    "$MACOS_DIR/scripts/build-rust.sh" release
    (cd "$MACOS_DIR" && xcodegen generate)
    (cd "$MACOS_DIR" && xcodebuild -project GitProjectsManager.xcodeproj \
        -scheme GitProjectsManager -configuration Release \
        -derivedDataPath DerivedData build)
    success "Build complete"
else
    success "Skipped (using existing build; pass --build to rebuild)"
fi

[ -d "$APP_BUNDLE" ] \
  || error "App bundle not found at $APP_BUNDLE. Re-run with --build."

# ── Step 3: Stop any running instance ──
# A running app holds a file lock on its binary; replacing the bundle underneath
# it leaves a half-old/half-new install until the next launch. Kill it first.
step 3 "Stopping running instance"
if pgrep -f "/Contents/MacOS/$APP_NAME" >/dev/null 2>&1; then
    pkill -TERM -f "/Contents/MacOS/$APP_NAME" 2>/dev/null || true
    for _ in $(seq 1 16); do
        pgrep -f "/Contents/MacOS/$APP_NAME" >/dev/null 2>&1 || break
        sleep 0.5
    done
    if pgrep -f "/Contents/MacOS/$APP_NAME" >/dev/null 2>&1; then
        warn "Force-killing (graceful stop timed out)"
        pkill -KILL -f "/Contents/MacOS/$APP_NAME" 2>/dev/null || true
    fi
    success "Stopped running app"
else
    success "No running instances"
fi

# ── Step 4: Install ──
step 4 "Installing to /Applications"
if [ -d "$INSTALL_PATH" ]; then
    rm -rf "$INSTALL_PATH"
    warn "Replaced existing $INSTALL_PATH"
fi
# cp -R (not mv) so the DerivedData build stays intact for the next install.
cp -R "$APP_BUNDLE" "$INSTALL_PATH"

# Strip the quarantine xattr and ad-hoc re-sign so Gatekeeper doesn't flag the
# unsigned locally-built bundle with a "developer cannot be verified" dialog.
xattr -cr "$INSTALL_PATH" 2>/dev/null || true
codesign --force --deep --sign - "$INSTALL_PATH" 2>/dev/null || true

# Register with Launch Services so Spotlight, Launchpad, and `open -a` resolve
# the freshly-copied bundle without waiting for Finder to touch it.
LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
[ -x "$LSREGISTER" ] && "$LSREGISTER" -f "$INSTALL_PATH" >/dev/null 2>&1 || true
success "Installed: $INSTALL_PATH"

echo -e "\n${GREEN}═══ $APP_NAME installed ═══${NC}"
echo -e "  ${CYAN}Open from /Applications, Spotlight, or:  open -a '$APP_NAME'${NC}"
