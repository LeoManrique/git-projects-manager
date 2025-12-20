# Tauri v2 Setup Instructions

This document provides step-by-step instructions for setting up and running the Tauri v2 Git Projects Manager application.

## Project Structure

The complete project has been created with the following structure:

```
tauri-app/
├── src-tauri/                          # Rust Backend
│   ├── src/
│   │   ├── commands/                   # Tauri Command Handlers
│   │   │   ├── mod.rs
│   │   │   ├── folder.rs              # Folder management commands
│   │   │   └── scan.rs                # Repository scanning commands
│   │   ├── domain/                    # Business Logic Layer
│   │   │   ├── mod.rs                 # Domain models (MonitoredFolder, RepoStatus, etc.)
│   │   │   ├── config.rs              # Configuration management
│   │   │   ├── repository.rs          # Repository models
│   │   │   └── scanner.rs             # Parallel repository scanner
│   │   ├── infrastructure/            # External Integrations
│   │   │   ├── mod.rs
│   │   │   ├── git.rs                 # Git operations using git2
│   │   │   ├── storage.rs             # File persistence
│   │   │   └── exclude.rs             # Directory exclusion patterns
│   │   ├── main.rs                    # Application entry point
│   │   └── state.rs                   # Application state management
│   ├── Cargo.toml                     # Rust dependencies
│   ├── build.rs                       # Build script
│   └── tauri.conf.json                # Tauri configuration
│
├── src/                               # React Frontend
│   ├── components/                    # React components (placeholder)
│   ├── lib/
│   │   └── api.ts                     # Tauri API wrapper
│   ├── types/
│   │   └── index.ts                   # TypeScript type definitions
│   ├── App.tsx                        # Main application component
│   ├── App.css                        # Application styles
│   ├── main.tsx                       # React entry point
│   └── index.css                      # Global styles
│
├── package.json                       # Node.js dependencies and scripts
├── tsconfig.json                      # TypeScript configuration
├── tsconfig.node.json                 # TypeScript config for Node
├── vite.config.ts                     # Vite build configuration
├── index.html                         # HTML entry point
├── .eslintrc.cjs                      # ESLint configuration
├── .prettierrc                        # Prettier configuration
├── .gitignore                         # Git ignore rules
└── README.md                          # Project documentation
```

## Prerequisites

Before you begin, ensure you have the following installed:

### Required Tools

1. **Node.js** (v16 or later)
   - Download from: https://nodejs.org/
   - Verify installation: `node --version`

2. **Rust** (latest stable)
   - Install from: https://rustup.rs/
   - On Windows: `rustup-init.exe`
   - On macOS/Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   - Verify installation: `rustc --version`

3. **Git**
   - Download from: https://git-scm.com/
   - Verify installation: `git --version`

### Platform-Specific Requirements

#### Windows
- **Microsoft Visual Studio C++ Build Tools**
  - Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/
  - OR install via: https://aka.ms/vs/17/release/vs_BuildTools.exe
  - Select "Desktop development with C++" workload

#### macOS
- **Xcode Command Line Tools**
  ```bash
  xcode-select --install
  ```

#### Linux (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

For other Linux distributions, see: https://tauri.app/v2/guides/prerequisites/

## Installation Steps

1. **Navigate to the project directory:**
   ```bash
   cd tauri-app
   ```

2. **Install Node.js dependencies:**
   ```bash
   npm install
   ```

   This will install all required frontend dependencies including:
   - React and React DOM
   - Tauri API packages
   - TypeScript
   - Vite
   - Development tools (ESLint, Prettier, etc.)

3. **Verify Rust dependencies:**
   The Rust dependencies will be downloaded automatically when you first run the app.
   You can pre-download them with:
   ```bash
   cd src-tauri
   cargo fetch
   cd ..
   ```

## Running the Application

### Development Mode

To run the application in development mode with hot-reload:

```bash
npm run tauri:dev
```

This will:
- Start the Vite development server
- Compile the Rust backend
- Launch the application window
- Enable hot-reload for both frontend and backend changes
- Open DevTools automatically (in debug builds)

