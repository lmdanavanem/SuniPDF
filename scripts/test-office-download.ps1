# Simulated transfers only: load the downloader definition without executing setup.
param(
    [string]$Source = (Join-Path $PSScriptRoot 'fetch-test-libreoffice.ps1'),
    [string]$ReportPath = ''
)
$ErrorActionPreference = 'Stop'
$savedExitCode = $global:LASTEXITCODE
$parseErrors = $null
$tokens = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($Source, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
foreach ($name in @('Invoke-PinnedOfficeDownload','Assert-PlainOfficePath','Assert-PinnedOfficeInstaller','Get-PinnedOfficeInstaller')) {
    $function = @($ast.FindAll({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true))
    if ($function.Count -ne 1) { throw "Office function is missing or duplicated: $name" }
    . ([scriptblock]::Create($function[0].Extent.Text))
}

$tempDirectory = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempDirectory ('sunipdf-office-download-test-' + [guid]::NewGuid().ToString('N'))))
$null = New-Item -ItemType Directory -Path $testRoot
$script:results = [Collections.Generic.List[object]]::new()
function Start-Sleep { param($Seconds) if ($Seconds -ne 2) { throw 'Unexpected retry delay.' } }
function curl.exe {
    $arguments = @($args)
    if ($script:call -ge $script:steps.Count) { throw 'Unexpected extra network attempt.' }
    $step = $script:steps[$script:call++]
    foreach ($pair in @(@('--retry','0'), @('--max-time','300'), @('--connect-timeout','20'), @('--max-filesize','8'), @('--proto','=https'), @('--proto-redir','=https'), @('--max-redirs','10'))) {
        $index = [Array]::IndexOf($arguments, $pair[0])
        if ($index -lt 0 -or $arguments[$index + 1] -cne $pair[1]) { throw "Missing transfer bound: $($pair[0])" }
    }
    if ($arguments[0] -cne '--disable') { throw 'User curl configuration was not disabled.' }
    $output = $arguments[[Array]::IndexOf($arguments, '--output') + 1]
    $resumeIndex = [Array]::IndexOf($arguments, '--continue-at')
    $offset = if ($resumeIndex -lt 0) { 0L } else { [long]$arguments[$resumeIndex + 1] }
    if ($offset -ne $step.offset) { throw "Wrong resume offset: expected $($step.offset), received $offset" }
    $current = if (Test-Path -LiteralPath $output) { (Get-Item -LiteralPath $output).Length } else { 0 }
    if ($current -ne $offset) { throw 'A partial file was discarded or an unsupported resume was reused.' }
    if ($null -ne $step.content) {
        if ($step.truncate) {
            [IO.File]::WriteAllText($output, $step.content, [Text.Encoding]::ASCII)
        } else {
            $prefix = if ($current) { [IO.File]::ReadAllText($output) } else { '' }
            if (-not $step.content.StartsWith($prefix, [StringComparison]::Ordinal)) { throw 'Mock received a corrupted partial prefix.' }
            $suffix = [Text.Encoding]::ASCII.GetBytes($step.content.Substring($prefix.Length))
            $stream = [IO.File]::Open($output, [IO.FileMode]::Append, [IO.FileAccess]::Write)
            try { $stream.Write($suffix, 0, $suffix.Length) } finally { $stream.Dispose() }
        }
    }
    $global:LASTEXITCODE = $step.exit
    return [string]$step.http
}
function Test-Download {
    param([string]$Name, [object[]]$Steps, [string]$Initial = '', [string]$ExpectedError = '')
    $Refuse = [bool]$ExpectedError
    $script:steps = $Steps
    $script:call = 0
    $path = Join-Path $testRoot ($Name + '.part')
    if ($Initial.Length) { [IO.File]::WriteAllText($path, $Initial, [Text.Encoding]::ASCII) }
    $errorText = $null
    try { Invoke-PinnedOfficeDownload -Url 'https://example.invalid/pinned.msi' -Destination $path -ExpectedBytes 8 }
    catch { $errorText = $_.Exception.Message }
    if ($Refuse -and -not $errorText) { throw "Case should have refused: $Name" }
    if ($Refuse -and $errorText -notlike ($ExpectedError + '*')) { throw "Wrong refusal for ${Name}: $errorText" }
    if (-not $Refuse -and $errorText) { throw "Case failed: ${Name}: $errorText" }
    if ($script:call -ne $Steps.Count) { throw "Wrong attempt count: $Name" }
    if (-not $Refuse -and [IO.File]::ReadAllText($path) -cne '12345678') { throw "Downloaded content differs: $Name" }
    $script:results.Add([pscustomobject]@{case=$Name; passed=$true; calls=$script:call; refused=[bool]$Refuse})
}
function Get-FileHash {
    param([string]$LiteralPath, [string]$Algorithm)
    $script:hashChecks++
    Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $LiteralPath -Algorithm $Algorithm
}
function Get-AuthenticodeSignature {
    param([string]$LiteralPath)
    if (-not (Test-Path -LiteralPath $LiteralPath -PathType Leaf)) { throw 'Signature checked before staging the installer.' }
    $script:signatureChecks++
    return [pscustomobject]@{Status=$script:signatureStatus; SignerCertificate=[pscustomobject]@{Subject=$script:publisher}}
}
function Test-InstallerCache {
    param([string]$Name, [string]$Initial, [switch]$Miss, [string]$SignatureStatus='Valid',
        [string]$Publisher='CN=Publisher, O=The Document Foundation, C=DE', [string]$ExpectedError='')
    $directory = Join-Path $testRoot $Name
    $cache = Join-Path $directory 'cache/installer.msi'
    $stage = Join-Path $directory 'stage/installer.msi'
    [void][IO.Directory]::CreateDirectory((Split-Path -Parent $stage))
    if (-not $Miss) {
        [void][IO.Directory]::CreateDirectory((Split-Path -Parent $cache))
        [IO.File]::WriteAllText($cache, $Initial, [Text.Encoding]::ASCII)
    }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $hash = [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::ASCII.GetBytes('12345678'))).Replace('-','') }
    finally { $sha.Dispose() }
    $script:steps = @()
    if ($Miss) { $script:steps = @(@{offset=0;content='12345678';exit=0;http='200'}) }
    $script:call = 0; $script:hashChecks = 0; $script:signatureChecks = 0
    $script:signatureStatus = $SignatureStatus; $script:publisher = $Publisher
    $errorText = $null; $verified = $null
    try { $verified = Get-PinnedOfficeInstaller -Url 'https://example.invalid/pinned.msi' -Destination $stage -CachePath $cache -ExpectedBytes 8 -ExpectedSha256 $hash }
    catch { $errorText = $_.Exception.Message }
    if ($ExpectedError) {
        if (-not $errorText -or $errorText -notlike ($ExpectedError + '*')) { throw "Wrong cache refusal for ${Name}: $errorText" }
        if ($Miss -and (Test-Path -LiteralPath $cache)) { throw 'Unverified installer was cached.' }
    } else {
        if ($errorText) { throw "Cache case failed: ${Name}: $errorText" }
        if ($verified.cache_hit -ne (-not $Miss) -or $script:hashChecks -ne 1 -or $script:signatureChecks -ne 1 -or
            [IO.File]::ReadAllText($stage) -cne '12345678' -or [IO.File]::ReadAllText($cache) -cne '12345678') { throw "Installer cache bypassed verification or changed bytes: $Name" }
    }
    if ($script:call -ne $script:steps.Count) { throw "Wrong cache download count: $Name" }
    if (-not $Miss -and [IO.File]::ReadAllText($cache) -cne $Initial) { throw 'An existing cache was changed.' }
    $script:results.Add([pscustomobject]@{case=$Name;passed=$true;calls=$script:call;refused=[bool]$ExpectedError;hash_checks=$script:hashChecks;signature_checks=$script:signatureChecks})
}
function Test-InstallerPath {
    param([string]$Name, [switch]$LinkedStage, [switch]$ReusedStage)
    $directory = Join-Path $testRoot $Name
    [void][IO.Directory]::CreateDirectory($directory)
    $cacheDirectory = Join-Path $directory 'cache'
    $stageDirectory = Join-Path $directory 'stage'
    $linked = if ($LinkedStage) { $stageDirectory } else { $cacheDirectory }
    $linkTarget = Join-Path $directory 'link-target'
    [void][IO.Directory]::CreateDirectory($linkTarget)
    if ($ReusedStage) {
        [void][IO.Directory]::CreateDirectory($stageDirectory)
        [IO.File]::WriteAllText((Join-Path $stageDirectory 'installer.msi'), 'preserved', [Text.Encoding]::ASCII)
    } else {
        $null = New-Item -ItemType Junction -Path $linked -Target $linkTarget
    }
    $script:steps = @(); $script:call = 0
    $errorText = $null
    try {
        try { $null = Get-PinnedOfficeInstaller -Url 'https://example.invalid/pinned.msi' -Destination (Join-Path $stageDirectory 'installer.msi') -CachePath (Join-Path $cacheDirectory 'installer.msi') -ExpectedBytes 8 -ExpectedSha256 ('A'*64) }
        catch { $errorText = $_.Exception.Message }
        $expected = if ($ReusedStage) { 'Converter installer stage must be a new file separate from the cache.' } else { 'Converter installer path contains a reparse point.' }
        if ($errorText -cne $expected -or $script:call -ne 0 -or @(Get-ChildItem -LiteralPath $linkTarget -Force).Count) { throw "Unsafe installer path was not refused: $Name ($errorText)" }
        if ($ReusedStage -and [IO.File]::ReadAllText((Join-Path $stageDirectory 'installer.msi')) -cne 'preserved') { throw 'An existing stage file was changed.' }
        $script:results.Add([pscustomobject]@{case=$Name;passed=$true;calls=0;refused=$true})
    } finally {
        if (-not $ReusedStage) {
            if (-not [IO.Path]::GetFullPath($linked).StartsWith($testRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Test junction cleanup escaped its temporary directory.' }
            [IO.Directory]::Delete($linked)
        }
    }
}

try {
    Test-Download fresh @(@{offset=0;content='12345678';exit=0;http='200'})
    Test-Download resume @(@{offset=0;content='123';exit=28;http='200'}, @{offset=3;content='123456';exit=28;http='206'}, @{offset=6;content='12345678';exit=0;http='206'})
    Test-Download unsupported_range @(@{offset=0;content='123';exit=28;http='200'}, @{offset=3;content=$null;exit=33;http='200'}, @{offset=0;content='12345678';exit=0;http='200'})
    Test-Download transient_http @(@{offset=0;content=$null;exit=22;http='503'}, @{offset=0;content='12345678';exit=0;http='200'})
    Test-Download connection_loss @(@{offset=0;content='123';exit=56;http='200'}, @{offset=3;content='12345678';exit=0;http='206'})
    Test-Download exhausted @(@{offset=0;content=$null;exit=28;http='000'}, @{offset=0;content=$null;exit=28;http='000'}, @{offset=0;content=$null;exit=28;http='000'}) -ExpectedError 'Pinned LibreOffice download failed after three bounded attempts;'
    Test-Download not_found @(@{offset=0;content=$null;exit=22;http='404'}) -ExpectedError 'Pinned LibreOffice download failed (curl 22, HTTP 404).'
    Test-Download invalid_tls @(@{offset=0;content=$null;exit=60;http='000'}) -ExpectedError 'Pinned LibreOffice download failed (curl 60, HTTP 000).'
    Test-Download oversized_response @(@{offset=0;content='123456789';exit=0;http='200'}) -ExpectedError 'Converter partial download size is invalid.'
    Test-Download incomplete_success @(@{offset=0;content='123';exit=0;http='200'}) -ExpectedError 'Converter response did not complete the expected transfer.'
    Test-Download wrong_resume_status @(@{offset=3;content='12345678';exit=0;http='200'}) -Initial '123' -ExpectedError 'Converter response did not complete the expected transfer.'
    Test-Download invalid_http @(@{offset=0;content=$null;exit=0;http='unexpected'}) -ExpectedError 'Converter download returned an invalid HTTP status.'
    Test-Download complete @() -Initial '12345678'
    Test-Download oversized_initial @() -Initial '123456789' -ExpectedError 'Converter download exceeds its pinned size.'
    Test-Download truncated_partial @(@{offset=3;content='1';truncate=$true;exit=28;http='206'}) -Initial '123' -ExpectedError 'Converter partial download size is invalid.'
    Test-Download one_restart_only @(@{offset=3;content=$null;exit=33;http='200'}, @{offset=0;content='123';exit=28;http='200'}, @{offset=3;content=$null;exit=33;http='200'}) -Initial '123' -ExpectedError 'Pinned LibreOffice download failed (curl 33, HTTP 200).'
    Test-InstallerCache cache_miss -Miss
    Test-InstallerCache cache_hit -Initial '12345678'
    Test-InstallerCache cache_partial -Initial '123' -ExpectedError 'Converter cached MSI size mismatch.'
    Test-InstallerCache cache_oversized -Initial '123456789' -ExpectedError 'Converter cached MSI size mismatch.'
    Test-InstallerCache cache_bad_hash -Initial 'XXXXXXXX' -ExpectedError 'Converter MSI size/checksum mismatch.'
    Test-InstallerCache cache_bad_signature -Initial '12345678' -SignatureStatus 'NotSigned' -ExpectedError 'Converter MSI publisher signature is not valid for The Document Foundation.'
    Test-InstallerCache cache_wrong_publisher -Initial '12345678' -Publisher 'CN=Other, O=Other Publisher' -ExpectedError 'Converter MSI publisher signature is not valid for The Document Foundation.'
    Test-InstallerCache miss_bad_signature -Miss -SignatureStatus 'NotSigned' -ExpectedError 'Converter MSI publisher signature is not valid for The Document Foundation.'
    Test-InstallerPath cache_reparse
    Test-InstallerPath stage_reparse -LinkedStage
    Test-InstallerPath stage_reused -ReusedStage
    $proof = [pscustomobject]@{success=$true; powershell=[string]$PSVersionTable.PSVersion; network_calls=0; installer_executions=0; cases=$script:results.Count; tests=@($script:results.ToArray())}
    if ($ReportPath) { [IO.File]::WriteAllText([IO.Path]::GetFullPath($ReportPath), ($proof | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false)) }
    Write-Host "Office download: $($script:results.Count) simulated transfer cases passed; no network or installer execution."
} finally {
    $global:LASTEXITCODE = $savedExitCode
    $allowedRoot = $tempDirectory + '\'
    if (-not $testRoot.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Test cleanup escaped its temporary directory.' }
    Remove-Item -LiteralPath $testRoot -Recurse -Force
}
