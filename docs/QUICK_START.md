# Quick Start

## Run the App

```bash
npm install
npm run tauri:dev
```

## Commands

```bash
npm run tauri:dev      # Development mode
npm run tauri:build    # Production build
npm run lint           # Check code
npm run format         # Format code
```

## Prerequisites

- Node.js 16+
- Rust (latest stable)
- Git
- **Windows**: Visual Studio C++ Build Tools
- **macOS**: `xcode-select --install`
- **Linux**: See [SETUP.md](SETUP.md)

## Troubleshooting

If the app won't start:
1. Delete `node_modules` and run `npm install`
2. Run `cd src-tauri && cargo clean`

For detailed setup instructions, see [SETUP.md](SETUP.md).
For architecture details, see [DESIGN.md](DESIGN.md).
