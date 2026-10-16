# Real public research PDFs for QA only; never copy these downloads into application packages.
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$stage = Join-Path $root 'target/regression-corpus'
$manifest = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'regression-corpus.json')) | ConvertFrom-Json
function Assert-CorpusFile([string]$Path, [long]$Bytes, [string]$Sha256) {
    if ((Get-Item -LiteralPath $Path).Length -ne $Bytes) { throw 'Corpus file size mismatch.' }
    $stream = [IO.File]::OpenRead($Path)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $hash = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '')
        if ($hash -ne $Sha256) { throw 'Corpus file checksum mismatch.' }
        $stream.Position = 0
        $header = New-Object byte[] 5
        if ($stream.Read($header, 0, 5) -ne 5 -or [Text.Encoding]::ASCII.GetString($header) -ne '%PDF-') {
            throw 'Corpus download is not a PDF.'
        }
    } finally { $stream.Dispose(); $sha.Dispose() }
}
if ($SelfTest) {
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
        Write-Host 'Real corpus size/checksum self-test passed.'
    } finally { Remove-Item -LiteralPath $temp }
    exit 0
}
if ($manifest.version -ne 1 -or @($manifest.documents).Count -ne 4) { throw 'Unexpected corpus manifest.' }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
$attribution = @('QA copies only. Original sources remain unchanged.', $manifest.license_policy_url, $manifest.copying_policy_url, $manifest.library_policy_url, '')
foreach ($entry in $manifest.documents) {
    $url = [Uri]$entry.url
    if ($url.Scheme -ne 'https' -or $url.Host -notin @('nvlpubs.nist.gov', 'tf.nist.gov', 'tsapps.nist.gov') -or
        $entry.filename -notmatch '^[a-z0-9-]+\.pdf$' -or $entry.sha256 -notmatch '^[A-F0-9]{64}$' -or
        $entry.bytes -le 5 -or $entry.bytes -gt 16MB -or -not $entry.license_evidence) { throw 'Invalid corpus pin or provenance.' }
    $file = Join-Path $stage $entry.filename
    if (-not (Test-Path -LiteralPath $file)) {
        $part = "$file.part"
        try {
            & curl.exe --location --fail --silent --show-error --retry 2 --connect-timeout 20 --max-time 120 --max-filesize $entry.bytes --proto '=https' --proto-redir '=https' --output $part $entry.url
            if ($LASTEXITCODE -ne 0) { throw "Corpus download failed: $($entry.id)" }
            Assert-CorpusFile $part $entry.bytes $entry.sha256
            Move-Item -LiteralPath $part -Destination $file
        } finally { if (Test-Path -LiteralPath $part) { Remove-Item -LiteralPath $part } }
    }
    Assert-CorpusFile $file $entry.bytes $entry.sha256
    $attribution += "$($entry.authors) ($($entry.year)), $($entry.title). $($entry.url)"
    $attribution += 'Republished courtesy of the National Institute of Standards and Technology.'
    Write-Host "Verified real QA document: $($entry.id) ($($entry.page_count) pages)."
}
[IO.File]::WriteAllLines((Join-Path $stage 'ATTRIBUTION.txt'), $attribution, [Text.UTF8Encoding]::new($false))
if ($env:GITHUB_ENV) { Add-Content -LiteralPath $env:GITHUB_ENV -Value "SUNIPDF_REAL_CORPUS=$stage" -Encoding utf8 }
Write-Host "Verified four licensed public research documents: $stage"
