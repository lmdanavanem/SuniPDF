# Offline transport doubles exercise cache behavior without downloading or installing applications.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'upgrade-baseline-cache.ps1')
$workspace = Join-Path ([IO.Path]::GetTempPath()) ('SuniPDF-baseline-cache-test-' + [guid]::NewGuid().ToString('N'))
$cache = Join-Path $workspace 'cache'
$outside = Join-Path $workspace 'outside'
[IO.Directory]::CreateDirectory($workspace) | Out-Null
$fixture = [Text.Encoding]::UTF8.GetBytes('Pinned historical package fixture')
$fixturePath = Join-Path $workspace 'fixture'
[IO.File]::WriteAllBytes($fixturePath, $fixture)
$hash = Get-SuniBaselineDigest $fixturePath
$script:sidecarCalls = 0
$script:archiveCalls = 0
$script:sidecarMode = 'valid'
$script:archiveMode = 'valid'
$script:observedUris = [Collections.Generic.List[string]]::new()
function Assert([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Reject([scriptblock]$Action, [string]$Message) {
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    Assert $rejected $Message
}
function Read-SuniBaselineSidecar([string]$Uri) {
    $script:sidecarCalls++
    $script:observedUris.Add($Uri)
    $name = [IO.Path]::GetFileName(([uri]$Uri).AbsolutePath).Replace('.sha256', '')
    if ($script:sidecarMode -eq 'bad-hash') { return (('0' * 64) + '  ' + $name) }
    if ($script:sidecarMode -eq 'bad-name') { return ($hash + '  unexpected.zip') }
    if ($script:sidecarMode -eq 'extra-token') { return ($hash + '  ' + $name + ' unexpected') }
    return ($hash + '  ' + $name)
}
function Save-SuniBaselineArchive([string]$Uri, [string]$Destination) {
    $script:archiveCalls++
    $script:observedUris.Add($Uri)
    if ($script:archiveMode -eq 'partial-error') {
        [IO.File]::WriteAllText($Destination, 'Incomplete download')
        throw 'Simulated interrupted transport'
    }
    if ($script:archiveMode -eq 'bad-bytes') { [IO.File]::WriteAllText($Destination, 'Wrong package') }
    else { [IO.File]::WriteAllBytes($Destination, $fixture) }
}
$baselineArgs = @{ Version='0.1.0'; CurrentVersion='0.2.0'; ExpectedSha256=$hash; Kind='portable'; CacheDirectory=$cache }
$cases = [Collections.Generic.List[string]]::new()
$junction = Join-Path $workspace 'junction-cache'
try {
    $portablePins = Read-SuniBaselinePins (Join-Path $PSScriptRoot 'test-portable-update.ps1') 'pins'
    $msiPins = Read-SuniBaselinePins (Join-Path $PSScriptRoot 'test-msi-upgrade.ps1') 'baselines'
    Assert ($portablePins.Contains('0.76.0') -and $msiPins.Contains('0.76.0')) 'The latest published baseline is missing.'
    $maliciousPins = Join-Path $workspace 'nonliteral.ps1'
    $sentinel = Join-Path $workspace 'must-not-exist'
    [IO.File]::WriteAllText($maliciousPins, ('$pins = @{ bad = [IO.File]::WriteAllText(''' + $sentinel.Replace("'", "''") + ''', ''executed'') }'))
    Reject { Read-SuniBaselinePins $maliciousPins 'pins' } 'Nonliteral pin data was evaluated.'
    Assert (-not [IO.File]::Exists($sentinel)) 'Reading pin metadata executed script content.'
    $cases.Add('baseline-preparation-reads-literal-pin-data-without-executing-tests')

    $path = Get-SuniUpgradeBaseline @baselineArgs
    Assert ($script:sidecarCalls -eq 1 -and $script:archiveCalls -eq 1) 'A cold cache did not fetch both inputs.'
    Assert ((Get-SuniBaselineDigest $path) -eq $hash) 'A cold cache returned different bytes.'
    Assert ($path.StartsWith([IO.Path]::GetFullPath($cache) + '\')) 'Cache escaped the test root.'
    $cases.Add('cold-cache-downloads-and-promotes-pinned-bytes')

    [IO.File]::WriteAllText("$path.sha256", 'Untrusted local checksum')
    $again = Get-SuniUpgradeBaseline @baselineArgs
    Assert ($again -eq $path -and $script:sidecarCalls -eq 2 -and $script:archiveCalls -eq 1) 'A hit skipped live sidecar checking or downloaded again.'
    Assert ([IO.File]::ReadAllText("$path.sha256") -eq "$hash  SuniPDF-0.1.0-win-x64-portable.zip") 'Local checksum was trusted instead of repaired.'
    $cases.Add('warm-cache-rechecks-published-checksum-and-repairs-local-metadata')

    [IO.File]::WriteAllText($path, 'Corrupted cached archive')
    $again = Get-SuniUpgradeBaseline @baselineArgs
    Assert ($script:archiveCalls -eq 2 -and (Get-SuniBaselineDigest $again) -eq $hash) 'A corrupt hit was not replaced with verified bytes.'
    $cases.Add('corrupted-cache-is-rehashed-and-repaired')

    foreach ($mode in @('bad-hash', 'bad-name', 'extra-token')) {
        $script:sidecarMode = $mode
        Reject { Get-SuniUpgradeBaseline @baselineArgs } "Published sidecar $mode was accepted."
        Assert ($script:archiveCalls -eq 2 -and (Get-SuniBaselineDigest $path) -eq $hash) 'Invalid sidecar modified a valid cached file.'
    }
    $script:sidecarMode = 'valid'
    $cases.Add('published-sidecar-hash-name-and-format-fail-closed-on-cache-hit')

    [IO.File]::WriteAllText($path, 'Existing corrupt entry')
    $corruptHash = Get-SuniBaselineDigest $path
    foreach ($mode in @('bad-bytes', 'partial-error')) {
        $script:archiveMode = $mode
        Reject { Get-SuniUpgradeBaseline @baselineArgs } "Download $mode was promoted."
        Assert ((Get-SuniBaselineDigest $path) -eq $corruptHash) 'Failed download replaced the previous entry.'
        Assert (@(Get-ChildItem -LiteralPath (Split-Path -Parent $path) -Force | Where-Object Name -Like '.*').Count -eq 0) 'Failed download left partial cache files.'
    }
    $script:archiveMode = 'valid'
    $cases.Add('wrong-and-interrupted-downloads-are-never-promoted')

    $before = $script:sidecarCalls
    foreach ($version in @('0.2.0', '0.3.0')) {
        $request = $baselineArgs.Clone(); $request.Version = $version
        Reject { Get-SuniUpgradeBaseline @request } 'A current/future package entered the historical cache.'
    }
    Assert ($script:sidecarCalls -eq $before) 'Current/future versions reached the transport.'
    $cases.Add('current-and-future-packages-are-excluded')

    $request = $baselineArgs.Clone(); $request.Kind = 'msi'
    $msi = Get-SuniUpgradeBaseline @request
    Assert ($msi -ne $path -and $msi.EndsWith('.msi') -and (Get-SuniBaselineDigest $msi) -eq $hash) 'MSI cache namespace collided with ZIP.'
    Assert (@($script:observedUris | Where-Object { $_ -notmatch '^https://github\.com/lmdanavanem/SuniPDF/releases/download/v0\.1\.0/SuniPDF-0\.1\.0-win-x64(?:-portable\.zip|\.msi)(?:\.sha256)?$' }).Count -eq 0) 'A request escaped the pinned release URL shape.'
    $cases.Add('zip-and-msi-namespaces-use-only-the-official-release-path')

    [IO.Directory]::CreateDirectory($outside) | Out-Null
    New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null
    $request = $baselineArgs.Clone(); $request.CacheDirectory = $junction
    Reject { Get-SuniUpgradeBaseline @request } 'A reparse-point cache root was accepted.'
    Assert (@(Get-ChildItem -LiteralPath $outside -Force).Count -eq 0) 'Cache wrote through a reparse point.'
    $cases.Add('restored-reparse-paths-are-rejected')
    @{ status='passed'; cases=@($cases); network_requests=0; application_installations=0 } | ConvertTo-Json -Depth 3
} finally {
    if (Test-Path -LiteralPath $junction) { [IO.Directory]::Delete($junction) }
    $full = [IO.Path]::GetFullPath($workspace)
    $temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if (-not $full.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe cache self-test cleanup path.' }
    Remove-Item -LiteralPath $full -Recurse -Force
}
