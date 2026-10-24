#!/bin/sh
# Construit dist/SuniPDF-<version>-macos-arm64.dmg et son .sha256 : SuniPDF.app (interface, CLI,
# PDFium épinglé, notices de licences vérifiées), signée ad hoc (pas de compte Apple Developer :
# Gatekeeper demande « Ouvrir quand même »).
# Usage : scripts/package-macos.sh [--install]   (--install : copie aussi l'app dans /Applications)
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
if [ "$(uname -s)-$(uname -m)" != Darwin-arm64 ]; then
    echo "Construire sur un Mac Apple silicon (arm64)." >&2
    exit 1
fi
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
name="SuniPDF-$version-macos-arm64.dmg"
# Même plancher que les API AppKit utilisées (application PDF par défaut : macOS 12).
export MACOSX_DEPLOYMENT_TARGET=12.0

scripts/fetch-pdfium.sh arm64
cargo build --release --locked -p sunipdf-desktop -p sunipdf-cli

# Dossier de travail canonique : la vérification des licences refuse tout lien symbolique.
stage="$root/dist/macos"
app="$stage/SuniPDF.app"
work="$root/dist/macos-work"
rm -rf "$stage" "$work" "dist/$name" "dist/$name.sha256"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$work"
cp target/release/sunipdf target/release/sunipdf-cli "$app/Contents/MacOS/"
# À côté de l'exécutable : premier dossier exploré par sunipdf_core::default_library_dirs.
cp third_party/pdfium/mac-arm64/lib/libpdfium.dylib "$app/Contents/MacOS/"

iconset="$work/SuniPDF.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    cp "assets/icon/sunipdf-$size.png" "$iconset/icon_${size}x${size}.png"
done
for size in 16 32 128 256; do
    cp "assets/icon/sunipdf-$((size * 2)).png" "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/SuniPDF.icns"

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
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>$MACOSX_DEPLOYMENT_TARGET</string>
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

metadata="$work"
cargo metadata --locked --format-version 1 > "$metadata/all.json"
cargo metadata --locked --format-version 1 --filter-platform aarch64-apple-darwin > "$metadata/macos.json"
rustc -Vv > "$metadata/rust.txt"
cargo -Vv > "$metadata/cargo.txt"
python3 -I scripts/license-inventory.py --macos --stage "$stage" \
    --metadata "$metadata/all.json" --linux-metadata "$metadata/macos.json" \
    --rust-sysroot "$(rustc --print sysroot)" \
    --rust-version-file "$metadata/rust.txt" --cargo-version-file "$metadata/cargo.txt"
rm -rf "$work"

# La signature scelle les ressources, dont l'inventaire : elle vient après lui.
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
python3 -I scripts/license-inventory.py --macos --verify --stage "$stage"

ln -s /Applications "$stage/Applications"
hdiutil create -quiet -volname "SuniPDF $version" -srcfolder "$stage" -fs HFS+ -format UDZO "dist/$name"
(cd dist && shasum -a 256 "$name" > "$name.sha256")
echo "OK : dist/$name"

if [ "${1:-}" = "--install" ]; then
    rm -rf /Applications/SuniPDF.app
    cp -R "$app" /Applications/
    echo "Installé : /Applications/SuniPDF.app"
fi
