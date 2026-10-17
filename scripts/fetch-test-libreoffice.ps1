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
& curl.exe --location --fail --silent --show-error --retry 2 --connect-timeout 20 --max-time 300 --max-filesize $pin.bytes --proto '=https' --proto-redir '=https' --output $installer $pin.url
if ($LASTEXITCODE -ne 0) { throw 'Pinned LibreOffice download failed.' }
if ((Get-Item -LiteralPath $installer).Length -ne $pin.bytes -or (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ne $pin.sha256) { throw 'Converter MSI size/checksum mismatch.' }
$signature = Get-AuthenticodeSignature -LiteralPath $installer
if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate -or $signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=The Document Foundation(,|$)') { throw 'Converter MSI publisher signature is not valid for The Document Foundation.' }
$log = Join-Path $stage 'installation.log'
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
