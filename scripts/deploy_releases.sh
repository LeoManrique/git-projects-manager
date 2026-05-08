#!/usr/bin/env bash
set -euo pipefail

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
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PACKAGE_JSON="$PROJECT_ROOT/package.json"
CARGO_TOML="$PROJECT_ROOT/src-tauri/Cargo.toml"
# tauri.conf.json reads its version from "../package.json" so it doesn't need
# bumping here.

VERSION="${1:-}"

# ── Step 1: Validate prerequisites ──
step 1 "Validating prerequisites"

for cmd in gh git pnpm cargo zip; do
  command -v "$cmd" &>/dev/null || error "$cmd is not installed"
  success "$cmd found"
done

gh auth status &>/dev/null || error "gh CLI not authenticated. Run: gh auth login"
success "gh authenticated"

# ── Step 2: Determine version and platform ──
step 2 "Determining version and platform"

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

    cd "$PROJECT_ROOT"
    for f in "$PACKAGE_JSON" "$CARGO_TOML"; do
      if ! git diff --quiet -- "$f" || ! git diff --cached --quiet -- "$f"; then
        error "$(basename "$f") has uncommitted changes; commit or stash before bumping"
      fi
    done

    # sed -i.bak + rm is the portable form (BSD and GNU sed both accept it).
    # Use 0,/pattern/ to replace only the first occurrence — guards against
    # hitting a same-versioned dependency further down the file.
    sed -i.bak "0,/\"version\": \"$CURRENT_VERSION\"/s//\"version\": \"$VERSION\"/" "$PACKAGE_JSON"
    sed -i.bak "0,/^version = \"$CURRENT_VERSION\"/s//version = \"$VERSION\"/" "$CARGO_TOML"
    rm -f "$PACKAGE_JSON.bak" "$CARGO_TOML.bak"

    # Cargo.lock also pins the package version — refresh it so the bump commit
    # passes `cargo check` cleanly and CI doesn't see a dirty lockfile.
    (cd "$PROJECT_ROOT/src-tauri" && cargo update -p git-projects-manager --offline >/dev/null 2>&1) || true

    git add "$PACKAGE_JSON" "$CARGO_TOML" "$PROJECT_ROOT/src-tauri/Cargo.lock"
    git commit -m "Bump version to $VERSION"
    git push
    success "Bumped $CURRENT_VERSION → $VERSION and pushed commit"
  fi
fi

TAG="v$VERSION"
success "Version: $VERSION (tag: $TAG)"

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
success "Platform: $PLATFORM"

# ── Step 3: Tag release ──
step 3 "Tagging release"

cd "$PROJECT_ROOT"
if git rev-parse "$TAG" &>/dev/null; then
  warn "Tag $TAG already exists, skipping"
else
  git tag -a "$TAG" -m "Release $TAG"
  git push origin "$TAG"
  success "Created and pushed tag $TAG"
fi

# ── Step 4: Build app ──
step 4 "Building app for $PLATFORM"

cd "$PROJECT_ROOT"
# Wipe prior bundle output so artifact globs in the next step can't pick up
# leftovers from an older version.
rm -rf "$PROJECT_ROOT/src-tauri/target/release/bundle"
pnpm tauri build
success "Build complete"

# ── Step 5: Package artifact ──
step 5 "Packaging artifact"

DIST_DIR="$PROJECT_ROOT/dist-release"
mkdir -p "$DIST_DIR"
BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"

case "$OS" in
  darwin)
    APP_PATH="$BUNDLE_DIR/macos/$APP_NAME.app"
    [ -d "$APP_PATH" ] || error "App bundle not found at $APP_PATH"
    ARTIFACT="git-projects-manager-$VERSION-$PLATFORM.zip"
    rm -f "$DIST_DIR/$ARTIFACT"
    # `ditto` preserves macOS metadata (extended attrs, code signatures)
    # which `zip -r` strips. Required if you ever sign the .app.
    (cd "$BUNDLE_DIR/macos" && ditto -c -k --sequesterRsrc --keepParent "$APP_NAME.app" "$DIST_DIR/$ARTIFACT")
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
