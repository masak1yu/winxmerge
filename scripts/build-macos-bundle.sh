#!/bin/bash
set -euo pipefail

# Build WinXMerge.app with embedded Finder Sync Extension
# Usage: ./scripts/build-macos-bundle.sh [--debug]
#
# Environment variables:
#   CODESIGN_IDENTITY  - Code signing identity (default: "Developer ID Application: Masayuki Uchida (3D5V7PNTDJ)")
#   DEVELOPER_DIR      - Xcode path (default: /Applications/Xcode.app/Contents/Developer)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CODESIGN_IDENTITY="${CODESIGN_IDENTITY:-Developer ID Application: Masayuki Uchida (3D5V7PNTDJ)}"

# Always use real Xcode — Nix may override DEVELOPER_DIR with its SDK path
if [[ -d "/Applications/Xcode.app/Contents/Developer" ]]; then
    export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
else
    export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
fi

BUILD_CONFIG="release"
CARGO_PROFILE="--release"
SWIFT_OPT="-O"
if [[ "${1:-}" == "--debug" ]]; then
    BUILD_CONFIG="debug"
    CARGO_PROFILE=""
    SWIFT_OPT="-Onone -g"
fi

APP_NAME="WinXMerge"
APP_BUNDLE="$PROJECT_ROOT/target/${APP_NAME}.app"
APPEX_NAME="WinXMergeFinderSync"
FINDER_SYNC_DIR="$PROJECT_ROOT/macos/FinderSync"

echo "==> Building Rust binary ($BUILD_CONFIG)..."
cd "$PROJECT_ROOT"
cargo build $CARGO_PROFILE --features desktop

RUST_BINARY="$PROJECT_ROOT/target/${BUILD_CONFIG}/winxmerge"

echo "==> Building Finder Sync Extension ($BUILD_CONFIG)..."

# Use xcodebuild with clean PATH (nix ld conflicts with Xcode linker flags)
XCODE_CONFIG="Debug"
if [[ "$BUILD_CONFIG" == "release" ]]; then
    XCODE_CONFIG="Release"
fi

env -i \
    HOME="$HOME" \
    PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin:${DEVELOPER_DIR}/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
    DEVELOPER_DIR="$DEVELOPER_DIR" \
"${DEVELOPER_DIR}/usr/bin/xcodebuild" \
    -project "$FINDER_SYNC_DIR/FinderSync.xcodeproj" \
    -target "$APPEX_NAME" \
    -configuration "$XCODE_CONFIG" \
    -arch arm64 \
    CODE_SIGN_IDENTITY=- \
    CODE_SIGNING_ALLOWED=NO \
    SYMROOT="$PROJECT_ROOT/target/xcode-build" \
    build 2>&1 | tail -3

APPEX_BUNDLE="$PROJECT_ROOT/target/xcode-build/${XCODE_CONFIG}/${APPEX_NAME}.appex"
if [[ ! -d "$APPEX_BUNDLE" ]]; then
    echo "ERROR: appex not found at $APPEX_BUNDLE"
    exit 1
fi
echo "    Built: $APPEX_BUNDLE"

echo "==> Assembling ${APP_NAME}.app bundle..."
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/PlugIns"
mkdir -p "$APP_BUNDLE/Contents/Resources"
mkdir -p "$APP_BUNDLE/Contents/Frameworks"

# Copy binary
cp "$RUST_BINARY" "$APP_BUNDLE/Contents/MacOS/winxmerge"
chmod +x "$APP_BUNDLE/Contents/MacOS/winxmerge"

# Bundle Nix store dylibs into the app and rewrite references.
# macOS 11+ removed many /usr/lib dylibs from disk (they live in the shared
# cache), so pointing at /usr/lib is unreliable. Bundling is the standard
# approach for app distribution.
FRAMEWORKS_DIR="$APP_BUNDLE/Contents/Frameworks"

# Phase 1: Copy Nix dylibs and rewrite references in the main binary
nix_libs=$(otool -L "$APP_BUNDLE/Contents/MacOS/winxmerge" | grep '/nix/store/' | awk '{print $1}')
for nix_lib in $nix_libs; do
    lib_name=$(basename "$nix_lib")
    echo "  Bundling $lib_name"
    cp "$nix_lib" "$FRAMEWORKS_DIR/$lib_name"
    chmod 644 "$FRAMEWORKS_DIR/$lib_name"
    install_name_tool -change "$nix_lib" "@executable_path/../Frameworks/$lib_name" "$APP_BUNDLE/Contents/MacOS/winxmerge"
done

# Phase 2: Fix references inside the bundled dylibs themselves
for dylib in "$FRAMEWORKS_DIR"/*.dylib; do
    [ -f "$dylib" ] || continue
    lib_name=$(basename "$dylib")
    # Fix the dylib's own install name
    install_name_tool -id "@executable_path/../Frameworks/$lib_name" "$dylib"
    # Rewrite any Nix store references within this dylib
    for dep in $(otool -L "$dylib" | grep '/nix/store/' | awk '{print $1}'); do
        dep_name=$(basename "$dep")
        if [ -f "$FRAMEWORKS_DIR/$dep_name" ]; then
            install_name_tool -change "$dep" "@executable_path/../Frameworks/$dep_name" "$dylib"
        else
            # Dependency not yet bundled — copy it too
            echo "  Bundling transitive dep $dep_name"
            cp "$dep" "$FRAMEWORKS_DIR/$dep_name"
            chmod 644 "$FRAMEWORKS_DIR/$dep_name"
            install_name_tool -id "@executable_path/../Frameworks/$dep_name" "$FRAMEWORKS_DIR/$dep_name"
            install_name_tool -change "$dep" "@executable_path/../Frameworks/$dep_name" "$dylib"
        fi
    done
done

# Copy Info.plist
cp "$PROJECT_ROOT/macos/Info.plist" "$APP_BUNDLE/Contents/Info.plist"

# Copy icon
cp "$PROJECT_ROOT/assets/icons/app-icon.icns" "$APP_BUNDLE/Contents/Resources/app-icon.icns"

# Copy Finder Sync extension
cp -R "$APPEX_BUNDLE" "$APP_BUNDLE/Contents/PlugIns/"

echo "==> Code signing..."
# Sign bundled frameworks first (inside-out signing order)
for dylib in "$FRAMEWORKS_DIR"/*.dylib; do
    [ -f "$dylib" ] || continue
    codesign --force --sign "$CODESIGN_IDENTITY" --options runtime "$dylib"
done

# Sign the extension
codesign --force --options runtime \
    --entitlements "$FINDER_SYNC_DIR/FinderSync.entitlements" \
    --sign "$CODESIGN_IDENTITY" \
    "$APP_BUNDLE/Contents/PlugIns/${APPEX_NAME}.appex"

# Sign the main app
codesign --force --options runtime \
    --entitlements "$PROJECT_ROOT/macos/WinXMerge.entitlements" \
    --sign "$CODESIGN_IDENTITY" \
    "$APP_BUNDLE"

echo "==> Verifying signature..."
codesign --verify --deep --strict "$APP_BUNDLE"

echo ""
echo "SUCCESS: $APP_BUNDLE"
echo ""
echo "To install:"
echo "  cp -R \"$APP_BUNDLE\" /Applications/"
echo ""
echo "To enable Finder extension:"
echo "  System Settings > Privacy & Security > Extensions > Added Extensions > WinXMerge"
