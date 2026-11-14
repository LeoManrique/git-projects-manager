#!/usr/bin/env bash
set -euo pipefail

# Installs Git Projects Manager from a *local* build for the host platform:
#   macOS → the native SwiftUI app (macos/) at /Applications/Git Projects Manager.app
#           (requires macOS 26+)
#   Linux → the Tauri app (desktop/): binary, .desktop entry and icons under
#           /usr/local (any distro; written for Arch, where no release .deb fits)
#
# Unlike install_release.sh (which downloads a published artifact), this builds
# the app on this machine and installs that build. Pass --no-build to skip the
# build and install the existing one.
#
#   scripts/install_local.sh [--no-build]
#
# Windows has its own counterpart: scripts/install_windows.py.

# ── Colors ──
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

TOTAL_STEPS=4
step()    { echo -e "\n${BLUE}[$1/$TOTAL_STEPS]${NC} ${CYAN}$2${NC}"; }
success() { echo -e "  ${GREEN}✓ $1${NC}"; }
warn()    { echo -e "  ${YELLOW}⚠ $1${NC}"; }
error()   { echo -e "  ${RED}✗ $1${NC}"; exit 1; }

USAGE="usage: install_local.sh [--no-build]"
BUILD=1
for arg in "$@"; do
    case "$arg" in
        --no-build) BUILD=0 ;;
        -h|--help) echo "$USAGE"; exit 0 ;;
        *) error "Unknown argument: $arg ($USAGE)" ;;
    esac
done

# ── Paths & identity ──
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
DESKTOP_DIR="$REPO_ROOT/desktop"
MACOS_DIR="$REPO_ROOT/macos"
PACKAGE_JSON="$DESKTOP_DIR/package.json"

APP_NAME="Git Projects Manager"   # macOS bundle name / Linux display name
BINARY_NAME="git-projects-manager" # Tauri executable (Cargo package name)

# macOS: Release build produced by xcodebuild, installed to /Applications.
MACOS_BUNDLE="$MACOS_DIR/DerivedData/Build/Products/Release/$APP_NAME.app"
MACOS_DEST="/Applications/$APP_NAME.app"

# Linux: raw release binary from `tauri build --no-bundle` (frontend embedded).
LINUX_BINARY="$DESKTOP_DIR/src-tauri/target/release/$BINARY_NAME"
ICONS_DIR="$DESKTOP_DIR/src-tauri/icons"
PREFIX="/usr/local"

# package.json is the single source of truth for the version (deploy_releases.sh
# bumps the macOS project.yml to match). First top-level "version": "x.y.z" line.
VERSION=$(sed -n 's/.*"version": "\([0-9][0-9.]*\)".*/\1/p' "$PACKAGE_JSON" | head -1)
[ -n "$VERSION" ] || error "Could not read version from $PACKAGE_JSON"

# require <cmd>... — abort if any build tool is missing.
require() {
    for cmd in "$@"; do
        command -v "$cmd" &>/dev/null || error "$cmd is not installed"
    done
}

# stop_matching <pgrep match flag> <pattern> — graceful TERM, then KILL.
stop_matching() {
    if ! pgrep "$1" "$2" >/dev/null 2>&1; then
        success "No running instances"
        return 0
    fi
    pkill -TERM "$1" "$2" 2>/dev/null || true
    for _ in $(seq 1 16); do
        pgrep "$1" "$2" >/dev/null 2>&1 || break
        sleep 0.5
    done
    if pgrep "$1" "$2" >/dev/null 2>&1; then
        warn "Force-killing (graceful stop timed out)"
        pkill -KILL "$1" "$2" 2>/dev/null || true
    fi
    success "Stopped running app"
}

# ── macOS ──

build_macos() {
    require cargo xcodegen xcodebuild
    # Same sequence as deploy_releases.sh / `just build-macos`: build the Rust
    # FFI staticlib + Swift bindings, regenerate the Xcode project, then build.
    "$MACOS_DIR/scripts/build-rust.sh" release
    (cd "$MACOS_DIR" && xcodegen generate)
    (cd "$MACOS_DIR" && xcodebuild -project GitProjectsManager.xcodeproj \
        -scheme GitProjectsManager -configuration Release \
        -derivedDataPath DerivedData build)
}

install_macos() {
    if [ -d "$MACOS_DEST" ]; then
        rm -rf "$MACOS_DEST"
        warn "Replaced existing $MACOS_DEST"
    fi
    # cp -R (not mv) so the DerivedData build stays intact for the next install.
    cp -R "$MACOS_BUNDLE" "$MACOS_DEST"

    # Strip the quarantine xattr and ad-hoc re-sign so Gatekeeper doesn't flag
    # the unsigned locally-built bundle with a "developer cannot be verified" dialog.
    xattr -cr "$MACOS_DEST" 2>/dev/null || true
    codesign --force --deep --sign - "$MACOS_DEST" 2>/dev/null || true

    # Register with Launch Services so Spotlight, Launchpad, and `open -a`
    # resolve the freshly-copied bundle without waiting for Finder to touch it.
    local lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
    [ -x "$lsregister" ] && "$lsregister" -f "$MACOS_DEST" >/dev/null 2>&1 || true
    success "Installed: $MACOS_DEST"
}

