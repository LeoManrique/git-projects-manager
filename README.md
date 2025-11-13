# Git Projects Manager

A cross-platform desktop application that helps developers monitor and manage multiple Git repositories across their file system.

## About

Git Projects Manager scans directories for Git repositories and provides a unified interface to track repository status at a glance. It identifies repositories with uncommitted changes, unpushed commits, and scanning errors across multiple folders in parallel, making it easy to stay on top of your Git workspace.

Built with **Wails** (Go + React), this application provides native performance with a modern web-based UI.

## Features

- **Multi-folder monitoring**: Add and manage multiple directories to scan for Git repositories
- **Parallel scanning**: Recursively scan directories and check repository status concurrently for fast results
- **Status tracking**: Identify repositories with:
  - Uncommitted changes (staging area and working directory)
  - Unpushed commits (local commits not yet pushed to remote)
  - Clean status (no changes, everything pushed)
  - Scanning errors (permission issues, invalid repos)
- **Persistent configuration**: Monitored folders are saved automatically and restored on app restart
- **Cross-platform**: Works on Windows, macOS, and Linux (including WSL)
- **Smart exclusions**: Automatically skips common directories (node_modules, venv, .git, build artifacts, etc.) to improve scanning speed

## How It Works

### Architecture

The application uses a **backend-frontend separation** pattern:

- **Backend (Go)**: Handles Git operations, file system scanning, and configuration management
  - `git_repository.go` - Git command execution and repository detection
  - `config_manager.go` - Persistent folder configuration
  - `exclude_patterns.go` - Optimized directory traversal with smart skipping

- **Frontend (React)**: Provides the user interface with two main sections:
  - **Scan Results** tab - View repository status and initiate scans
  - **Manage Folders** tab - Add, edit, and delete monitored folders

### Scanning Process

1. User selects a folder to scan or scans all monitored folders
2. Backend walks the directory tree recursively, skipping excluded patterns
3. For each Git repository found, status is checked in parallel using goroutines:
   - `git status --porcelain` - Check for uncommitted changes
   - `git log @{upstream}..HEAD` - Check for unpushed commits
4. Results are categorized and displayed with execution time
5. Color-coded indicators show repository status at a glance

## Live Development

To run in live development mode, run `wails dev` in the project directory. This will run a Vite development
server that will provide very fast hot reload of your frontend changes. If you want to develop in a browser
and have access your Go methods, there is also a dev server that runs on http://localhost:34115. Connect
to this in your browser, and you can call your Go code from devtools.

## Building

To build a redistributable, production mode package, use `wails build`.

You can configure the project by editing `wails.json`. More information about the project settings can be found
here: https://wails.io/docs/reference/project-config
