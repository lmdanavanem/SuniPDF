# Real public research PDFs for QA only; never copy these downloads into application packages.
param([switch]$SelfTest, [switch]$Interop)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$corpusName = if ($Interop) { 'interop-corpus' } else { 'regression-corpus' }
$stage = Join-Path $root "target/$corpusName"
$manifest = [IO.File]::ReadAllText((Join-Path $PSScriptRoot "$corpusName.json")) | ConvertFrom-Json
function Test-CorpusSource([Uri]$Url, [bool]$InteropMode, [string]$Revision) {
    if ($Url.Scheme -ne 'https' -or $Url.UserInfo) { return $false }
    if ($InteropMode) {
        return $Revision -cmatch '^[a-f0-9]{40}$' -and $Url.Host -eq 'raw.githubusercontent.com' -and
            $Url.AbsolutePath.StartsWith("/pdf-association/pdf20examples/$Revision/", [StringComparison]::Ordinal)
    }
    return $Url.Host -in @('nvlpubs.nist.gov', 'tf.nist.gov', 'tsapps.nist.gov')
}
function Assert-CorpusFile([string]$Path, [long]$Bytes, [string]$Sha256, [int]$HeaderOffset = 0) {
    if ($HeaderOffset -lt 0 -or $HeaderOffset -gt 1024 -or $HeaderOffset + 5 -gt $Bytes) { throw 'Invalid pinned PDF header offset.' }
    if ((Get-Item -LiteralPath $Path).Length -ne $Bytes) { throw 'Corpus file size mismatch.' }
    $stream = [IO.File]::OpenRead($Path)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $hash = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '')
        if ($hash -ne $Sha256) { throw 'Corpus file checksum mismatch.' }
        $stream.Position = $HeaderOffset
        $header = New-Object byte[] 5
        if ($stream.Read($header, 0, 5) -ne 5 -or [Text.Encoding]::ASCII.GetString($header) -ne '%PDF-') {
            throw 'Corpus download is not a PDF.'
        }
    } finally { $stream.Dispose(); $sha.Dispose() }
}
if ($SelfTest) {
    $revision = 'a' * 40
    if (-not (Test-CorpusSource 'https://nvlpubs.nist.gov/example.pdf' $false '') -or
        -not (Test-CorpusSource "https://raw.githubusercontent.com/pdf-association/pdf20examples/$revision/example.pdf" $true $revision)) { throw 'Pinned corpus origins were refused.' }
    foreach ($url in @('http://nvlpubs.nist.gov/example.pdf', 'https://nvlpubs.nist.gov.attacker.invalid/example.pdf', 'https://user@nvlpubs.nist.gov/example.pdf')) {
        if (Test-CorpusSource $url $false '') { throw 'An unsafe research-corpus origin was accepted.' }
    }
    foreach ($url in @("https://raw.githubusercontent.com/other/pdf20examples/$revision/example.pdf", "https://raw.githubusercontent.com/pdf-association/pdf20examples/main/example.pdf", "https://raw.githubusercontent.com/pdf-association/pdf20examples/$revision/../other/example.pdf")) {
        if (Test-CorpusSource $url $true $revision) { throw 'An unpinned interoperability origin was accepted.' }
    }
    $temp = [IO.Path]::GetTempFileName()
    try {
        [IO.File]::WriteAllText($temp, '%PDF-corpus self-test')
        $bytes = (Get-Item -LiteralPath $temp).Length
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $hash = [BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($temp))).Replace('-', '') } finally { $sha.Dispose() }
        Assert-CorpusFile $temp $bytes $hash
        foreach ($bad in @(@{bytes=$bytes;hash=('0' * 64)}, @{bytes=($bytes + 1);hash=$hash})) {
            $rejected = $false
            try { Assert-CorpusFile $temp $bad.bytes $bad.hash } catch { $rejected = $true }
            if (-not $rejected) { throw 'Corrupt corpus input was accepted.' }
        }
        [IO.File]::WriteAllText($temp, 'prefix %PDF-offset self-test')
        $bytes = (Get-Item -LiteralPath $temp).Length
        $hash = (Get-FileHash -LiteralPath $temp -Algorithm SHA256).Hash
        Assert-CorpusFile $temp $bytes $hash 7
        foreach ($offset in @(-1, 0, 6, 1025)) {
            $rejected = $false
            try { Assert-CorpusFile $temp $bytes $hash $offset } catch { $rejected = $true }
            if (-not $rejected) { throw 'An invalid PDF header pin was accepted.' }
        }
        Write-Host 'Real corpus size/checksum self-test passed.'
    } finally { Remove-Item -LiteralPath $temp }
    exit 0
}
$expectedCount = if ($Interop) { 7 } else { 4 }
if ($manifest.version -ne 1 -or @($manifest.documents).Count -ne $expectedCount -or
    ($Interop -and $manifest.upstream_revision -notmatch '^[a-f0-9]{40}$')) { throw 'Unexpected corpus manifest.' }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
$attribution = @('QA copies only. Original sources remain unchanged.', $manifest.license_policy_url, $manifest.copying_policy_url, $manifest.library_policy_url, '')
foreach ($entry in $manifest.documents) {
    $url = [Uri]$entry.url
    $sourceAllowed = Test-CorpusSource $url ([bool]$Interop) $manifest.upstream_revision
    $headerOffset = if ($Interop) { [int]$entry.header_offset } else { 0 }
    if ($url.Scheme -ne 'https' -or -not $sourceAllowed -or
        $entry.filename -notmatch '^[a-z0-9-]+\.pdf$' -or $entry.sha256 -notmatch '^[A-F0-9]{64}$' -or
        $entry.bytes -le 5 -or $entry.bytes -gt 16MB -or -not $entry.license_evidence) { throw 'Invalid corpus pin or provenance.' }
    $file = Join-Path $stage $entry.filename
    if (-not (Test-Path -LiteralPath $file)) {
        $part = "$file.part"
        try {
            & curl.exe --location --fail --silent --show-error --retry 2 --connect-timeout 20 --max-time 120 --max-filesize $entry.bytes --proto '=https' --proto-redir '=https' --output $part $entry.url
            if ($LASTEXITCODE -ne 0) { throw "Corpus download failed: $($entry.id)" }
            Assert-CorpusFile $part $entry.bytes $entry.sha256 $headerOffset
            Move-Item -LiteralPath $part -Destination $file
        } finally { if (Test-Path -LiteralPath $part) { Remove-Item -LiteralPath $part } }
    }
    Assert-CorpusFile $file $entry.bytes $entry.sha256 $headerOffset
    $year = if ($entry.year) { " ($($entry.year))" } else { '' }
    $attribution += "$($entry.authors)$year, $($entry.title). $($entry.url)"
    $attribution += if ($Interop) { 'PDF Association example files, CC BY-SA 4.0. QA derivatives retain this attribution and license; they are not application payloads.' } else { 'Republished courtesy of the National Institute of Standards and Technology.' }
    Write-Host "Verified real QA document: $($entry.id) ($($entry.page_count) pages)."
}
[IO.File]::WriteAllLines((Join-Path $stage 'ATTRIBUTION.txt'), $attribution, [Text.UTF8Encoding]::new($false))
$variable = if ($Interop) { 'SUNIPDF_INTEROP_CORPUS' } else { 'SUNIPDF_REAL_CORPUS' }
if ($env:GITHUB_ENV) { Add-Content -LiteralPath $env:GITHUB_ENV -Value "$variable=$stage" -Encoding utf8 }
Write-Host "Verified $expectedCount licensed QA documents: $stage"
