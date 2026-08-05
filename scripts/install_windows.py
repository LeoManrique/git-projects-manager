#!/usr/bin/env python3
"""Installs the Tauri Git Projects Manager (desktop/) from a *local* build.

Windows counterpart to install_arch.sh / install_macos.sh: it installs a bundle
you build on this machine, not a published release artifact (install_release.sh
downloads those, and only covers macOS and Linux).

Pass --build to (re)build first; without it, this installs the NSIS installer
already sitting in desktop/src-tauri/target/release/bundle/nsis/ whose version
matches desktop/package.json.

    python scripts/install_windows.py --build

The NSIS installer is the same bundle deploy_releases.sh ships for Windows, so
it registers the Start menu entry, the uninstaller and the WebView2 runtime
check for us — nothing is copied by hand here.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import NoReturn

# Windows-only stdlib module; guarded so a run on another OS reaches the
# platform check in step 1 instead of dying on an ImportError. Annotations
# referring to it stay safe thanks to `from __future__ import annotations`.
if sys.platform == "win32":
    import winreg


# ── Console setup ──
def _init_console() -> tuple[bool, bool]:
    """Enable ANSI escapes and UTF-8 output. Returns (color_ok, utf8_ok)."""
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, OSError):
        pass
    utf8 = (sys.stdout.encoding or "").lower().replace("-", "").startswith("utf")

    if os.environ.get("NO_COLOR") or not sys.stdout.isatty():
        return False, utf8
    # Windows Terminal and git-bash already interpret escapes; legacy conhost
    # needs ENABLE_VIRTUAL_TERMINAL_PROCESSING (0x0004) turned on explicitly.
    try:
        import ctypes

        kernel32 = ctypes.windll.kernel32
        handle = kernel32.GetStdHandle(-11)  # STD_OUTPUT_HANDLE
        mode = ctypes.c_uint32()
        if not kernel32.GetConsoleMode(handle, ctypes.byref(mode)):
            return False, utf8
        return bool(kernel32.SetConsoleMode(handle, mode.value | 0x0004)), utf8
    except (AttributeError, OSError):
        return False, utf8


_COLOR, _UTF8 = _init_console()

RED, GREEN, YELLOW, BLUE, CYAN, NC = (
    ("\033[0;31m", "\033[0;32m", "\033[1;33m", "\033[0;34m", "\033[0;36m", "\033[0m")
    if _COLOR
    else ("", "", "", "", "", "")
)
MARK_OK, MARK_WARN, MARK_ERR = ("✓", "⚠", "✗") if _UTF8 else ("[ok]", "[!]", "[x]")
RULE = "═══" if _UTF8 else "==="

TOTAL_STEPS = 4


def step(number: int, message: str) -> None:
    print(f"\n{BLUE}[{number}/{TOTAL_STEPS}]{NC} {CYAN}{message}{NC}")


def success(message: str) -> None:
    print(f"  {GREEN}{MARK_OK} {message}{NC}")


def warn(message: str) -> None:
    print(f"  {YELLOW}{MARK_WARN} {message}{NC}")


def error(message: str) -> NoReturn:
    print(f"  {RED}{MARK_ERR} {message}{NC}")
    sys.exit(1)


# ── Paths & identity ──
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
DESKTOP_DIR = REPO_ROOT / "desktop"
PACKAGE_JSON = DESKTOP_DIR / "package.json"
NSIS_DIR = DESKTOP_DIR / "src-tauri" / "target" / "release" / "bundle" / "nsis"

APP_NAME = "Git Projects Manager"  # tauri.conf.json productName
BINARY_NAME = "git-projects-manager.exe"  # Cargo package name
IDENTIFIER = "com.gitprojectsmanager.app"  # tauri.conf.json identifier
UNINSTALL_KEY = rf"Software\Microsoft\Windows\CurrentVersion\Uninstall\{IDENTIFIER}"


def run(command: list[str], cwd: Path, what: str) -> None:
    """Run a subcommand, aborting the script if it exits non-zero."""
    result = subprocess.run(command, cwd=cwd, check=False)
    if result.returncode != 0:
        error(f"{what} failed (exit {result.returncode})")


def app_version() -> str:
    """Read the version from package.json — the single source of truth."""
    try:
        version = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))["version"]
    except (OSError, ValueError, KeyError) as exc:
        error(f"Could not read version from {PACKAGE_JSON}: {exc}")
    return str(version)


def find_installer(version: str) -> Path:
    """Locate the NSIS installer built for `version`.

    Matching on the version keeps a stale installer from an older build (Tauri
    puts the version in the filename) from being installed silently.
    """
    matches = sorted(NSIS_DIR.glob(f"*_{version}_*-setup.exe"))
    if matches:
        return matches[-1]

    others = sorted(NSIS_DIR.glob("*-setup.exe")) if NSIS_DIR.is_dir() else []
    if others:
        names = ", ".join(path.name for path in others)
        error(
            f"No installer for v{version} in {NSIS_DIR}. Found: {names}. "
            "Re-run with --build."
        )
    error(f"No NSIS installer found in {NSIS_DIR}. Re-run with --build.")


def is_running() -> bool:
    result = subprocess.run(
        ["tasklist", "/FI", f"IMAGENAME eq {BINARY_NAME}", "/NH"],
        capture_output=True,
        text=True,
        check=False,
    )
    return BINARY_NAME.lower() in result.stdout.lower()


def taskkill(force: bool) -> None:
    subprocess.run(
        ["taskkill", "/IM", BINARY_NAME, "/T"] + (["/F"] if force else []),
        capture_output=True,
        check=False,
    )


def installed_entry() -> tuple[Path | None, str | None]:
    """Read InstallLocation/DisplayVersion from the NSIS uninstall registry key.

    Tauri's installer defaults to a per-user install (HKCU), but a machine-wide
    one from an earlier config would land in HKLM — check both.
    """
    for hive in (winreg.HKEY_CURRENT_USER, winreg.HKEY_LOCAL_MACHINE):
        try:
            with winreg.OpenKey(
                hive, UNINSTALL_KEY, 0, winreg.KEY_READ | winreg.KEY_WOW64_64KEY
            ) as key:
                location = _reg_str(key, "InstallLocation")
                if location:
                    return Path(location), _reg_str(key, "DisplayVersion")
        except OSError:
            continue
    return None, None


def _reg_str(key: winreg.HKEYType, name: str) -> str | None:
    try:
        value, _ = winreg.QueryValueEx(key, name)
    except OSError:
        return None
    return str(value) if value else None


def fallback_install_dir() -> Path | None:
    """Locate the install by convention, for when the registry key is missing."""
    bases = ("LOCALAPPDATA", "ProgramW6432", "ProgramFiles")
    for base in filter(None, (os.environ.get(name) for name in bases)):
        candidate = Path(base) / APP_NAME
        if (candidate / BINARY_NAME).is_file():
            return candidate
    return None


def main() -> None:
    parser = argparse.ArgumentParser(
        description=f"Install {APP_NAME} on Windows from a local build.",
    )
    parser.add_argument(
        "--build",
        action="store_true",
        help="rebuild the app before installing (default: use the existing build)",
    )
    args = parser.parse_args()

    # ── Step 1: Check platform ──
    step(1, "Checking platform")
    if sys.platform != "win32":
        error("This installer only runs on Windows")
    release, build, *_ = platform.win32_ver()
    success(f"Windows {release} ({build})")

    version = app_version()

    # ── Step 2: Build ──
    step(2, f"Building {APP_NAME} v{version}")
    if args.build:
        tools = {name: shutil.which(name) for name in ("pnpm", "cargo")}
        for name, path in tools.items():
            if path is None:
                error(f"{name} is not installed")
        # Wipe prior output so find_installer() can't pick up a leftover from an
        # older version — same reason deploy_releases.sh clears the bundle dir.
        shutil.rmtree(NSIS_DIR, ignore_errors=True)
        # Only the bundle format Windows actually ships, matching deploy_releases.sh.
        run([tools["pnpm"], "tauri", "build", "--bundles", "nsis"], DESKTOP_DIR, "Build")
        success("Build complete")
    else:
        success("Skipped (using existing build; pass --build to rebuild)")

    installer = find_installer(version)

    # ── Step 3: Stop any running instance ──
    # Windows holds a lock on a running executable, so the installer cannot
    # replace it. NSIS would prompt about this; in silent mode we handle it here.
    step(3, "Stopping running instance")
    if is_running():
        taskkill(force=False)
        for _ in range(16):
            if not is_running():
                break
            time.sleep(0.5)
        if is_running():
            warn("Force-killing (graceful stop timed out)")
            taskkill(force=True)
            time.sleep(0.5)
        success("Stopped running app")
    else:
        success("No running instances")

    # ── Step 4: Install ──
    # /S is NSIS silent mode. The installer writes the Start menu shortcut, the
    # uninstall entry and the WebView2 runtime check itself.
    step(4, f"Installing {installer.name}")
    result = subprocess.run([str(installer), "/S"], check=False)
    if result.returncode == 1223:
        error("Installation cancelled at the Windows elevation prompt")
    if result.returncode != 0:
        error(f"Installer exited with code {result.returncode}")

    install_dir, installed_version = installed_entry()
    if install_dir is None:
        install_dir = fallback_install_dir()
    if install_dir is None or not (install_dir / BINARY_NAME).is_file():
        error("Installer finished but the app was not found on disk")
    if installed_version and installed_version != version:
        warn(f"Registry reports v{installed_version}, expected v{version}")
    success(f"Installed: {install_dir}")

    print(f"\n{GREEN}{RULE} {APP_NAME} {version} installed {RULE}{NC}")
    print(
        f'  {CYAN}Launch from the Start menu, or run:  "{install_dir / BINARY_NAME}"{NC}'
    )


if __name__ == "__main__":
    main()
