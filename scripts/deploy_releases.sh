#!/usr/bin/env bash
set -euo pipefail

# Builds this device's release artifact and uploads it to the GitHub release:
#   macOS   → the native SwiftUI app (macos/), zipped .app (macOS 26+)
#   Linux   → the Tauri app as a .deb
#   Windows → the Tauri app as an NSIS installer .exe
# Run once per device; each run adds that platform's artifact to the same tag.
# install_release.sh downloads and installs these artifacts.

# ── Colors ──
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

TOTAL_STEPS=6
REPO="LeoManrique/git-projects-manager"
APP_NAME="Git Projects Manager"

step()    { echo -e "\n${BLUE}[$1/$TOTAL_STEPS]${NC} ${CYAN}$2${NC}"; }
success() { echo -e "  ${GREEN}✓ $1${NC}"; }
warn()    { echo -e "  ${YELLOW}⚠ $1${NC}"; }
error()   { echo -e "  ${RED}✗ $1${NC}"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
DESKTOP_DIR="$REPO_ROOT/desktop"
MACOS_DIR="$REPO_ROOT/macos"
PACKAGE_JSON="$DESKTOP_DIR/package.json"
CARGO_TOML="$DESKTOP_DIR/src-tauri/Cargo.toml"
PROJECT_YML="$MACOS_DIR/project.yml"
# tauri.conf.json reads its version from "../package.json" so it doesn't need
# bumping here.
BUNDLE_DIR="$DESKTOP_DIR/src-tauri/target/release/bundle"   # Tauri output (Linux/Windows)
DIST_DIR="$REPO_ROOT/dist-release"

VERSION="${1:-}"

# Platform first — prerequisites and build steps depend on it.
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64) ARCH_LABEL="x86_64" ;;
  arm64|aarch64) ARCH_LABEL="aarch64" ;;
  *) error "Unsupported arch: $ARCH" ;;
esac

case "$OS" in
  darwin)  PLATFORM="macOS-$ARCH_LABEL" ;;
  linux)   PLATFORM="linux-$ARCH_LABEL" ;;
  mingw*|msys*|cygwin*) OS=windows; PLATFORM="windows-$ARCH_LABEL" ;;
  *) error "Unsupported OS: $OS" ;;
esac

# ── Step 1: Validate prerequisites ──
step 1 "Validating prerequisites ($PLATFORM)"

# macOS ships the native SwiftUI app; the other platforms ship the Tauri app.
if [ "$OS" = darwin ]; then
  REQUIRED=(gh git cargo xcodegen xcodebuild)
else
  REQUIRED=(gh git pnpm cargo)
fi
for cmd in "${REQUIRED[@]}"; do
  command -v "$cmd" &>/dev/null || error "$cmd is not installed"
  success "$cmd found"
done

gh auth status &>/dev/null || error "gh CLI not authenticated. Run: gh auth login"
success "gh authenticated"

# ── Step 2: Determine version ──
step 2 "Determining version"

# package.json is the single source of truth. Match the first top-level
# "version": "x.y.z" line — dependency entries use "<pkg>": "^x.y.z" so they
# don't collide with this pattern.
CURRENT_VERSION=$(sed -n 's/.*"version": "\([0-9][0-9.]*\)".*/\1/p' "$PACKAGE_JSON" | head -1)
[ -z "$CURRENT_VERSION" ] && error "Could not read version from $PACKAGE_JSON"

if [ -z "$VERSION" ]; then
  VERSION="$CURRENT_VERSION"
  success "Using version from package.json: $VERSION"