### Production Build

To build the application for production:

```bash
npm run tauri:build
```

The built application will be in:
- Windows: `src-tauri/target/release/bundle/`
- macOS: `src-tauri/target/release/bundle/`
- Linux: `src-tauri/target/release/bundle/`

### Platform-Specific Builds

```bash
# Build for Windows
npm run tauri:build:windows

# Build for macOS (universal binary)
npm run tauri:build:mac

# Build for Linux
npm run tauri:build:linux
```

## Available Commands

### Development
- `npm run dev` - Start Vite dev server only
- `npm run tauri:dev` - Start full Tauri development mode
- `npm run tauri` - Run Tauri CLI commands

### Building
- `npm run build` - Build frontend only (TypeScript + Vite)
- `npm run tauri:build` - Build complete application
- `npm run preview` - Preview production build

### Testing & Linting
- `npm run test` - Run tests (Vitest)
- `npm run test:ui` - Run tests with UI
- `npm run lint` - Run ESLint
- `npm run format` - Format code with Prettier

## Architecture Overview

### Backend (Rust)

The backend follows a domain-driven architecture:

1. **Commands Layer** (`src-tauri/src/commands/`)
   - Exposes Tauri commands to the frontend
   - Handles folder management (add, update, delete, list)
   - Handles repository scanning operations

2. **Domain Layer** (`src-tauri/src/domain/`)
   - Core business logic
   - Data models (MonitoredFolder, RepoStatus, ScanResult)
   - Configuration management
   - Parallel repository scanner using Rayon

3. **Infrastructure Layer** (`src-tauri/src/infrastructure/`)
   - Git operations using git2 library
   - File system operations
   - Directory exclusion patterns

### Frontend (React/TypeScript)

- **API Wrapper** (`src/lib/api.ts`) - Type-safe wrapper around Tauri commands
- **Type Definitions** (`src/types/index.ts`) - Shared TypeScript interfaces
- **Components** (`src/components/`) - React components (to be populated)
- **App Component** (`src/App.tsx`) - Main application UI

## Key Features Implemented

1. **Folder Management**
   - Add/remove monitored folders
   - Persistent configuration storage
   - Folder browsing dialog

2. **Repository Scanning**
   - Parallel scanning using Rayon
   - Detects pending changes
   - Detects unpushed commits
   - Smart directory exclusions (node_modules, dist, etc.)
   - Cancelable scans

3. **Performance**
   - Multi-threaded Rust backend
   - Parallel repository processing
   - Efficient Git operations

## Configuration

The application stores its configuration in:
- Windows: `%APPDATA%\.git-projects-manager\config.json`
- macOS: `~/Library/Application Support/.git-projects-manager/config.json`
- Linux: `~/.config/.git-projects-manager/config.json`

## Troubleshooting

### "Command not found: tauri"
Ensure you've run `npm install` to install the Tauri CLI.

### Rust compilation errors
1. Ensure you have the latest Rust: `rustup update`
2. Clear the build cache: `cd src-tauri && cargo clean`

### Frontend build errors
1. Delete `node_modules` and `package-lock.json`
2. Run `npm install` again

### Windows: "link.exe not found"
Install Visual Studio C++ Build Tools (see Prerequisites above).

### Linux: WebKit errors
Install the required system dependencies (see Linux prerequisites above).

## Next Steps

1. **Install Dependencies**
   ```bash
   npm install
   ```

2. **Run in Development Mode**
   ```bash
   npm run tauri:dev
   ```

3. **Start Development**
   - Modify React components in `src/`
   - Update Rust backend in `src-tauri/src/`
   - Both will hot-reload automatically

## Additional Resources

- [Tauri Documentation](https://tauri.app/v2/)
- [Tauri API Reference](https://tauri.app/v2/api/js/)
- [React Documentation](https://react.dev/)
- [Vite Documentation](https://vitejs.dev/)
- [Rust Documentation](https://doc.rust-lang.org/)

## Support

For issues or questions:
1. Check the Tauri documentation
2. Review the migration plan document
3. Check existing issues in the repository
