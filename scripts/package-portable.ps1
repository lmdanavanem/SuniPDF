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
if (-not $SkipBuild) {
    cargo build --release --workspace --target $target -j 2
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

$name = "SuniPDF-$version-win-$Arch-portable"
$stage = Join-Path $root "dist\$name"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force "$stage\licenses" | Out-Null
Copy-Item "$targetDir\sunipdf.exe", "$targetDir\sunipdf-cli.exe" $stage
Copy-Item "third_party\pdfium\win-$Arch\bin\pdfium.dll" $stage
Copy-Item "third_party\pdfium\win-$Arch\LICENSE" "$stage\licenses\pdfium-binaries-MIT.txt"
Copy-Item "third_party\pdfium\win-$Arch\licenses\*" "$stage\licenses\"
Copy-Item "docs\THIRD_PARTY_NOTICES.md" "$stage\licenses\"
Copy-Item "docs\USER_GUIDE.fr.md", "docs\USER_GUIDE.en.md", "README.md" $stage -ErrorAction SilentlyContinue
Set-Content -Path "$stage\LISEZMOI.txt" -Encoding utf8 -Value @"
SuniPDF $version (portable, Windows $Arch)
Lancer sunipdf.exe. Aucune installation, aucune télémétrie. Les préférences sont dans %APPDATA%\SuniPDF.
Ligne de commande : sunipdf-cli.exe --help
Licences : dossier licenses\
"@

$zip = Join-Path $root "dist\$name.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path "$stage\*" -DestinationPath $zip
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash
Set-Content -Path "$zip.sha256" -Value "$hash  $name.zip" -Encoding ascii
Write-Host "OK : $zip"
Write-Host "SHA256 : $hash"
