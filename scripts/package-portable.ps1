# Construit la version portable : dist\SuniPDF-<version>-win-<arch>-portable.zip + SHA256.
# Usage : .\scripts\package-portable.ps1 [-Arch x64|arm64] [-SkipBuild]
param(
    [ValidateSet("x64", "arm64")] [string]$Arch = "x64",
    [switch]$SkipBuild
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$version = (Select-String -Path "Cargo.toml" -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$target = if ($Arch -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
$targetDir = "target\$target\release"

& (Join-Path $PSScriptRoot "fetch-pdfium.ps1") -Arch $Arch
& (Join-Path $PSScriptRoot "fetch-ocr.ps1") -Arch $Arch
if (-not $SkipBuild) {
    cargo build --release --workspace --locked --target $target -j 2
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

$name = "SuniPDF-$version-win-$Arch-portable"
$dist = [IO.Path]::GetFullPath((Join-Path $root 'dist'))
$stage = [IO.Path]::GetFullPath((Join-Path $dist $name))
if (-not $stage.StartsWith($dist + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'The package stage is outside dist.' }
if (Test-Path -LiteralPath $stage) {
    if ((Get-Item -LiteralPath $stage -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'The package stage cannot be a linked directory.' }
    Remove-Item -LiteralPath $stage -Recurse -Force
}
New-Item -ItemType Directory -Force "$stage\licenses" | Out-Null
Copy-Item "$targetDir\sunipdf.exe", "$targetDir\sunipdf-cli.exe" $stage
Copy-Item "third_party\pdfium\win-$Arch\bin\pdfium.dll" $stage
Copy-Item "third_party\pdfium\win-$Arch\LICENSE" "$stage\licenses\pdfium-binaries-MIT.txt"
Copy-Item "third_party\pdfium\win-$Arch\licenses\*" "$stage\licenses\"
Copy-Item "docs\THIRD_PARTY_NOTICES.md" "$stage\licenses\"
Copy-Item "assets\licenses\update-dependencies.txt" "$stage\licenses\"
Copy-Item "assets\licenses\xml-rs-MIT.txt" "$stage\licenses\"
Copy-Item "assets\licenses\lopdf-MIT.txt" "$stage\licenses\"
Copy-Item "assets\licenses\wix3-MS-RL.txt", "LICENSE-MIT", "LICENSE-APACHE" "$stage\licenses\"
# Flat licenses payloads keep updates compatible with the 0.8.x portable updater allowlist.
$ocr = Join-Path $root "third_party/ocr/win-$Arch"
Copy-Item -LiteralPath (Join-Path $ocr 'tesseract.exe'), (Join-Path $ocr 'tessdata/eng.traineddata'), (Join-Path $ocr 'tessdata/fra.traineddata') -Destination "$stage\licenses\"
foreach ($license in (Get-ChildItem -LiteralPath (Join-Path $ocr 'licenses') -File)) {
    Copy-Item -LiteralPath $license.FullName -Destination (Join-Path "$stage\licenses" ("ocr-" + $license.Name))
}
Copy-Item -LiteralPath (Join-Path $ocr 'NOTICE.txt') -Destination "$stage\licenses\ocr-NOTICE.txt"
Copy-Item -LiteralPath (Join-Path $ocr 'VERSION') -Destination "$stage\licenses\ocr-VERSION.txt"
Copy-Item "docs\USER_GUIDE.en.md", "CHANGELOG.md" $stage
& python (Join-Path $PSScriptRoot 'package-guides.py') --source-root $root --stage $stage --tag "v$version"
if ($LASTEXITCODE -ne 0) { throw 'Packaged user guide link preparation failed.' }
Set-Content -Path "$stage\README.txt" -Encoding utf8 -Value @"
SuniPDF $version (portable, Windows $Arch)
Run sunipdf.exe. No installation is required. Preferences are stored in %APPDATA%\SuniPDF.
The optional startup update check contacts GitHub for release information. No PDF files are sent.
English and French text recognition (OCR) works offline with its included engine and language data.
Command line: sunipdf-cli.exe --help
User guide: USER_GUIDE.en.md
Licenses: licenses\
"@

$licenseMetadata = Join-Path $root "target/license-metadata-$Arch"
New-Item -ItemType Directory -Force $licenseMetadata | Out-Null
foreach ($entry in @(@{ Name = 'all'; Filter = @() }, @{ Name = 'x64'; Filter = @('--filter-platform', 'x86_64-pc-windows-msvc') }, @{ Name = 'arm64'; Filter = @('--filter-platform', 'aarch64-pc-windows-msvc') })) {
    $arguments = @('metadata', '--locked', '--format-version', '1') + $entry.Filter
    $metadata = & cargo @arguments
    if ($LASTEXITCODE -ne 0) { throw "Locked license metadata failed: $($entry.Name)" }
    [IO.File]::WriteAllText((Join-Path $licenseMetadata ($entry.Name + '.json')), ($metadata -join "`n"), [Text.UTF8Encoding]::new($false))
}
$rustSysroot = & rustc --print sysroot
if ($LASTEXITCODE -ne 0) { throw 'Rust sysroot is unavailable for standard-library notices.' }
$rustVersion = & rustc -Vv
if ($LASTEXITCODE -ne 0) { throw 'Rust toolchain provenance is unavailable.' }
$rustVersionFile = Join-Path $licenseMetadata 'rust-version.txt'
[IO.File]::WriteAllText($rustVersionFile, ($rustVersion -join "`n"), [Text.UTF8Encoding]::new($false))
$cargoVersion = & cargo -Vv
if ($LASTEXITCODE -ne 0) { throw 'Cargo build-tool provenance is unavailable.' }
$cargoVersionFile = Join-Path $licenseMetadata 'cargo-version.txt'
[IO.File]::WriteAllText($cargoVersionFile, ($cargoVersion -join "`n"), [Text.UTF8Encoding]::new($false))
& python (Join-Path $PSScriptRoot 'license-inventory.py') --stage $stage --arch $Arch --metadata (Join-Path $licenseMetadata 'all.json') --x64-metadata (Join-Path $licenseMetadata 'x64.json') --arm64-metadata (Join-Path $licenseMetadata 'arm64.json') --rust-sysroot $rustSysroot --rust-version-file $rustVersionFile --cargo-version-file $cargoVersionFile
if ($LASTEXITCODE -ne 0) { throw 'Distribution license generation or payload verification failed.' }

$zip = Join-Path $root "dist\$name.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path "$stage\*" -DestinationPath $zip
& python (Join-Path $PSScriptRoot 'license-inventory.py') --verify --archive $zip
if ($LASTEXITCODE -ne 0) { throw 'Archived distribution notices differ from the generated inventory.' }
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash
Set-Content -Path "$zip.sha256" -Value "$hash  $name.zip" -Encoding ascii
Write-Host "OK : $zip"
Write-Host "SHA256 : $hash"
