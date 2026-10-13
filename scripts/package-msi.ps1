# Construit l'installateur MSI avec WiX Toolset v3 (candle.exe / light.exe).
# Prérequis : dossier portable préparé par package-portable.ps1 (dist\SuniPDF-<version>-win-<arch>-portable).
# WiX 3.14 est préinstallé sur les runners GitHub windows-latest ; localement : winget install WiXToolset.WiXToolset.
param(
    [ValidateSet("x64", "arm64")] [string]$Arch = "x64"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$version = (Select-String -Path "Cargo.toml" -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$stage = Join-Path $root "dist\SuniPDF-$version-win-$Arch-portable"
if (-not (Test-Path "$stage\sunipdf.exe")) { throw "Dossier portable absent : lancer package-portable.ps1 -Arch $Arch d'abord." }

$wixBin = $env:WIX_BIN
if (-not $wixBin) {
    $candidates = @("$env:WIX\bin", "${env:ProgramFiles(x86)}\WiX Toolset v3.14\bin", "${env:ProgramFiles(x86)}\WiX Toolset v3.11\bin")
    $wixBin = $candidates | Where-Object { $_ -and (Test-Path (Join-Path $_ "candle.exe")) } | Select-Object -First 1
}
if (-not $wixBin) {
    # Binaires autonomes officiels (aucune installation) : https://github.com/wixtoolset/wix3/releases
    $dl = "$root	hird_party\wix314"
    New-Item -ItemType Directory -Force $dl | Out-Null
    Invoke-WebRequest -Uri "https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip" -OutFile "$dl\wix.zip" -UseBasicParsing
    Expand-Archive -Path "$dl\wix.zip" -DestinationPath $dl -Force
    Remove-Item "$dl\wix.zip"
    $wixBin = $dl
}

$obj = Join-Path $root "target\wix"
New-Item -ItemType Directory -Force $obj | Out-Null
$msi = Join-Path $root "dist\SuniPDF-$version-win-$Arch.msi"
& (Join-Path $wixBin "candle.exe") -nologo -arch $Arch "-dVersion=$version" "-dArch=$Arch" "-dSourceDir=$stage" "-dIconPath=$root\assets\icon\sunipdf.ico" `
    -out "$obj\SuniPDF.wixobj" "$root\packaging\wix\SuniPDF.wxs"
if ($LASTEXITCODE -ne 0) { throw "candle failed" }
& (Join-Path $wixBin "light.exe") -nologo -sval -out $msi "$obj\SuniPDF.wixobj"
if ($LASTEXITCODE -ne 0) { throw "light failed" }
$hash = (Get-FileHash $msi -Algorithm SHA256).Hash
Set-Content -Path "$msi.sha256" -Value "$hash  $(Split-Path -Leaf $msi)" -Encoding ascii
Write-Host "OK : $msi"
Write-Host "SHA256 : $hash"
