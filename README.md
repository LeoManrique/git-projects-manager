# Git Projects Manager - Tauri v2

A desktop application built with Tauri v2, React, and TypeScript for managing and monitoring Git repositories.

## Features

- Monitor multiple folders for Git repositories
- Scan repositories for pending changes and unpushed commits
- Fast parallel scanning using Rust
- Cross-platform support (Windows, macOS, Linux)

## Project Structure

```
tauri-app/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri command handlers
│   │   ├── domain/         # Business logic
│   │   ├── infrastructure/ # External integrations
│   │   ├── main.rs         # Entry point
│   │   └── state.rs        # Application state
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
├── src/                    # React frontend
│   ├── components/         # React components
│   ├── lib/                # API wrapper
│   ├── types/              # TypeScript types
│   ├── App.tsx             # Main app component
│   └── main.tsx            # Entry point
├── package.json            # Node dependencies
└── vite.config.ts          # Vite configuration
```

## Prerequisites

- [Node.js](https://nodejs.org/) (v16 or later)
- [Rust](https://www.rust-lang.org/tools/install)
- [Git](https://git-scm.com/)

### Platform-specific requirements:

**Windows:**
- Microsoft Visual Studio C++ Build Tools

**macOS:**
- Xcode Command Line Tools

**Linux:**
- See [Tauri prerequisites](https://tauri.app/v2/guides/prerequisites/)

## Development

1. Install dependencies:
```bash
npm install
```

2. Run in development mode:
```bash
npm run tauri:dev
```

## Building

Build for your current platform:
```bash
npm run tauri:build
```

Platform-specific builds:
```bash
# Windows
npm run tauri:build:windows

# macOS
npm run tauri:build:mac

# Linux
npm run tauri:build:linux
```

## Architecture

The application follows a domain-driven architecture:

- **Commands Layer**: Tauri command handlers that expose functionality to the frontend
- **Domain Layer**: Core business logic and models
- **Infrastructure Layer**: Git operations, file system access, and external integrations

## Technology Stack

### Backend (Rust)
- Tauri 2.0 - Desktop application framework
- git2 - Git operations
- rayon - Parallel processing
- serde - Serialization/deserialization
- tokio - Async runtime

### Frontend (React/TypeScript)
- React 18 - UI framework
- TypeScript - Type safety
- Vite - Build tool and dev server
- Tauri API - Native functionality access

## License

MIT
