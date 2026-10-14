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
# Empreintes SHA256 de pdfium.dll attendues (bblanchon/pdfium-binaries, chromium/<build>).
$expected = @{
    "7881-x64"   = "79D4676B656CFB1ABCEA88F9ADE3B4B0826C5200382DB5F4EC72A636C598C118"
    "7881-arm64" = "267A6F08A9C854D9949754A53B7630F23C0B67DE5C7CA273B6ABF178B49158C2"
}
function Test-Pdfium {
    $hash = (Get-FileHash $dll -Algorithm SHA256).Hash
    $want = $expected["$Build-$Arch"]
    if ($want -and $hash -ne $want) { throw "Empreinte inattendue pour $dll : $hash (attendu $want)." }
    if (-not $want) { Write-Warning "Aucune empreinte épinglée pour le build $Build ($Arch) : $hash" }
    $hash
}
if ((Test-Path $dll) -and -not $Force) {
    $present = (Select-String -Path (Join-Path $dir "VERSION") -Pattern '^BUILD=(\d+)' -ErrorAction SilentlyContinue).Matches.Groups[1].Value
    if ($present -ne $Build) { throw "PDFium build $present présent, $Build demandé : relancer avec -Force." }
    $null = Test-Pdfium
    Write-Host "PDFium $Build déjà présent et vérifié : $dll"
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
$hash = Test-Pdfium
Write-Host "OK : $dll"
Write-Host "SHA256 pdfium.dll : $hash"
Get-Content (Join-Path $dir "VERSION")
