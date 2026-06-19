# git-projects-manager — task runner
# Monorepo: desktop/ (Tauri + React) and server/ (axum + SQLite)

# List available recipes
default:
    @just --list

# --- Install dependencies ---

# Install all dependencies (desktop + server)
install: install-desktop install-server

# Install desktop frontend dependencies (pnpm)
install-desktop:
    cd desktop && pnpm install

# Fetch server (Rust) dependencies
install-server:
    cd server && cargo fetch

# --- Dev ---

# Run the desktop app in dev mode (Tauri + Vite)
dev:
    cd desktop && pnpm tauri:dev

# Run only the Vite frontend in dev mode (no Tauri shell)
dev-web:
    cd desktop && pnpm dev

# Run the sync server in dev mode
dev-server:
    cd server && cargo run

# --- Build ---

# Build everything for release (desktop + server)
build: build-desktop build-server

# Build the desktop app bundle for the current platform
build-desktop:
    cd desktop && pnpm tauri:build

# Build the sync server in release mode
build-server:
    cd server && cargo build --release
