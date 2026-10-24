# Reuse the exact desktop unit-test executable qualified by the workspace gate.
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
function Get-DesktopTestExecutable([string]$Text) {
    $plain = [regex]::Replace($Text, '\x1B\[[0-?]*[ -/]*[@-~]', '')
    $pattern = '(?m)^\s*Running\s+unittests\s+src[\\/]main\.rs\s+\((?<exe>target[\\/]debug[\\/]deps[\\/]sunipdf-[a-f0-9]{16}\.exe)\)\s*$'
    $found = [regex]::Matches($plain, $pattern)
    if ($found.Count -ne 1) { throw 'Workspace log must identify exactly one desktop unit-test executable.' }
    return [IO.Path]::GetFullPath((Join-Path $root $found[0].Groups['exe'].Value))
}
if ($SelfTest) {
    $valid = 'Running unittests src\main.rs (target\debug\deps\sunipdf-0123456789abcdef.exe)'
    $expected = Join-Path $root 'target\debug\deps\sunipdf-0123456789abcdef.exe'
    if ((Get-DesktopTestExecutable $valid) -cne $expected) { throw 'Desktop executable selection failed.' }
    if ((Get-DesktopTestExecutable $valid.Replace('\', '/')) -cne $expected) { throw 'Forward-slash executable selection failed.' }
    foreach ($invalid in @('', "$valid`n$valid", $valid.Replace('target\debug\deps', '..\outside'), $valid.Replace('sunipdf-', 'sunipdf_cli-'), $valid.Replace('src\main.rs', 'tests\main.rs'))) {
        $refused = $false
        try { Get-DesktopTestExecutable $invalid | Out-Null } catch { $refused = $true }
        if (-not $refused) { throw 'Unsafe or ambiguous desktop executable was accepted.' }
    }
    Write-Host 'Reader UI test selection and path bounds passed. No executable was run.'
    exit 0
}
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:GITHUB_REPOSITORY -ne 'lmdanavanem/SuniPDF' -or
    $env:RUNNER_OS -ne 'Windows' -or $env:RUNNER_ARCH -ne 'X64') {
    throw 'Reader UI evidence runs only in this repository Windows x64 CI.'
}
$log = Join-Path $root 'target/workspace-tests.log'
if (-not (Test-Path -LiteralPath $log -PathType Leaf) -or (Get-Item -LiteralPath $log).Length -gt 8MB) {
    throw 'Workspace test log is missing or exceeds its size bound.'
}
$executable = Get-DesktopTestExecutable ([IO.File]::ReadAllText($log))
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw 'The workspace-built desktop test executable is missing.' }
Push-Location (Join-Path $root 'apps/sunipdf-desktop')
try {
    $test = 'reader_ui_evidence::reader_ui_dx12_warp_evidence'
    $listing = @(& $executable --list --ignored)
    if ($LASTEXITCODE -ne 0 -or @($listing | Where-Object { $_ -ceq "${test}: test" }).Count -ne 1) {
        throw 'The exact reader UI test is absent from the qualified executable.'
    }
    $started = [DateTime]::UtcNow
    & $executable --ignored --exact reader_ui_evidence::reader_ui_dx12_warp_evidence --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The actual reader UI evidence gate failed.' }
    $reportPath = Join-Path $root 'target/reader-ui-evidence/reader-ui-report.json'
    $reportFile = Get-Item -LiteralPath $reportPath
    if ($reportFile.Length -gt 128KB -or $reportFile.LastWriteTimeUtc -lt $started) { throw 'Reader UI report is oversized or stale.' }
    $report = [IO.File]::ReadAllText($reportPath) | ConvertFrom-Json
    if ($report.status -cne 'passed' -or $report.sourceSha -cne $env:GITHUB_SHA -or
        -not $report.fixtureUnchanged -or @($report.scenes).Count -ne 5) { throw 'Reader UI report did not qualify the current source.' }
    $expected = @('reader-en-dark-measurements', 'reader-fr-light-720-zoom149', 'reader-en-light-pages', 'reader-en-dark-docked-register', 'reader-fr-light-720-docked-quantities')
    foreach ($scene in $report.scenes) {
        if ($scene.name -cnotin $expected -or $scene.png -cne "$($scene.name).png" -or
            -not $scene.actualApplicationRenderUi -or $scene.actualPdfiumTiles -lt 1 -or
            $scene.pdfFixtureBlueInkPixels -le 20 -or $scene.pngSha256 -cnotmatch '^[a-f0-9]{64}$') {
            throw 'Reader UI scene metadata is invalid.'
        }
        $png = Join-Path $root "target/reader-ui-evidence/$($scene.png)"
        if ((Get-FileHash -LiteralPath $png -Algorithm SHA256).Hash.ToLowerInvariant() -cne $scene.pngSha256) {
            throw 'Reader UI screenshot digest differs from its report.'
        }
        if ($scene.name -like '*docked*' -and (-not $scene.dockedTableControlsAndRowVerified -or $scene.dockedTable -cnotin @('annotations','quantities'))) {
            throw 'Docked table evidence is incomplete.'
        }
    }
    if (@($report.scenes.name | Select-Object -Unique).Count -ne 5) { throw 'Reader UI scene names are not unique.' }
} finally { Pop-Location }
