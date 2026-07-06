#!/bin/bash
# Build the Rust FFI staticlib and (re)generate the Swift bindings.
# Called by the Xcode pre-build phase and usable standalone:
#   macos/scripts/build-rust.sh [debug|release]
set -euo pipefail

PROFILE="${1:-${CONFIGURATION:-debug}}"
# Xcode passes CONFIGURATION=Debug/Release; normalize.
PROFILE="$(echo "$PROFILE" | tr '[:upper:]' '[:lower:]')"

MACOS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FFI_DIR="$MACOS_DIR/ffi"
GEN_DIR="$MACOS_DIR/generated"

export PATH="$HOME/.cargo/bin:$PATH"
cd "$FFI_DIR"

if [[ "$PROFILE" == "release" ]]; then
    cargo build --release
    LIB="$FFI_DIR/target/release/libgpm_ffi.a"
else
    cargo build
    LIB="$FFI_DIR/target/debug/libgpm_ffi.a"
fi

cargo run --quiet --bin uniffi-bindgen -- generate \
    --library "$LIB" --language swift --out-dir "$GEN_DIR"

# Xcode finds the FFI clang module via SWIFT_INCLUDE_PATHS; it expects the
# canonical module.modulemap filename.
mv -f "$GEN_DIR/gpm_ffiFFI.modulemap" "$GEN_DIR/module.modulemap"

echo "rust: built $LIB and regenerated Swift bindings in $GEN_DIR"