# ── Linux ──

build_linux() {
    require pnpm cargo
    # Only the binary is installed, so skip every bundle format in tauri.conf.json
    # (deb/rpm/appimage are slow and the last two can fail on Arch).
    # beforeBuildCommand still runs, and the frontend is embedded in the binary.
    (cd "$DESKTOP_DIR" && pnpm tauri build --no-bundle)
}

install_linux() {
    local sudo=""
    if [ "$(id -u)" -ne 0 ]; then
        command -v sudo &>/dev/null || error "Need root to write to $PREFIX, but sudo is not available. Re-run as root."
        sudo="sudo"
    fi

    local share="$PREFIX/share"
    $sudo install -Dm755 "$LINUX_BINARY" "$PREFIX/bin/$BINARY_NAME"

    # hicolor icons, named after the binary so Icon= in the .desktop entry finds them.
    local size
    for size in 32 64 128; do
        $sudo install -Dm644 "$ICONS_DIR/${size}x${size}.png" \
            "$share/icons/hicolor/${size}x${size}/apps/$BINARY_NAME.png"
    done
    $sudo install -Dm644 "$ICONS_DIR/128x128@2x.png" "$share/icons/hicolor/256x256/apps/$BINARY_NAME.png"
    $sudo install -Dm644 "$ICONS_DIR/icon.png" "$share/icons/hicolor/512x512/apps/$BINARY_NAME.png"

    # Mirrors the entry Tauri's deb bundler writes. The filename and
    # StartupWMClass match the binary name, which GTK uses as the Wayland app_id
    # and X11 WM_CLASS, so docks and taskbars pair the window with this icon.
    $sudo install -dm755 "$share/applications"
    $sudo tee "$share/applications/$BINARY_NAME.desktop" >/dev/null <<DESKTOP
[Desktop Entry]
Name=$APP_NAME
Comment=Manage your git repositories
Exec=$BINARY_NAME
StartupWMClass=$BINARY_NAME
Icon=$BINARY_NAME
Type=Application
Categories=Development;Utility;
Terminal=false
DESKTOP

    # /usr/local/share/icons/hicolor has no index.theme, so the cache needs
    # --ignore-theme-index (-t) or gtk-update-icon-cache refuses to write it.
    if command -v gtk-update-icon-cache &>/dev/null; then
        $sudo gtk-update-icon-cache -f -t "$share/icons/hicolor" >/dev/null 2>&1 || true
    fi
    if command -v update-desktop-database &>/dev/null; then
        $sudo update-desktop-database "$share/applications" >/dev/null 2>&1 || true
    fi
    success "Installed: $PREFIX/bin/$BINARY_NAME"
    success "Launcher:  $share/applications/$BINARY_NAME.desktop"
}

# ── Step 1: Check platform ──
step 1 "Checking platform"
case "$(uname -s)" in
    Darwin)
        PLATFORM=macos
        # The native SwiftUI app targets macOS 26+; refuse early on older systems.
        MACOS_VERSION=$(sw_vers -productVersion)
        [ "${MACOS_VERSION%%.*}" -ge 26 ] 2>/dev/null \
          || error "The macOS app requires macOS 26 or later (this Mac runs $MACOS_VERSION)"
        success "macOS $MACOS_VERSION"
        ;;
    Linux)
        PLATFORM=linux
        success "Linux ($(uname -m))"
        ;;
    *) error "This installer runs on macOS and Linux (on Windows use scripts/install_windows.py)" ;;
esac

# ── Step 2: Build ──
step 2 "Building $APP_NAME v$VERSION"
if [ "$BUILD" -eq 1 ]; then
    "build_$PLATFORM"
    success "Build complete"
else
    success "Skipped (--no-build; using the existing build)"
fi

if [ "$PLATFORM" = macos ]; then
    [ -d "$MACOS_BUNDLE" ] || error "App bundle not found at $MACOS_BUNDLE. Re-run without --no-build."
else
    [ -x "$LINUX_BINARY" ] || error "Binary not found at $LINUX_BINARY. Re-run without --no-build."
fi

# ── Step 3: Stop any running instance ──
# A running app holds its binary open; replacing it underneath leaves a
# half-old/half-new install until the next launch. Kill it first.
step 3 "Stopping running instance"
if [ "$PLATFORM" = macos ]; then
    stop_matching -f "/Contents/MacOS/$APP_NAME"
else
    # Match argv[0] with -f, not the process name with -x: Linux truncates
    # process names to 15 characters, so "git-projects-manager" never matches -x.
    stop_matching -f "^([^ ]*/)?$BINARY_NAME( |\$)"
fi

# ── Step 4: Install ──
step 4 "Installing $APP_NAME v$VERSION"
"install_$PLATFORM"

echo -e "\n${GREEN}═══ $APP_NAME $VERSION installed ═══${NC}"
if [ "$PLATFORM" = macos ]; then
    echo -e "  ${CYAN}Open from /Applications, Spotlight, or:  open -a '$APP_NAME'${NC}"
else
    echo -e "  ${CYAN}Launch from your app menu, or run:  $BINARY_NAME${NC}"
fi
