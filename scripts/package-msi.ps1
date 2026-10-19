# Construit l'installateur MSI avec WiX Toolset v3 (candle.exe / light.exe).
# Prérequis : dossier portable préparé par package-portable.ps1 (dist\SuniPDF-<version>-win-<arch>-portable).
# WiX 3.14 est préinstallé sur les runners GitHub windows-latest ; localement : winget install WiXToolset.WiXToolset.
param(
    [ValidateSet("x64", "arm64")] [string]$Arch = "x64",
    [switch]$SelfTest
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

function Assert-ArchiveChecksum([string]$Path, [string]$Expected) {
    if ($Expected -notmatch '^[a-fA-F0-9]{64}$' -or
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ine $Expected) {
        throw 'WiX archive checksum mismatch; refusing extraction.'
    }
}

if ($SelfTest) {
    $directory = Join-Path $root ('target/wix-download-self-test-' + [guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory($directory) | Out-Null
    $fixture = Join-Path $directory 'archive-fixture.bin'
    $expected = 'BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD'
    [IO.File]::WriteAllText($fixture, 'abc', [Text.Encoding]::ASCII)
    Assert-ArchiveChecksum $fixture $expected
    [IO.File]::WriteAllText($fixture, 'abcX', [Text.Encoding]::ASCII)
    $rejected = $false
    try { Assert-ArchiveChecksum $fixture $expected } catch { $rejected = $true }
    if (-not $rejected) { throw 'Modified archive was accepted.' }
    Write-Host 'WiX download checksum self-test: matching archive accepted, modified archive refused.'
    return
}

Set-Location $root
$version = (Select-String -Path "Cargo.toml" -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$stage = Join-Path $root "dist\SuniPDF-$version-win-$Arch-portable"
if (-not (Test-Path "$stage\sunipdf.exe")) { throw "Portable package missing: run package-portable.ps1 -Arch $Arch first." }
& python (Join-Path $PSScriptRoot 'license-inventory.py') --verify --stage $stage
if ($LASTEXITCODE -ne 0) { throw 'The prepared portable payload or license inventory is invalid.' }
foreach ($file in @('tesseract.exe', 'eng.traineddata', 'fra.traineddata', 'ocr-tesseract.txt', 'ocr-leptonica.txt', 'ocr-tessdata-fast.txt', 'ocr-AUTHORS.txt', 'ocr-NOTICE.txt')) {
    if (-not (Test-Path -LiteralPath (Join-Path "$stage\licenses" $file))) { throw "Bundled OCR file missing: $file" }
}

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
    # Official wix3141rtm asset 158158052 (41,297,555 bytes), verified before extraction.
    Assert-ArchiveChecksum "$dl\wix.zip" '6AC824E1642D6F7277D0ED7EA09411A508F6116BA6FAE0AA5F2C7DAA2FF43D31'
    Expand-Archive -Path "$dl\wix.zip" -DestinationPath $dl -Force
    Remove-Item "$dl\wix.zip"
    $wixBin = $dl
}

$obj = Join-Path $root "target\wix-$Arch"
New-Item -ItemType Directory -Force $obj | Out-Null
$msi = Join-Path $root "dist\SuniPDF-$version-win-$Arch.msi"
$wixProvenance = [ordered]@{ schema = 1; tools = @(foreach ($name in @('candle.exe', 'light.exe')) {
    $path = Join-Path $wixBin $name
    [ordered]@{ name = $name; version = (Get-Item -LiteralPath $path).VersionInfo.ProductVersion; sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
}) }
$wixEvidence = Join-Path $obj 'wix-build-provenance.json'
[IO.File]::WriteAllText($wixEvidence, ($wixProvenance | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
# Keep the already verified portable ZIP and stage unchanged when adding MSI build evidence.
$msiStage = Join-Path $obj ('payload-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $msiStage | Out-Null
Copy-Item -Path (Join-Path $stage '*') -Destination $msiStage -Recurse
$stage = $msiStage
& python (Join-Path $PSScriptRoot 'license-inventory.py') --stage $stage --record-wix $wixEvidence
if ($LASTEXITCODE -ne 0) { throw 'Selected MSI build-tool evidence could not be verified.' }

# One component per flat license/OCR file, including the runtime required for offline recognition.
$licenses = Get-ChildItem -File (Join-Path $stage "licenses") | Sort-Object Name
if (-not $licenses) { throw "No licenses found in $stage\licenses" }
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

# The installed package has its own readme.
$readme = "$obj\README.txt"
$readmeText = "SuniPDF $version (Windows $Arch)`r`n" +
    "Installed in this directory. Preferences are stored in %APPDATA%\SuniPDF.`r`n" +
    "The optional startup update check contacts GitHub for release information. No PDF files are sent.`r`n" +
    "English and French text recognition (OCR) works offline with its included engine and language data.`r`n" +
    "Command line: sunipdf-cli.exe --help`r`n" +
    "Licenses and third-party notices: licenses\`r`n"
[System.IO.File]::WriteAllText($readme, $readmeText, (New-Object System.Text.UTF8Encoding $true))

& (Join-Path $wixBin "candle.exe") -nologo -arch $Arch "-dVersion=$version" "-dArch=$Arch" "-dSourceDir=$stage" "-dReadmePath=$readme" "-dIconPath=$root\assets\icon\sunipdf.ico" `
    -out "$obj\" "$root\packaging\wix\SuniPDF.wxs" "$obj\Licenses.wxs"
if ($LASTEXITCODE -ne 0) { throw "candle failed" }
& (Join-Path $wixBin "light.exe") -nologo -sval -out $msi "$obj\SuniPDF.wixobj" "$obj\Licenses.wixobj"
if ($LASTEXITCODE -ne 0) { throw "light failed" }
& (Join-Path $PSScriptRoot 'verify-msi-licenses.ps1') -MsiPath $msi -OutputDirectory $obj
$hash = (Get-FileHash $msi -Algorithm SHA256).Hash
Set-Content -Path "$msi.sha256" -Value "$hash  $(Split-Path -Leaf $msi)" -Encoding ascii
Write-Host "OK : $msi"
Write-Host "SHA256 : $hash"
