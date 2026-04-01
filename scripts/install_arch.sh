#!/bin/bash
set -euo pipefail

APP_NAME="git-projects-manager"
DISPLAY_NAME="Git Projects Manager"
BINARY_NAME="git-projects-manager"
VERSION="2.0.0"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RELEASE_DIR="$PROJECT_DIR/src-tauri/target/release"
ICONS_DIR="$PROJECT_DIR/src-tauri/icons"
BINARY="$RELEASE_DIR/$BINARY_NAME"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found at $BINARY"
    echo "Run 'pnpm tauri build' first."
    exit 1
fi

echo "Installing $DISPLAY_NAME v$VERSION..."

# Install binary
sudo install -Dm755 "$BINARY" "/usr/local/bin/$BINARY_NAME"

# Install icons
for size in 32 64 128; do
    icon="$ICONS_DIR/${size}x${size}.png"
    if [ -f "$icon" ]; then
        sudo install -Dm644 "$icon" "/usr/local/share/icons/hicolor/${size}x${size}/apps/$APP_NAME.png"
    fi
done

# 256x256 from the @2x icon
if [ -f "$ICONS_DIR/128x128@2x.png" ]; then
    sudo install -Dm644 "$ICONS_DIR/128x128@2x.png" "/usr/local/share/icons/hicolor/256x256/apps/$APP_NAME.png"
fi

# Install desktop entry
sudo mkdir -p /usr/local/share/applications
sudo tee "/usr/local/share/applications/$APP_NAME.desktop" > /dev/null <<DESKTOP
[Desktop Entry]
Name=$DISPLAY_NAME
Exec=$BINARY_NAME
Icon=$APP_NAME
Type=Application
Categories=Development;Utility;
Comment=Manage your git repositories
Terminal=false
StartupWMClass=$BINARY_NAME
DESKTOP

# Update caches
if command -v gtk-update-icon-cache &>/dev/null; then
    sudo gtk-update-icon-cache -f /usr/local/share/icons/hicolor/ 2>/dev/null || true
fi
if command -v update-desktop-database &>/dev/null; then
    sudo update-desktop-database /usr/local/share/applications/ 2>/dev/null || true
fi
if command -v xdg-desktop-menu &>/dev/null; then
    xdg-desktop-menu forceupdate 2>/dev/null || true
fi

echo "Installed successfully!"
echo "  Binary: /usr/local/bin/$BINARY_NAME"
echo "  Desktop: /usr/local/share/applications/$APP_NAME.desktop"
