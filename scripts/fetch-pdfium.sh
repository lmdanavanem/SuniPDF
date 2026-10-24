#!/bin/sh
# Fetch the pinned Apple Silicon runtime; verify the archive before extraction.
set -eu
arch=${1:-$(uname -m)}
[ "$arch" = arm64 ] || { echo "Only Apple Silicon (arm64) is supported." >&2; exit 1; }
build=7881
expected=1bc45b15466b34cef96641ce25c77a876e70010c6b114f909dda2f5325fc5bd7
archive_expected=52e94ca5aa8847934330daf3f8150c190682c5ca93831468794f8b90d4392e40
root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/third_party/pdfium/mac-$arch"
lib="$dir/lib/libpdfium.dylib"
if [ ! -f "$lib" ]; then
    mkdir -p "$dir"
    url="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F$build/pdfium-mac-$arch.tgz"
    echo "Downloading $url"
    curl --proto '=https' --proto-redir '=https' -fsSL -o "$dir/pdfium.tgz" "$url"
    archive_actual=$(shasum -a 256 "$dir/pdfium.tgz" | cut -d' ' -f1)
    [ "$archive_actual" = "$archive_expected" ] || { echo "PDFium archive checksum mismatch." >&2; exit 1; }
    tar -xzf "$dir/pdfium.tgz" -C "$dir"
    rm "$dir/pdfium.tgz"
fi
grep -qx "BUILD=$build" "$dir/VERSION" || { echo "Unexpected PDFium build in $dir" >&2; exit 1; }
actual=$(shasum -a 256 "$lib" | cut -d' ' -f1)
[ "$actual" = "$expected" ] || { echo "PDFium library checksum mismatch." >&2; exit 1; }
echo "Verified PDFium $build: $lib"
