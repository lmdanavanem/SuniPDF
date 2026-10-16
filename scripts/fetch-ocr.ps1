# Build the offline OCR runtime from pinned upstream sources, without image codec DLLs.
param(
    [ValidateSet('x64', 'arm64')] [string]$Arch = 'x64',
    [string]$Generator = '',
    [switch]$Force
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$downloads = Join-Path $root 'target/ocr-downloads'
$build = Join-Path $root "target/ocr-build-$Arch"
$install = Join-Path $build 'install'
$stage = Join-Path $root "third_party/ocr/win-$Arch"
$marker = Join-Path $stage 'VERSION'
$version = "TESSERACT=5.5.3;LEPTONICA=1.87.0;TESSDATA=87416418657359cb625c412a48b6e1d6d41c29bd;PNM_ONLY=1;STDINT_PATCH=1;NOTICES=1;ARCH=$Arch"
if ((Test-Path -LiteralPath $marker) -and -not $Force) {
    if ([IO.File]::ReadAllText($marker).Trim() -ne $version) { throw 'Unexpected OCR build version; rerun with -Force.' }
    foreach ($file in @('tesseract.exe', 'tessdata/eng.traineddata', 'tessdata/fra.traineddata', 'licenses/AUTHORS.txt', 'licenses/COPYRIGHTS.txt', 'SHA256SUMS')) {
        if (-not (Test-Path -LiteralPath (Join-Path $stage $file))) { throw "Incomplete OCR runtime: $file" }
    }
    foreach ($line in [IO.File]::ReadAllLines((Join-Path $stage 'SHA256SUMS'))) {
        $parts = $line -split '  ', 2
        if ($parts.Length -ne 2 -or (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $stage $parts[1])).Hash -ne $parts[0]) { throw 'OCR runtime checksum mismatch.' }
    }
    Write-Host "Verified OCR runtime: $stage"
    exit 0
}
New-Item -ItemType Directory -Force -Path $downloads, $build, $stage, (Join-Path $stage 'tessdata'), (Join-Path $stage 'licenses') | Out-Null
$sources = @(
    @{ File = 'tesseract-5.5.3.tar.gz'; Url = 'https://github.com/tesseract-ocr/tesseract/archive/refs/tags/5.5.3.tar.gz'; Hash = '9218E62793116D42A9F6D14CD9348518B27F382096EEA3D0F2D1A24616BB5884' },
    @{ File = 'leptonica-1.87.0.tar.gz'; Url = 'https://github.com/DanBloomberg/leptonica/archive/refs/tags/1.87.0.tar.gz'; Hash = 'FA2B40C5CAEA96D1BB93A97486262AED8731B69CE25A84A6BF5D25323E33F631' },
    @{ File = 'eng.traineddata'; Url = 'https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/87416418657359cb625c412a48b6e1d6d41c29bd/eng.traineddata'; Hash = '7D4322BD2A7749724879683FC3912CB542F19906C83BCC1A52132556427170B2' },
    @{ File = 'fra.traineddata'; Url = 'https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/87416418657359cb625c412a48b6e1d6d41c29bd/fra.traineddata'; Hash = 'CED037562E8C80C13122DECE28DD477D399AF80911A28791A66A63AC1E3445CA' }
)
foreach ($source in $sources) {
    $path = Join-Path $downloads $source.File
    if (-not (Test-Path -LiteralPath $path)) {
        & curl.exe --location --fail --silent --show-error --output $path $source.Url
        if ($LASTEXITCODE -ne 0) { throw "Download failed: $($source.File)" }
    }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $source.Hash) { throw "Source checksum mismatch: $($source.File)" }
}
$tar = Join-Path $env:SystemRoot 'System32/tar.exe'
foreach ($name in @('tesseract-5.5.3', 'leptonica-1.87.0')) {
    if (-not (Test-Path -LiteralPath (Join-Path $downloads "$name/CMakeLists.txt"))) {
        & $tar -xzf (Join-Path $downloads "$name.tar.gz") -C $downloads
        if ($LASTEXITCODE -ne 0) { throw "Extraction failed: $name" }
    }
}
if (-not $Generator) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format json | ConvertFrom-Json
    if (-not $vs) { throw 'Visual Studio C++ build tools are required.' }
    $major = ([version]$vs.installationVersion).Major
    $years = @{ 17 = 2022; 18 = 2026 }
    if (-not $years.ContainsKey($major)) { throw 'Specify -Generator for this Visual Studio version.' }
    $Generator = "Visual Studio $major $($years[$major])"
}
$platform = if ($Arch -eq 'arm64') { 'ARM64' } else { 'x64' }
function Invoke-CMake([string[]]$Arguments) {
    & cmake @Arguments
    if ($LASTEXITCODE -ne 0) { throw "CMake failed ($LASTEXITCODE)." }
}
$leptSource = Join-Path $downloads 'leptonica-1.87.0'
$leptBuild = Join-Path $build 'leptonica'
Invoke-CMake @('-S', $leptSource, '-B', $leptBuild, '-G', $Generator, '-A', $platform, "-DCMAKE_INSTALL_PREFIX=$install", '-DCMAKE_POLICY_DEFAULT_CMP0091=NEW', '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded', '-DBUILD_SHARED_LIBS=OFF', '-DBUILD_PROG=OFF', '-DSW_BUILD=OFF', '-DENABLE_ZLIB=OFF', '-DENABLE_PNG=OFF', '-DENABLE_GIF=OFF', '-DENABLE_JPEG=OFF', '-DENABLE_TIFF=OFF', '-DENABLE_WEBP=OFF', '-DENABLE_OPENJPEG=OFF')
Invoke-CMake @('--build', $leptBuild, '--config', 'Release', '--parallel', '2')
Invoke-CMake @('--install', $leptBuild, '--config', 'Release')
$tessSource = Join-Path $downloads 'tesseract-5.5.3'
# Upstream enables unrestricted /MP; keep native builds within the same two-job budget.
$cmakeFile = Join-Path $tessSource 'CMakeLists.txt'
[IO.File]::WriteAllText($cmakeFile, [IO.File]::ReadAllText($cmakeFile).Replace(' /MP"', ' /MP2"'), [Text.UTF8Encoding]::new($false))
# Tesseract 5.5.3 image.h relies on a transitive uint32_t declaration removed in Leptonica 1.87.
$imageHeader = Join-Path $tessSource 'src/ccstruct/image.h'
$imageSource = [IO.File]::ReadAllText($imageHeader)
if (-not $imageSource.Contains('#include <stdint.h>')) {
    [IO.File]::WriteAllText($imageHeader, $imageSource.Replace('#include <allheaders.h>', "#include <stdint.h>`n#include <allheaders.h>"), [Text.UTF8Encoding]::new($false))
}
$tessBuild = Join-Path $build 'tesseract'
Invoke-CMake @('-S', $tessSource, '-B', $tessBuild, '-G', $Generator, '-A', $platform, "-DCMAKE_PREFIX_PATH=$install", "-DCMAKE_INSTALL_PREFIX=$install", '-DBUILD_SHARED_LIBS=OFF', '-DWIN32_MT_BUILD=ON', '-DSW_BUILD=OFF', '-DOPENMP_BUILD=OFF', '-DGRAPHICS_DISABLED=ON', '-DDISABLED_LEGACY_ENGINE=ON', '-DENABLE_LTO=OFF', '-DENABLE_NATIVE=OFF', '-DBUILD_TRAINING_TOOLS=OFF', '-DBUILD_TESTS=OFF', '-DDISABLE_TIFF=ON', '-DDISABLE_ARCHIVE=ON', '-DDISABLE_CURL=ON', '-DENABLE_PRECOMPILED_HEADERS=OFF')
Invoke-CMake @('--build', $tessBuild, '--config', 'Release', '--target', 'tesseract', '--parallel', '2')
$exe = Join-Path $tessBuild 'bin/Release/tesseract.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'Tesseract executable was not produced.' }
$bytes = [IO.File]::ReadAllBytes($exe)
$machine = [BitConverter]::ToUInt16($bytes, [BitConverter]::ToInt32($bytes, 0x3c) + 4)
$expectedMachine = if ($Arch -eq 'arm64') { 0xAA64 } else { 0x8664 }
if ($machine -ne $expectedMachine) { throw 'OCR executable architecture mismatch.' }
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'tesseract.exe')
foreach ($lang in @('eng', 'fra')) { Copy-Item -LiteralPath (Join-Path $downloads "$lang.traineddata") -Destination (Join-Path $stage "tessdata/$lang.traineddata") }
Copy-Item -LiteralPath (Join-Path $tessSource 'LICENSE') -Destination (Join-Path $stage 'licenses/tesseract.txt')
Copy-Item -LiteralPath (Join-Path $leptSource 'leptonica-license.txt') -Destination (Join-Path $stage 'licenses/leptonica.txt')
# The traineddata repository also uses Apache-2.0.
Copy-Item -LiteralPath (Join-Path $tessSource 'LICENSE') -Destination (Join-Path $stage 'licenses/tessdata-fast.txt')
Copy-Item -LiteralPath (Join-Path $tessSource 'AUTHORS') -Destination (Join-Path $stage 'licenses/AUTHORS.txt')
$copyrights = [Text.StringBuilder]::new()
$null = $copyrights.AppendLine('Tesseract upstream source copyright notices (Apache-2.0; see tesseract.txt).')
foreach ($source in (Get-ChildItem -LiteralPath (Join-Path $tessSource 'src'), (Join-Path $tessSource 'include') -File -Recurse | Sort-Object FullName)) {
    if ($source.Extension -notin @('.h', '.cpp')) { continue }
    $lines = [IO.File]::ReadAllLines($source.FullName) | Where-Object { $_ -match '(?i)copyright' }
    if ($lines) {
        $null = $copyrights.AppendLine($source.FullName.Substring($tessSource.Length + 1).Replace('\', '/'))
        foreach ($line in $lines) { $null = $copyrights.AppendLine($line.Trim()) }
    }
}
[IO.File]::WriteAllText((Join-Path $stage 'licenses/COPYRIGHTS.txt'), $copyrights.ToString(), [Text.UTF8Encoding]::new($false))
$notice = "Tesseract 5.5.3: https://github.com/tesseract-ocr/tesseract`nLeptonica 1.87.0: https://github.com/DanBloomberg/leptonica`nEnglish/French tessdata_fast: https://github.com/tesseract-ocr/tessdata_fast/tree/87416418657359cb625c412a48b6e1d6d41c29bd`nStatic MSVC runtime; PNM input only; no network support, training tools or legacy OCR engine.`nSource patches: explicit stdint.h include in ccstruct/image.h; MSVC /MP capped at two jobs.`n"
[IO.File]::WriteAllText((Join-Path $stage 'NOTICE.txt'), $notice, [Text.UTF8Encoding]::new($false))
$checksums = foreach ($file in (Get-ChildItem -LiteralPath $stage -File -Recurse | Where-Object { $_.Name -notin @('VERSION', 'SHA256SUMS') } | Sort-Object FullName)) { (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash + '  ' + $file.FullName.Substring($stage.Length + 1).Replace('\', '/') }
[IO.File]::WriteAllLines((Join-Path $stage 'SHA256SUMS'), $checksums, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText($marker, $version, [Text.UTF8Encoding]::new($false))
Write-Host "OCR runtime ready: $stage"
