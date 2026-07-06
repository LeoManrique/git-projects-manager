# git-projects-manager — task runner
# Monorepo: core/ (shared Rust), desktop/ (Tauri + React, Win/Linux),
# macos/ (SwiftUI + UniFFI), server/ (axum + SQLite)

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

# Build and launch the native macOS app (Debug)
dev-macos: macos-project
    cd macos && xcodebuild -project GitProjectsManager.xcodeproj \
        -scheme GitProjectsManager -configuration Debug \
        -derivedDataPath DerivedData build
    open "macos/DerivedData/Build/Products/Debug/Git Projects Manager.app"

# --- Build ---

# Build everything for release (desktop + server + macOS app)
build: build-desktop build-server build-macos

# Build the desktop app bundle for the current platform
build-desktop:
    cd desktop && pnpm tauri:build

# Build the sync server in release mode
build-server:
    cd server && cargo build --release

# Regenerate the Xcode project (needs the Rust bindings to exist first)
macos-project:
    macos/scripts/build-rust.sh debug
    cd macos && xcodegen generate

# Build the native macOS app (Release)
build-macos: macos-project
    cd macos && xcodebuild -project GitProjectsManager.xcodeproj \
        -scheme GitProjectsManager -configuration Release \
        -derivedDataPath DerivedData build

# --- Quality ---

# Run all Rust tests
test:
    cd core && cargo test
    cd macos/ffi && cargo test

# Clippy pedantic across all Rust crates (CLAUDE.md requirement)
clippy:
    cd core && cargo clippy --all-targets -- -W clippy::pedantic
    cd desktop/src-tauri && cargo clippy --all-targets -- -W clippy::pedantic
    cd macos/ffi && cargo clippy --all-targets -- -W clippy::pedantic
