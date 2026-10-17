# Official pinned independent reader for tests only; never copied into dist or third_party.
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$version = '1.28.5'
$sha256 = '13476a53e84f5527983158acb6442c7853b3cdc022f3b24880780dcfba1fcc99'
$url = "https://github.com/ArtifexSoftware/mupdf-downloads/releases/download/$version/mupdf-$version-windows.zip"
$cache = Join-Path $root "target/test-mupdf/$version"
function Assert-MuPdfArchive([string]$Path, [string]$Expected) {
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Expected) { throw 'Independent MuPDF archive checksum mismatch.' }
}
if ($SelfTest) {
    $test = Join-Path $root ('target/mupdf-checksum-' + [guid]::NewGuid().ToString('N') + '.txt')
    New-Item -ItemType Directory -Force (Split-Path -Parent $test) | Out-Null
    [IO.File]::WriteAllText($test, 'test-only checksum fixture', [Text.UTF8Encoding]::new($false))
    $expected = (Get-FileHash -LiteralPath $test -Algorithm SHA256).Hash
    Assert-MuPdfArchive $test $expected
    [IO.File]::AppendAllText($test, 'changed')
    $rejected = $false
    try { Assert-MuPdfArchive $test $expected } catch { $rejected = $true }
    if (-not $rejected) { throw 'MuPDF checksum self-test did not reject modified data.' }
    Write-Host 'MuPDF archive checksum self-test passed.'
    exit 0
}
New-Item -ItemType Directory -Force $cache | Out-Null
$archive = Join-Path $cache "mupdf-$version-windows.zip"
if (-not (Test-Path -LiteralPath $archive)) {
    & curl.exe --location --fail --silent --show-error --output $archive $url
    if ($LASTEXITCODE -ne 0) { throw 'Official independent MuPDF download failed.' }
}
Assert-MuPdfArchive $archive $sha256
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
try {
    foreach ($name in @('mutool.exe', 'COPYING.txt', 'README.txt')) {
        $matches = @($zip.Entries | Where-Object { $_.FullName -eq "mupdf-$version-windows/$name" })
        if ($matches.Count -ne 1 -or $matches[0].Length -gt 128MB) { throw "Missing or ambiguous independent-reader resource: $name" }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($matches[0], (Join-Path $cache $name), $true)
    }
} finally { $zip.Dispose() }
$provenance = [ordered]@{ schema = 1; version = $version; source = $url; archiveSha256 = $sha256; digestSource = 'https://api.github.com/repos/ArtifexSoftware/mupdf-downloads/releases/tags/1.28.5'; scope = 'Independent test reader only; not linked into SuniPDF and not shipped in application packages'; license = 'AGPL-3.0 terms in COPYING.txt'; files = @(foreach ($name in @('mutool.exe', 'COPYING.txt', 'README.txt')) { [ordered]@{ name = $name; sha256 = (Get-FileHash -LiteralPath (Join-Path $cache $name) -Algorithm SHA256).Hash.ToLowerInvariant() } }) }
[IO.File]::WriteAllText((Join-Path $cache 'provenance.json'), ($provenance | ConvertTo-Json -Depth 6) + "`n", [Text.UTF8Encoding]::new($false))
Write-Host "Verified independent MuPDF test reader: $cache"
