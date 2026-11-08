#!/bin/bash
# Build a local, ad-hoc-signed Wunicorm.app and a drag-to-install DMG.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

for tool in cargo codesign hdiutil iconutil install_name_tool otool plutil python3 sips; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'error: required tool not found: %s\n' "$tool" >&2
        exit 1
    fi
done

VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
ARCH="$(uname -m)"
PACKAGE_DIR="$ROOT/target/packaging"
APP_DIR="$PACKAGE_DIR/Wunicorm.app"
BINARY="$APP_DIR/Contents/MacOS/wunicorm"
ICONSET="$PACKAGE_DIR/AppIcon.iconset"
DMG_ROOT="$PACKAGE_DIR/dmg-root"
DMG_PATH="$PACKAGE_DIR/Wunicorm-$VERSION-$ARCH.dmg"

printf 'Building Wunicorm %s for %s...\n' "$VERSION" "$ARCH"
cargo build --release

rm -rf "$APP_DIR" "$DMG_ROOT" "$ICONSET"
rm -f "$DMG_PATH"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources" "$DMG_ROOT"

cp "$ROOT/target/release/memory-proto" "$BINARY"
chmod 755 "$BINARY"
cp "$ROOT/packaging/macos/Info.plist" "$APP_DIR/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$VERSION" "$APP_DIR/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$VERSION" "$APP_DIR/Contents/Info.plist"
plutil -lint "$APP_DIR/Contents/Info.plist"

# Generate the application icon from the tracked 1024x1024 source.
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
    retina=$((size * 2))
    sips -z "$size" "$size" "$ROOT/packaging/macos/AppIcon.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    sips -z "$retina" "$retina" "$ROOT/packaging/macos/AppIcon.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/AppIcon.icns"

# LadybugDB links against the host's OpenSSL. Vendor the two dylibs so
# the app does not depend on this machine's Homebrew installation.
HOST_BINARY="$ROOT/target/release/memory-proto"
SSL_SOURCE="$(otool -L "$HOST_BINARY" | python3 -c 'import sys; refs = [line.strip().split()[0] for line in sys.stdin if "/libssl" in line]; print(refs[0] if refs else "")')"
CRYPTO_SOURCE="$(otool -L "$HOST_BINARY" | python3 -c 'import sys; refs = [line.strip().split()[0] for line in sys.stdin if "/libcrypto" in line]; print(refs[0] if refs else "")')"
if [[ -z "$SSL_SOURCE" || -z "$CRYPTO_SOURCE" ]]; then
    printf 'error: could not locate the OpenSSL dynamic libraries\n' >&2
    exit 1
fi
SSL_NAME="${SSL_SOURCE##*/}"
CRYPTO_NAME="${CRYPTO_SOURCE##*/}"
FRAMEWORKS="$APP_DIR/Contents/Frameworks"
mkdir -p "$FRAMEWORKS"
cp "$SSL_SOURCE" "$FRAMEWORKS/$SSL_NAME"
cp "$CRYPTO_SOURCE" "$FRAMEWORKS/$CRYPTO_NAME"
chmod 755 "$FRAMEWORKS/$SSL_NAME" "$FRAMEWORKS/$CRYPTO_NAME"

# libssl refers to libcrypto by a versioned Cellar path; discover and
# rewrite that reference too.
SSL_CRYPTO_SOURCE="$(otool -L "$FRAMEWORKS/$SSL_NAME" | python3 -c 'import sys; refs = [line.strip().split()[0] for line in sys.stdin if "/libcrypto" in line]; print(refs[0] if refs else "")')"
if [[ -z "$SSL_CRYPTO_SOURCE" ]]; then
    printf 'error: could not locate libssl\x27s libcrypto dependency\n' >&2
    exit 1
fi
install_name_tool -id "@rpath/$SSL_NAME" "$FRAMEWORKS/$SSL_NAME"
install_name_tool -id "@rpath/$CRYPTO_NAME" "$FRAMEWORKS/$CRYPTO_NAME"
install_name_tool -change "$SSL_CRYPTO_SOURCE" "@rpath/$CRYPTO_NAME" "$FRAMEWORKS/$SSL_NAME"
install_name_tool -change "$SSL_SOURCE" "@rpath/$SSL_NAME" "$BINARY"
install_name_tool -change "$CRYPTO_SOURCE" "@rpath/$CRYPTO_NAME" "$BINARY"
install_name_tool -add_rpath "@executable_path/../Frameworks" "$BINARY"
cp /opt/homebrew/opt/openssl@4/LICENSE.txt "$APP_DIR/Contents/Resources/OpenSSL-LICENSE.txt"

# Refuse to silently ship references to this checkout or common local
# package-manager locations. System frameworks and /usr/lib are fine.
for object in "$BINARY" "$FRAMEWORKS/$SSL_NAME" "$FRAMEWORKS/$CRYPTO_NAME"; do
    if otool -L "$object" | grep -E '^[[:space:]]+(/Users/|/opt/homebrew/|/usr/local/opt/)'; then
        printf 'error: %s has machine-local dynamic library dependencies\n' "$object" >&2
        exit 1
    fi
done

codesign --force --sign - --timestamp=none "$FRAMEWORKS/$SSL_NAME"
codesign --force --sign - --timestamp=none "$FRAMEWORKS/$CRYPTO_NAME"
codesign --force --sign - --timestamp=none "$APP_DIR"
codesign --verify --strict --verbose=2 "$APP_DIR"

ditto "$APP_DIR" "$DMG_ROOT/Wunicorm.app"
ln -s /Applications "$DMG_ROOT/Applications"

hdiutil create -volname Wunicorm -srcfolder "$DMG_ROOT" -ov -format UDZO "$DMG_PATH"
hdiutil verify "$DMG_PATH"

printf '\nCreated: %s\n' "$DMG_PATH"
printf 'Mount it and drag Wunicorm.app to Applications.\n'
printf 'Local ad-hoc signature only; not Developer ID signed or notarized.\n'
