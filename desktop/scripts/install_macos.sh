#!/bin/bash
set -euo pipefail

APP_NAME="git-projects-manager"
DISPLAY_NAME="Git Projects Manager"
VERSION="2.0.0"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RELEASE_DIR="$PROJECT_DIR/src-tauri/target/release"
APP_BUNDLE="$RELEASE_DIR/bundle/macos/$DISPLAY_NAME.app"
INSTALL_PATH="/Applications/$DISPLAY_NAME.app"

if [ ! -d "$APP_BUNDLE" ]; then
    echo "App bundle not found at $APP_BUNDLE"
    echo "Run 'pnpm tauri build --bundles app' first."
    exit 1
fi

echo "Installing $DISPLAY_NAME v$VERSION to /Applications..."

if [ -d "$INSTALL_PATH" ]; then
    sudo rm -rf "$INSTALL_PATH"
fi

sudo cp -R "$APP_BUNDLE" "$INSTALL_PATH"

# Strip quarantine and ad-hoc re-sign so Gatekeeper doesn't flag the unsigned bundle
sudo xattr -dr com.apple.quarantine "$INSTALL_PATH" 2>/dev/null || true
sudo codesign --force --deep --sign - "$INSTALL_PATH" 2>/dev/null || true

echo "Installed successfully!"
echo "  App: $INSTALL_PATH"
echo "  Run: open -a '$DISPLAY_NAME'"
