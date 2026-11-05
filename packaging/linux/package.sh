#!/usr/bin/env bash
# Package a release binary as a tarball and an AppImage.
#
#   packaging/linux/package.sh <version> <arch> <binary> <out-dir>
#
# <arch> is x86_64 or aarch64. The tarball keeps a /usr-style layout
# (bin/, share/) that the AUR package installs from. The AppImage bundles no
# libraries: GPUI loads Wayland/X11/Vulkan from the system, which must match
# the user's drivers anyway (as Zed does). appimagetool and its runtime are
# pinned by version and checksum.
set -euo pipefail

version=$1 arch=$2 binary=$3 out=$4
root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$out"
out=$(cd "$out" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# --- tarball ---------------------------------------------------------------
name="frankmdfast-$version-linux-$arch"
stage="$work/$name"
install -Dm755 "$binary" "$stage/bin/frankmdfast"
install -Dm644 "$root/packaging/linux/frankmdfast.desktop" "$stage/share/applications/frankmdfast.desktop"
install -Dm644 "$root/packaging/linux/frankmdfast.metainfo.xml" "$stage/share/metainfo/frankmdfast.metainfo.xml"
install -Dm644 "$root/assets/brand/icon.svg" "$stage/share/icons/hicolor/scalable/apps/frankmdfast.svg"
install -Dm644 "$root/assets/brand/icon-512.png" "$stage/share/icons/hicolor/512x512/apps/frankmdfast.png"
install -Dm644 "$root/assets/brand/favicon-32x32.png" "$stage/share/icons/hicolor/32x32/apps/frankmdfast.png"
install -Dm644 "$root/assets/brand/favicon-16x16.png" "$stage/share/icons/hicolor/16x16/apps/frankmdfast.png"
install -Dm644 "$root/LICENSE" "$stage/LICENSE"
install -Dm644 "$root/README.md" "$stage/README.md"
tar -C "$work" -czf "$out/$name.tar.gz" "$name"

# --- AppImage --------------------------------------------------------------
case "$arch" in
  x86_64)
    tool_sha=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
    runtime_sha=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d ;;
  aarch64)
    tool_sha=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
    runtime_sha=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444 ;;
  *) echo "unsupported arch: $arch" >&2; exit 1 ;;
esac
tool="$work/appimagetool"
runtime="$work/runtime"
curl -fsSL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-$arch.AppImage"
curl -fsSL -o "$runtime" "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-$arch"
echo "$tool_sha  $tool" | sha256sum -c --quiet
echo "$runtime_sha  $runtime" | sha256sum -c --quiet
chmod +x "$tool"

appdir="$work/FrankMDFast.AppDir"
mkdir -p "$appdir/usr"
cp -a "$stage/bin" "$stage/share" "$appdir/usr/"
cp "$root/packaging/linux/frankmdfast.desktop" "$appdir/frankmdfast.desktop"
cp "$root/assets/brand/icon-512.png" "$appdir/frankmdfast.png"
ln -s frankmdfast.png "$appdir/.DirIcon"
ln -s usr/bin/frankmdfast "$appdir/AppRun"

# Runners have no FUSE: let the tool unpack itself instead of mounting.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$arch" "$tool" --no-appstream \
  --runtime-file "$runtime" "$appdir" "$out/FrankMDFast-$version-$arch.AppImage"

ls -l "$out"
