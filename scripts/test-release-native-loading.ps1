param([string]$Directory = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Directory) {
    $version = (Select-String -LiteralPath (Join-Path $root 'Cargo.toml') -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
    $Directory = Join-Path $root "dist/SuniPDF-$version-win-x64-portable"
}
$package = (Resolve-Path -LiteralPath $Directory).Path
$fixture = Join-Path $root 'crates/sunipdf-core/tests/fixtures/two-pages.pdf'
$cli = Join-Path $package 'sunipdf-cli.exe'
$positive = & $cli info $fixture
if ($LASTEXITCODE -ne 0 -or ($positive | ConvertFrom-Json).page_count -ne 2) {
    throw 'Packaged CLI could not load its adjacent PDFium library.'
}
$isolated = Join-Path $root ('target/native-loading-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($isolated) | Out-Null
$isolatedCli = Join-Path $isolated 'sunipdf-cli.exe'
Copy-Item -LiteralPath $cli -Destination $isolatedCli
$previousOverride = $env:SUNIPDF_PDFIUM_DIR
$stderr = Join-Path $isolated 'stderr.json'
try {
    $env:SUNIPDF_PDFIUM_DIR = $package
    $process = Start-Process -FilePath $isolatedCli -ArgumentList ('info "' + $fixture + '"') -WorkingDirectory $package -WindowStyle Hidden -Wait -PassThru -RedirectStandardError $stderr
    $errorReport = [IO.File]::ReadAllText($stderr) | ConvertFrom-Json
    if ($process.ExitCode -eq 0 -or $errorReport.error -ne 'library_not_found') {
        throw 'Release CLI accepted an environment/current-directory native library override.'
    }
} finally {
    $env:SUNIPDF_PDFIUM_DIR = $previousOverride
}
Write-Host 'Release native loading: adjacent library accepted; environment and working-directory overrides ignored.'
