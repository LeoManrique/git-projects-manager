# Setup Guide

## Prerequisites

### Required Tools

1. **Node.js** (v16+) - https://nodejs.org/
2. **Rust** (latest stable) - https://rustup.rs/
3. **Git** - https://git-scm.com/

### Platform-Specific

#### Windows
Install [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "Desktop development with C++" workload.

#### macOS
```bash
xcode-select --install
```

#### Linux (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

For other distributions, see [Tauri prerequisites](https://tauri.app/start/prerequisites/).

## Configuration

The app stores data in:
- **macOS**: `~/Library/Application Support/.git-projects-manager/`
- **Linux**: `~/.config/.git-projects-manager/`
- **Windows**: `%APPDATA%\.git-projects-manager\`

Files:
- `config.json` - Monitored folders and settings
- `kanban.json` - Kanban board state

## Build Targets

```bash
npm run tauri:build           # Current platform
npm run tauri:build:windows   # Windows (MSI, NSIS)
npm run tauri:build:mac       # macOS (DMG, APP)
npm run tauri:build:linux     # Linux (DEB, RPM, AppImage)
```

Output: `src-tauri/target/release/bundle/`

## Troubleshooting

### "Command not found: tauri"
Run `npm install` first.

### Rust compilation errors
```bash
rustup update
cd src-tauri && cargo clean
```

### Windows: "link.exe not found"
Install Visual Studio C++ Build Tools.

### Linux: WebKit errors
Install the system dependencies listed above.