else
  [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || error "Version must be x.y.z (got: $VERSION)"
  if [ "$VERSION" = "$CURRENT_VERSION" ]; then
    success "Version $VERSION already current, no bump needed"
  else
    HIGHER=$(printf '%s\n%s\n' "$CURRENT_VERSION" "$VERSION" | sort -V | tail -1)
    [ "$HIGHER" = "$VERSION" ] || error "New version $VERSION is not greater than current $CURRENT_VERSION"

    cd "$REPO_ROOT"
    for f in "$PACKAGE_JSON" "$CARGO_TOML" "$PROJECT_YML"; do
      if ! git diff --quiet -- "$f" || ! git diff --cached --quiet -- "$f"; then
        error "$(basename "$f") has uncommitted changes; commit or stash before bumping"
      fi
    done

    # sed -i.bak + rm is the portable form (BSD and GNU sed both accept it).
    # Patterns are already specific: package.json deps use "^x.y.z" / "~x.y.z"
    # so they don't collide with "version": "x.y.z"; Cargo.toml deps use
    # `name = { version = "..." }` so anchoring to ^version= only hits [package];
    # MARKETING_VERSION appears once in project.yml.
    # Avoid the 0,/RE/s//.../ form — it's a GNU-sed extension that silently
    # no-ops on BSD/macOS sed.
    sed -i.bak "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/" "$PACKAGE_JSON"
    sed -i.bak "s/^version = \"$CURRENT_VERSION\"\$/version = \"$VERSION\"/" "$CARGO_TOML"
    sed -i.bak "s/MARKETING_VERSION: \"$CURRENT_VERSION\"/MARKETING_VERSION: \"$VERSION\"/" "$PROJECT_YML"
    rm -f "$PACKAGE_JSON.bak" "$CARGO_TOML.bak" "$PROJECT_YML.bak"

    # Verify all files actually changed — sed exits 0 on pattern miss.
    NEW_PJ=$(sed -n 's/.*"version": "\([0-9][0-9.]*\)".*/\1/p' "$PACKAGE_JSON" | head -1)
    NEW_CT=$(sed -n 's/^version = "\([0-9][0-9.]*\)".*/\1/p' "$CARGO_TOML" | head -1)
    NEW_PY=$(sed -n 's/.*MARKETING_VERSION: "\([0-9][0-9.]*\)".*/\1/p' "$PROJECT_YML" | head -1)
    [ "$NEW_PJ" = "$VERSION" ] || error "Failed to bump package.json (still $NEW_PJ)"
    [ "$NEW_CT" = "$VERSION" ] || error "Failed to bump Cargo.toml (still $NEW_CT)"
    [ "$NEW_PY" = "$VERSION" ] || error "Failed to bump project.yml (still $NEW_PY)"

    # Cargo.lock also pins the package version — refresh it so the bump commit
    # passes `cargo check` cleanly and CI doesn't see a dirty lockfile.
    (cd "$DESKTOP_DIR/src-tauri" && cargo update -p git-projects-manager --offline >/dev/null 2>&1) || true

    git add "$PACKAGE_JSON" "$CARGO_TOML" "$PROJECT_YML" "$DESKTOP_DIR/src-tauri/Cargo.lock"
    git commit -m "Bump version to $VERSION"
    git push
    success "Bumped $CURRENT_VERSION → $VERSION and pushed commit"
  fi
fi

TAG="v$VERSION"
success "Version: $VERSION (tag: $TAG)"

# ── Step 3: Tag release ──
step 3 "Tagging release"

cd "$REPO_ROOT"
if git rev-parse "$TAG" &>/dev/null; then
  warn "Tag $TAG already exists, skipping"
else
  git tag -a "$TAG" -m "Release $TAG"
  git push origin "$TAG"
  success "Created and pushed tag $TAG"
fi

# ── Step 4: Build app ──
step 4 "Building app for $PLATFORM"

if [ "$OS" = darwin ]; then
  # Native SwiftUI app — same sequence as `just build-macos`, with the release
  # version pinned so the bundle can't drift from the artifact name even when
  # no bump ran on this device.
  "$MACOS_DIR/scripts/build-rust.sh" release
  cd "$MACOS_DIR"
  xcodegen generate
  xcodebuild -project GitProjectsManager.xcodeproj \
    -scheme GitProjectsManager -configuration Release \
    -derivedDataPath DerivedData \
    MARKETING_VERSION="$VERSION" build
else
  # Tauri app. Only build the bundle format we actually ship per platform.
  case "$OS" in
    linux)   BUNDLE_TARGET="deb" ;;
    windows) BUNDLE_TARGET="nsis" ;;
  esac
  cd "$DESKTOP_DIR"
  # Wipe prior bundle output so artifact globs in the next step can't pick up
  # leftovers from an older version.
  rm -rf "$BUNDLE_DIR"
  pnpm tauri build --bundles "$BUNDLE_TARGET"
fi
success "Build complete"

# ── Step 5: Package artifact ──
step 5 "Packaging artifact"

mkdir -p "$DIST_DIR"

case "$OS" in
  darwin)
    APP_PATH="$MACOS_DIR/DerivedData/Build/Products/Release/$APP_NAME.app"
    [ -d "$APP_PATH" ] || error "App bundle not found at $APP_PATH"
    ARTIFACT="git-projects-manager-$VERSION-$PLATFORM.zip"
    rm -f "$DIST_DIR/$ARTIFACT"
    # `ditto` preserves macOS metadata (extended attrs, code signatures)
    # which `zip -r` strips. Required if you ever sign the .app.
    (cd "$(dirname "$APP_PATH")" && ditto -c -k --sequesterRsrc --keepParent "$APP_NAME.app" "$DIST_DIR/$ARTIFACT")
    ;;
  linux)
    DEB=$(ls "$BUNDLE_DIR/deb/"*.deb 2>/dev/null | head -1) || true
    [ -z "$DEB" ] && error "No .deb produced in $BUNDLE_DIR/deb/"
    ARTIFACT="git-projects-manager-$VERSION-$PLATFORM.deb"
    cp "$DEB" "$DIST_DIR/$ARTIFACT"
    ;;
  windows)
    EXE=$(ls "$BUNDLE_DIR/nsis/"*.exe 2>/dev/null | head -1) || true
    [ -z "$EXE" ] && error "No NSIS installer produced in $BUNDLE_DIR/nsis/"
    ARTIFACT="git-projects-manager-$VERSION-$PLATFORM.exe"
    cp "$EXE" "$DIST_DIR/$ARTIFACT"
    ;;
esac

success "Packaged: $ARTIFACT"

# ── Step 6: Upload to GitHub Release ──
step 6 "Uploading to GitHub Release"

if gh release view "$TAG" --repo "$REPO" &>/dev/null; then
  warn "Release $TAG already exists, uploading artifact"
  gh release upload "$TAG" "$DIST_DIR/$ARTIFACT" --clobber --repo "$REPO"
else
  gh release create "$TAG" "$DIST_DIR/$ARTIFACT" \
    --repo "$REPO" \
    --title "$APP_NAME $TAG" \
    --notes "## $APP_NAME $TAG

Build artifacts are uploaded per-platform. Run \`scripts/deploy_releases.sh\` on each device to add its build to this release."
fi
success "Uploaded $ARTIFACT to release $TAG"

echo -e "\n${GREEN}═══ Release $TAG complete ($PLATFORM) ═══${NC}"
echo -e "  ${CYAN}https://github.com/$REPO/releases/tag/$TAG${NC}"
