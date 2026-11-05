#!/usr/bin/env bash
# Build FrankMDFast.app from a (universal) binary and wrap it in a DMG.
#
#   packaging/macos/package.sh <version> <binary> <out-dir>
#
# Signing is optional and driven by the environment:
#   MACOS_CERTIFICATE (base64 .p12), MACOS_CERTIFICATE_PASSWORD,
#   MACOS_SIGNING_IDENTITY ("Developer ID Application: Name (TEAMID)")
#     -> Developer ID signature with the hardened runtime;
#   plus APPLE_ID, APPLE_APP_PASSWORD, APPLE_TEAM_ID
#     -> notarized and stapled DMG.
# Without them the app is ad-hoc signed (Gatekeeper asks on first open).
set -euo pipefail

version=$1 binary=$2 out=$3
root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$out"
out=$(cd "$out" && pwd)
work=$(mktemp -d)
cleanup() {
  if [ -n "${keychain:-}" ]; then security delete-keychain "$keychain" || true; fi
  rm -rf "$work"
}
trap cleanup EXIT

app="$work/FrankMDFast.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
install -m755 "$binary" "$app/Contents/MacOS/frankmdfast"
sed "s/@VERSION@/$version/g" "$root/packaging/macos/Info.plist" > "$app/Contents/Info.plist"

iconset="$work/frankmdfast.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z $size $size "$root/assets/brand/icon-512.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  [ $double -le 512 ] && sips -z $double $double "$root/assets/brand/icon-512.png" \
    --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/frankmdfast.icns"

if [ -n "${MACOS_CERTIFICATE:-}" ]; then
  keychain="$work/signing.keychain-db"
  kc_pass=$(openssl rand -hex 16)
  echo "$MACOS_CERTIFICATE" | base64 --decode > "$work/cert.p12"
  security create-keychain -p "$kc_pass" "$keychain"
  security set-keychain-settings -lut 3600 "$keychain"
  security unlock-keychain -p "$kc_pass" "$keychain"
  security import "$work/cert.p12" -k "$keychain" -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
  security set-key-partition-list -S apple-tool:,apple: -s -k "$kc_pass" "$keychain" >/dev/null
  security list-keychains -d user -s "$keychain" $(security list-keychains -d user | tr -d '"')
  rm -f "$work/cert.p12"
  codesign --force --options runtime --timestamp --keychain "$keychain" \
    --sign "$MACOS_SIGNING_IDENTITY" "$app"
else
  echo "MACOS_CERTIFICATE not set: ad-hoc signing" >&2
  codesign --force --sign - "$app"
fi
codesign --verify --strict "$app"

dmg="$out/FrankMDFast-$version-macos-universal.dmg"
staging="$work/dmg"
mkdir -p "$staging"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
# hdiutil's own size estimate for -srcfolder can come up short ("No space
# left on device"), so size the image explicitly with headroom.
size_mb=$(( $(du -sm "$staging" | cut -f1) + 40 ))
hdiutil create -volname "FrankMDFast" -srcfolder "$staging" -size "${size_mb}m" -ov -format UDZO "$dmg"

if [ -n "${MACOS_CERTIFICATE:-}" ]; then
  codesign --force --timestamp --keychain "$keychain" --sign "$MACOS_SIGNING_IDENTITY" "$dmg"
  if [ -n "${APPLE_ID:-}" ]; then
    xcrun notarytool submit "$dmg" --apple-id "$APPLE_ID" --password "$APPLE_APP_PASSWORD" \
      --team-id "$APPLE_TEAM_ID" --wait
    xcrun stapler staple "$dmg"
  fi
fi

ls -l "$out"
