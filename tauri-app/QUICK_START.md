# Quick Start Guide

## Get Up and Running in 3 Steps

### 1. Install Dependencies
```bash
cd tauri-app
npm install
```

### 2. Run the App
```bash
npm run tauri:dev
```

That's it! The application will compile and launch.

## What You Get

A complete Tauri v2 application with:
- Rust backend with domain-driven architecture
- React + TypeScript frontend
- Git repository scanning functionality
- Folder management system
- Production-ready build configuration

## Quick Commands

```bash
# Development
npm run tauri:dev          # Run in dev mode

# Building
npm run tauri:build        # Build for production

# Code Quality
npm run lint               # Check code
npm run format             # Format code
```

## Project Structure at a Glance

```
tauri-app/
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── commands/      # API endpoints
│   │   ├── domain/        # Business logic
│   │   └── infrastructure/# Git & file operations
│   └── Cargo.toml
│
├── src/                   # React frontend
│   ├── lib/api.ts        # Backend API wrapper
│   ├── types/            # TypeScript types
│   └── App.tsx           # Main component
│
└── package.json
```

## Prerequisites

- Node.js (v16+)
- Rust (latest stable)
- Git

**Windows:** Visual Studio C++ Build Tools
**macOS:** Xcode Command Line Tools
**Linux:** See SETUP.md

## Need More Details?

- Full setup instructions: See `SETUP.md`
- Architecture details: See `README.md`
- Migration plan: See `../TAURI_FULL_MIGRATION_PLAN.md`

## Troubleshooting

**App won't start?**
1. Check you have all prerequisites installed
2. Try: `cd src-tauri && cargo clean && cd ..`
3. Delete `node_modules` and run `npm install` again

**Build errors?**
- Windows: Install Visual Studio C++ Build Tools
- macOS: Run `xcode-select --install`
- Linux: Install webkit2gtk (see SETUP.md)

## What's Next?

The basic structure is ready. You can:
1. Customize the UI in `src/App.tsx`
2. Add more commands in `src-tauri/src/commands/`
3. Extend business logic in `src-tauri/src/domain/`
4. Add more features based on the migration plan

Happy coding!
