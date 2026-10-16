# QA tool only: official upstream Windows binaries, pinned independently of application packages.
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$version = '12.4.2'
$expected = 'DB87077E683630C1217E0E8F9A20A9749D952AB676E881C3689187763A5DE25D'
$root = Split-Path -Parent $PSScriptRoot
$stage = Join-Path $root "target/qpdf-$version-msvc64"
$archive = Join-Path $stage "qpdf-$version-msvc64.zip"
function Assert-QpdfArchive([string]$Path, [string]$Hash) {
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Hash) { throw 'QPDF archive checksum mismatch.' }
}
if ($SelfTest) {
    $temp = [IO.Path]::GetTempFileName()
    try {
        [IO.File]::WriteAllText($temp, 'qpdf checksum self-test')
        $hash = (Get-FileHash -LiteralPath $temp -Algorithm SHA256).Hash
        Assert-QpdfArchive $temp $hash
        $rejected = $false
        try { Assert-QpdfArchive $temp ('0' * 64) } catch { $rejected = $true }
        if (-not $rejected) { throw 'Incorrect archive hash was accepted.' }
        Write-Host 'QPDF archive checksum self-test passed.'
    } finally { Remove-Item -LiteralPath $temp }
    exit 0
}
New-Item -ItemType Directory -Force -Path $stage | Out-Null
if (-not (Test-Path -LiteralPath $archive)) {
    $url = "https://github.com/qpdf/qpdf/releases/download/v$version/qpdf-$version-msvc64.zip"
    & curl.exe --location --fail --silent --show-error --output $archive $url
    if ($LASTEXITCODE -ne 0) { throw 'QPDF download failed.' }
}
Assert-QpdfArchive $archive $expected
Expand-Archive -LiteralPath $archive -DestinationPath $stage -Force
$exe = Join-Path $stage "qpdf-$version-msvc64/bin/qpdf.exe"
if (-not (Test-Path -LiteralPath $exe)) { throw 'QPDF executable is missing from the pinned distribution.' }
$reported = & $exe --version
if ($LASTEXITCODE -ne 0 -or $reported[0] -ne "qpdf version $version") { throw 'Unexpected QPDF executable version.' }
if ($env:GITHUB_ENV) { Add-Content -LiteralPath $env:GITHUB_ENV -Value "SUNIPDF_QPDF=$exe" -Encoding utf8 }
Write-Host "Verified QPDF $version : $exe"
