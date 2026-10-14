# Télécharge les binaires PDFium (bblanchon/pdfium-binaries) dans third_party/pdfium/win-<arch>/.
# Version épinglée sur celle ciblée par pdfium-render 0.9.4 (feature pdfium_latest = build 7881).
param(
    [ValidateSet("x64", "arm64")] [string]$Arch = "x64",
    [string]$Build = "7881",
    [switch]$Force
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dir = Join-Path $root "third_party\pdfium\win-$Arch"
$dll = Join-Path $dir "bin\pdfium.dll"
if ((Test-Path $dll) -and -not $Force) {
    Write-Host "PDFium déjà présent : $dll"
    exit 0
}
New-Item -ItemType Directory -Force $dir | Out-Null
$url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F$Build/pdfium-win-$Arch.tgz"
$archive = Join-Path $dir "pdfium-win-$Arch.tgz"
Write-Host "Téléchargement $url"
Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing
# tar de Windows (bsdtar) explicitement : le tar GNU de Git Bash prend « C: » pour un hôte distant.
$tar = Join-Path $env:SystemRoot "System32\tar.exe"
if (-not (Test-Path $tar)) { $tar = "tar" }
& $tar -xzf $archive -C $dir
if ($LASTEXITCODE -ne 0) { throw "Extraction de $archive impossible (tar a renvoyé $LASTEXITCODE)." }
Remove-Item $archive
$hash = (Get-FileHash $dll -Algorithm SHA256).Hash
Write-Host "OK : $dll"
Write-Host "SHA256 pdfium.dll : $hash"
Get-Content (Join-Path $dir "VERSION")
