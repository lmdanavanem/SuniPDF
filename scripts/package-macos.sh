#!/bin/sh
# Construit dist/SuniPDF.app pour l'architecture de la machine (binaire release, PDFium, icône),
# signé ad hoc (pas de compte Apple Developer : Gatekeeper demande « Ouvrir quand même »).
# Usage : scripts/package-macos.sh [--install]   (--install : copie dans /Applications)
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
arch=$(uname -m | sed 's/x86_64/x64/')
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

scripts/fetch-pdfium.sh "$arch"
cargo build --release --locked -p sunipdf-desktop

app=dist/SuniPDF.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/licenses"
cp target/release/sunipdf "$app/Contents/MacOS/"
# À côté de l'exécutable : premier dossier exploré par sunipdf_core::default_library_dirs.
cp "third_party/pdfium/mac-$arch/lib/libpdfium.dylib" "$app/Contents/MacOS/"

licenses="$app/Contents/Resources/licenses"
cp LICENSE-MIT LICENSE-APACHE "$licenses/"
cp "third_party/pdfium/mac-$arch/LICENSE" "$licenses/pdfium-binaries-MIT.txt"
cp -R "third_party/pdfium/mac-$arch/licenses" "$licenses/pdfium"
cp assets/fonts/LICENSE-FluentSystemIcons.txt "$licenses/"

iconset=$(mktemp -d)/SuniPDF.iconset
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    cp "assets/icon/sunipdf-$size.png" "$iconset/icon_${size}x${size}.png"
done
for size in 16 32 128 256; do
    cp "assets/icon/sunipdf-$((size * 2)).png" "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/SuniPDF.icns"
rm -rf "$(dirname "$iconset")"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>SuniPDF</string>
    <key>CFBundleDisplayName</key><string>SuniPDF</string>
    <key>CFBundleIdentifier</key><string>com.navanem.sunipdf</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleExecutable</key><string>sunipdf</string>
    <key>CFBundleIconFile</key><string>SuniPDF</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSHumanReadableCopyright</key><string>© 2026 Emanuel DE ALMEIDA (Navanem). MIT OR Apache-2.0.</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key><string>PDF</string>
            <key>CFBundleTypeRole</key><string>Editor</string>
            <key>LSHandlerRank</key><string>Alternate</string>
            <key>LSItemContentTypes</key><array><string>com.adobe.pdf</string></array>
        </dict>
    </array>
</dict>
</plist>
EOF

codesign --force --deep --sign - "$app"
echo "OK : $app ($version, $arch)"

if [ "${1:-}" = "--install" ]; then
    rm -rf /Applications/SuniPDF.app
    cp -R "$app" /Applications/
    echo "Installé : /Applications/SuniPDF.app"
fi
