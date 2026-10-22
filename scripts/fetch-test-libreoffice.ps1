# Pinned converter QA only; application code never installs a converter.
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$pin = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'test-libreoffice.json')) | ConvertFrom-Json
$url = [Uri]$pin.url
if ($pin.version -ne '26.8.1' -or $pin.filename -ne 'LibreOffice_26.8.1_Win_x86-64.msi' -or
    $url.Scheme -ne 'https' -or $url.Host -ne 'download.documentfoundation.org' -or
    $url.AbsolutePath -ne '/libreoffice/stable/26.8.1/win/x86_64/LibreOffice_26.8.1_Win_x86-64.msi' -or
    $pin.sha256 -notmatch '^[A-F0-9]{64}$' -or $pin.bytes -ne 374870016) { throw 'Invalid converter QA pin.' }
function Assert-DisposableRunner {
    if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or
        $env:RUNNER_OS -ne 'Windows' -or $env:RUNNER_ARCH -ne 'X64' -or
        $env:GITHUB_REPOSITORY -ne 'lmdanavanem/SuniPDF' -or -not $env:RUNNER_TEMP -or
        $env:GITHUB_SHA -notmatch '^[a-f0-9]{40}$') {
        throw 'Converter installation is restricted to the disposable GitHub-hosted Windows x64 runner for this repository.'
    }
}
function Invoke-PinnedOfficeDownload {
    param([string]$Url, [string]$Destination, [long]$ExpectedBytes)
    if ($ExpectedBytes -le 0 -or ([Uri]$Url).Scheme -cne 'https') { throw 'Invalid pinned download request.' }
    $Destination = [IO.Path]::GetFullPath($Destination)
    $restartUsed = $false
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        $offset = 0L
        if (Test-Path -LiteralPath $Destination) {
            $file = Get-Item -LiteralPath $Destination -Force
            if ($file.PSIsContainer -or ($file.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Invalid converter download file.' }
            $offset = $file.Length
        }
        if ($offset -gt $ExpectedBytes) { throw 'Converter download exceeds its pinned size.' }
        if ($offset -eq $ExpectedBytes) { return }
        $arguments = @('--disable', '--location', '--fail', '--silent', '--show-error', '--retry', '0',
            '--connect-timeout', '20', '--max-time', '300', '--max-filesize', [string]$ExpectedBytes,
            '--max-redirs', '10', '--proto', '=https', '--proto-redir', '=https',
            '--output', $Destination, '--write-out', '%{http_code}')
        if ($offset -gt 0) { $arguments += @('--continue-at', [string]$offset) }
        $arguments += $Url
        # Keep the partial file between bounded attempts; curl's internal retry resets it.
        $PSNativeCommandUseErrorActionPreference = $false
        $http = [string](& curl.exe @arguments)
        $exitCode = $LASTEXITCODE
        if ($http -notmatch '^\d{3}$') { throw 'Converter download returned an invalid HTTP status.' }
        $received = if (Test-Path -LiteralPath $Destination) { (Get-Item -LiteralPath $Destination).Length } else { 0L }
        if ($received -gt $ExpectedBytes -or $received -lt $offset) { throw 'Converter partial download size is invalid.' }
        if ($exitCode -eq 0) {
            if ($received -ne $ExpectedBytes -or [int]$http -notin @(200,206) -or ($offset -gt 0 -and [int]$http -ne 206)) { throw 'Converter response did not complete the expected transfer.' }
            return
        }
        if ($exitCode -eq 33 -and $offset -gt 0 -and -not $restartUsed) {
            # A mirror without Range support may restart once within the same three-attempt bound.
            Remove-Item -LiteralPath $Destination -Force
            $restartUsed = $true
        } elseif ($exitCode -notin @(6,7,18,28,52,55,56,92) -and
            -not ($exitCode -eq 22 -and [int]$http -in @(408,429,500,502,503,504,522,524))) {
            throw "Pinned LibreOffice download failed (curl $exitCode, HTTP $http)."
        }
        if ($attempt -lt 3) { Start-Sleep -Seconds 2 }
    }
    throw 'Pinned LibreOffice download failed after three bounded attempts; partial data was not installed.'
}
function Assert-PlainOfficePath {
    param([string]$Path)
    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        $item = Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue
        if ($null -ne $item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Converter installer path contains a reparse point.' }
        $parent = [IO.Path]::GetDirectoryName($current)
        if ($parent -ceq $current) { break }
        $current = $parent
    }
}
function Assert-PinnedOfficeInstaller {
    param([string]$Path, [long]$ExpectedBytes, [string]$ExpectedSha256)
    Assert-PlainOfficePath -Path $Path
    $file = Get-Item -LiteralPath $Path -Force
    if ($file.PSIsContainer -or $file.Length -ne $ExpectedBytes -or
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -cne $ExpectedSha256) { throw 'Converter MSI size/checksum mismatch.' }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate -or
        $signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=The Document Foundation(,|$)') { throw 'Converter MSI publisher signature is not valid for The Document Foundation.' }
    return $signature
}
function Get-PinnedOfficeInstaller {
    param([string]$Url, [string]$Destination, [string]$CachePath, [long]$ExpectedBytes, [string]$ExpectedSha256)
    if ($ExpectedBytes -le 0 -or $ExpectedSha256 -cnotmatch '^[A-F0-9]{64}$') { throw 'Invalid converter installer verification pin.' }
    $Destination = [IO.Path]::GetFullPath($Destination)
    $CachePath = [IO.Path]::GetFullPath($CachePath)
    Assert-PlainOfficePath -Path $Destination
    Assert-PlainOfficePath -Path $CachePath
    if ($Destination -ieq $CachePath -or (Test-Path -LiteralPath $Destination)) { throw 'Converter installer stage must be a new file separate from the cache.' }
    $cacheHit = Test-Path -LiteralPath $CachePath
    if ($cacheHit) {
        $cached = Get-Item -LiteralPath $CachePath -Force
        if ($cached.PSIsContainer -or $cached.Length -ne $ExpectedBytes) { throw 'Converter cached MSI size mismatch.' }
        Write-Host 'Using cached pinned Office MSI; rechecking size, SHA-256 and publisher signature.'
        [IO.File]::Copy($CachePath, $Destination, $false)
    } else {
        Write-Host 'Downloading pinned Office MSI; no installer cache is available.'
        Invoke-PinnedOfficeDownload -Url $Url -Destination $Destination -ExpectedBytes $ExpectedBytes
    }
    # Cache presence never bypasses verification of the actual staged installer.
    $signature = Assert-PinnedOfficeInstaller -Path $Destination -ExpectedBytes $ExpectedBytes -ExpectedSha256 $ExpectedSha256
    if (-not $cacheHit) {
        $cacheDirectory = [IO.Path]::GetDirectoryName($CachePath)
        [void][IO.Directory]::CreateDirectory($cacheDirectory)
        Assert-PlainOfficePath -Path $CachePath
        [IO.File]::Copy($Destination, $CachePath, $false)
    }
    return [pscustomobject]@{signature=$signature; cache_hit=[bool]$cacheHit}
}
if ($SelfTest) {
    $names = @('GITHUB_ACTIONS','RUNNER_ENVIRONMENT','RUNNER_OS','RUNNER_ARCH','GITHUB_REPOSITORY','RUNNER_TEMP','GITHUB_SHA')
    $saved = @{}
    foreach ($name in $names) { $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
    try {
        [Environment]::SetEnvironmentVariable('GITHUB_ACTIONS', 'false', 'Process')
        $refused = $false
        try { Assert-DisposableRunner } catch { $refused = $true }
        if (-not $refused) { throw 'Host converter installation was not refused.' }
    } finally { foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') } }
    & (Join-Path $PSScriptRoot 'test-office-download.ps1') -Source $PSCommandPath
    Write-Host 'LibreOffice QA pin and development-host refusal passed. No installer downloaded or executed.'
    exit 0
}
Assert-DisposableRunner
if (Test-Path -LiteralPath (Join-Path $env:ProgramFiles 'LibreOffice/program/soffice.com')) { throw 'Refusing to replace an existing converter.' }
$tempRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\')
$stage = [IO.Path]::GetFullPath((Join-Path $tempRoot "sunipdf-office-qa-$($env:GITHUB_RUN_ID)-$($env:GITHUB_RUN_ATTEMPT)"))
if (-not $stage.StartsWith($tempRoot + '\', [StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $stage)) { throw 'Unsafe or reused converter QA directory.' }
if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($stage)).AvailableFreeSpace -lt 3GB) { throw 'Converter QA requires 3 GiB free.' }
New-Item -ItemType Directory -Path $stage | Out-Null
$installer = Join-Path $stage $pin.filename
$cachePath = Join-Path $root 'target/test-office-installer/installer.msi'
$verified = Get-PinnedOfficeInstaller -Url $pin.url -Destination $installer -CachePath $cachePath -ExpectedBytes $pin.bytes -ExpectedSha256 $pin.sha256
$signature = $verified.signature
$log = Join-Path $stage 'installation.log'
Write-Host 'Installing the verified pinned Office MSI in the disposable runner.'
$process = Start-Process -FilePath (Join-Path ([Environment]::SystemDirectory) 'msiexec.exe') -ArgumentList @('/i', "`"$installer`"", '/qn', '/norestart', '/L*v', "`"$log`"") -PassThru -WindowStyle Hidden
if (-not $process.WaitForExit(600000)) { $process.Kill(); throw 'Converter installation exceeded ten minutes.' }
if ($process.ExitCode -notin @(0,3010)) { throw "Converter installation failed with exit code $($process.ExitCode)." }
$converter = Join-Path $env:ProgramFiles 'LibreOffice/program/soffice.com'
if (-not (Test-Path -LiteralPath $converter)) { throw 'Pinned converter is not installed at its standard path.' }
$proof = [ordered]@{
    source_revision = $env:GITHUB_SHA
    workflow_run = $env:GITHUB_RUN_ID
    converter_version = $pin.version
    installer_sha256 = $pin.sha256
    installer_bytes = $pin.bytes
    installer_cache_hit = $verified.cache_hit
    source_url = $pin.url
    metadata_url = $pin.metadata_url
    authenticode_status = [string]$signature.Status
    publisher_subject = $signature.SignerCertificate.Subject
    publisher_thumbprint = $signature.SignerCertificate.Thumbprint
    installed_path = $converter
    installation_exit_code = $process.ExitCode
    scope = 'disposable GitHub-hosted runner only'
}
$target = Join-Path $root 'target'
New-Item -ItemType Directory -Path $target -Force | Out-Null
$proofPath = Join-Path $target 'office-converter-provenance.json'
if (Test-Path -LiteralPath $proofPath) { throw 'Converter provenance must be a new report.' }
[IO.File]::WriteAllText($proofPath, ($proof | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
Write-Host "Pinned LibreOffice $($pin.version) installed for disposable-runner QA."
