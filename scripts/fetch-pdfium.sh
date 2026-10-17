#!/bin/sh
# Télécharge PDFium (bblanchon/pdfium-binaries) pour macOS dans third_party/pdfium/mac-<arch>/.
# Même build épinglé que fetch-pdfium.ps1 (7881, cible de pdfium-render 0.9.4).
# Usage : scripts/fetch-pdfium.sh [arm64|x64]
set -eu
arch=${1:-$(uname -m | sed 's/x86_64/x64/')}
build=7881
case "$build-$arch" in
    7881-arm64) expected=1bc45b15466b34cef96641ce25c77a876e70010c6b114f909dda2f5325fc5bd7 ;;
    7881-x64) expected=4eaad6c3e8d786cf6f66a45d7d014edf5c65f372f98c3070e66595ebb50e43d9 ;;
    *) echo "Architecture inconnue : $arch" >&2; exit 1 ;;
esac
root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/third_party/pdfium/mac-$arch"
lib="$dir/lib/libpdfium.dylib"
if [ ! -f "$lib" ]; then
    mkdir -p "$dir"
    url="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F$build/pdfium-mac-$arch.tgz"
    echo "Téléchargement $url"
    curl -fsSL -o "$dir/pdfium.tgz" "$url"
    tar -xzf "$dir/pdfium.tgz" -C "$dir"
    rm "$dir/pdfium.tgz"
fi
grep -qx "BUILD=$build" "$dir/VERSION" || { echo "PDFium présent mais pas en build $build : supprimer $dir" >&2; exit 1; }
actual=$(shasum -a 256 "$lib" | cut -d' ' -f1)
[ "$actual" = "$expected" ] || { echo "Empreinte inattendue pour $lib : $actual (attendu $expected)" >&2; exit 1; }
echo "PDFium $build vérifié : $lib"
