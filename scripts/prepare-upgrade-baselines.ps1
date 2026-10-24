# Fill the verified byte cache before compilation and real migration tests begin.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'upgrade-baseline-cache.ps1')
$repo = Split-Path -Parent $PSScriptRoot
$match = Select-String -LiteralPath (Join-Path $repo 'Cargo.toml') -Pattern '^version = "(\d+\.\d+\.\d+)"' | Select-Object -First 1
if (-not $match) { throw 'Workspace release version is missing.' }
$currentVersion = $match.Matches[0].Groups[1].Value
$counts = @{}
foreach ($kind in @('portable', 'msi')) {
    $file = if ($kind -eq 'portable') { 'test-portable-update.ps1' } else { 'test-msi-upgrade.ps1' }
    $variable = if ($kind -eq 'portable') { 'pins' } else { 'baselines' }
    $pins = Read-SuniBaselinePins (Join-Path $PSScriptRoot $file) $variable
    $count = 0
    foreach ($version in @($pins.Keys | Sort-Object { [version]$_ })) {
        if ([version]$version -ge [version]$currentVersion) { continue }
        $hash = if ($kind -eq 'portable') { $pins[$version] } else { $pins[$version].hash }
        $null = Get-SuniUpgradeBaseline -Version $version -CurrentVersion $currentVersion -ExpectedSha256 $hash -Kind $kind
        $count++
    }
    if ($count -eq 0) { throw "No historical $kind baseline was verified." }
    $counts[$kind] = $count
}
@{ status='passed'; scope='Verified historical package bytes only; migration tests run separately'; version=$currentVersion; baselines=$counts } | ConvertTo-Json -Depth 3
