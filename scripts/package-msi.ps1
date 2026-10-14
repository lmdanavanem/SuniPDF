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
    $candidates = @((Join-Path $root "third_party\wix314"), "$env:WIX\bin", "${env:ProgramFiles(x86)}\WiX Toolset v3.14\bin", "${env:ProgramFiles(x86)}\WiX Toolset v3.11\bin")
    $wixBin = $candidates | Where-Object { $_ -and (Test-Path (Join-Path $_ "candle.exe")) } | Select-Object -First 1
}
if (-not $wixBin) {
    # Binaires autonomes officiels (aucune installation) : https://github.com/wixtoolset/wix3/releases
    $dl = Join-Path $root "third_party\wix314"
    New-Item -ItemType Directory -Force $dl | Out-Null
    Invoke-WebRequest -Uri "https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip" -OutFile "$dl\wix.zip" -UseBasicParsing
    Expand-Archive -Path "$dl\wix.zip" -DestinationPath $dl -Force
    Remove-Item "$dl\wix.zip"
    $wixBin = $dl
}

$obj = Join-Path $root "target\wix"
New-Item -ItemType Directory -Force $obj | Out-Null
$msi = Join-Path $root "dist\SuniPDF-$version-win-$Arch.msi"

# Licences et notices tierces : un composant par fichier du dossier licenses du paquet.
$licenses = Get-ChildItem -File (Join-Path $stage "licenses") | Sort-Object Name
if (-not $licenses) { throw "Aucune licence dans $stage\licenses" }
$components = for ($i = 0; $i -lt $licenses.Count; $i++) {
    $src = [System.Security.SecurityElement]::Escape($licenses[$i].FullName)
    "      <Component Id=`"License$i`" Guid=`"*`" Win64=`"yes`"><File Id=`"License$i`" Source=`"$src`" KeyPath=`"yes`" /></Component>"
}
$fragment = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Fragment>
    <ComponentGroup Id="LicenseFiles" Directory="LicensesFolder">
$($components -join "`n")
    </ComponentGroup>
  </Fragment>
</Wix>
"@
[System.IO.File]::WriteAllText("$obj\Licenses.wxs", $fragment, (New-Object System.Text.UTF8Encoding $false))

# LISEZMOI propre à l'installation (celui du paquet portable parle de « portable »).
$readme = "$obj\LISEZMOI.txt"
$readmeText = "SuniPDF $version (Windows $Arch)`r`n" +
    "Installé dans ce dossier. Aucune télémétrie ; les préférences sont dans %APPDATA%\SuniPDF.`r`n" +
    "Ligne de commande : sunipdf-cli.exe --help`r`n" +
    "Licences et notices tierces : dossier licenses\`r`n"
[System.IO.File]::WriteAllText($readme, $readmeText, (New-Object System.Text.UTF8Encoding $true))

& (Join-Path $wixBin "candle.exe") -nologo -arch $Arch "-dVersion=$version" "-dArch=$Arch" "-dSourceDir=$stage" "-dReadmePath=$readme" "-dIconPath=$root\assets\icon\sunipdf.ico" `
    -out "$obj\" "$root\packaging\wix\SuniPDF.wxs" "$obj\Licenses.wxs"
if ($LASTEXITCODE -ne 0) { throw "candle failed" }
& (Join-Path $wixBin "light.exe") -nologo -sval -out $msi "$obj\SuniPDF.wixobj" "$obj\Licenses.wixobj"
if ($LASTEXITCODE -ne 0) { throw "light failed" }
$hash = (Get-FileHash $msi -Algorithm SHA256).Hash
Set-Content -Path "$msi.sha256" -Value "$hash  $(Split-Path -Leaf $msi)" -Encoding ascii
Write-Host "OK : $msi"
Write-Host "SHA256 : $hash"
