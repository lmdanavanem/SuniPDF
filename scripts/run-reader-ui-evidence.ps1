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
        -not $report.fixtureUnchanged -or @($report.scenes).Count -ne 13) { throw 'Reader UI report did not qualify the current source.' }
    $expected = @('reader-en-dark-measurements', 'reader-fr-light-720-zoom149', 'reader-en-light-pages', 'reader-en-dark-docked-register', 'reader-fr-light-720-docked-quantities', 'reader-en-glass-light-pages', 'reader-en-glass-dark-measurements', 'reader-en-glass-dark-docked-register', 'settings-en-glass-light', 'settings-fr-glass-dark-720-reduced', 'reader-en-dark-split-independent', 'reader-fr-light-720-split-independent', 'reader-en-glass-light-split-synchronized')
    foreach ($scene in $report.scenes) {
        if ($scene.name -cnotin $expected -or $scene.png -cne "$($scene.name).png" -or
            -not $scene.actualApplicationRenderUi -or $scene.actualPdfiumTiles -lt 1 -or
            $scene.pngSha256 -cnotmatch '^[a-f0-9]{64}$') {
            throw 'Reader UI scene metadata is invalid.'
        }
        if ($scene.name -like 'settings-*') {
            if (-not $scene.themeSettingsOpen -or -not $scene.themeSettingsLabelsVerified -or $scene.pdfFixtureBlueInkVerified) {
                throw 'Settings evidence is missing its visible accessible controls or overstates obscured PDF coverage.'
            }
        } elseif ($scene.themeSettingsOpen -or -not $scene.pdfFixtureBlueInkVerified -or $scene.pdfFixtureBlueInkPixels -le 20) {
            throw 'Reader evidence must retain visible actual PDF ink.'
        }
        $expectedStyle = if ($scene.name -like '*glass*') { 'liquid_glass' } else { 'classic' }
        if ($scene.interfaceStyle -cne $expectedStyle -or [bool]$scene.reduceTransparency -ne ($scene.name -like '*reduced')) {
            throw 'Reader evidence appearance does not match its named scenario.'
        }
        $png = Join-Path $root "target/reader-ui-evidence/$($scene.png)"
        if ((Get-FileHash -LiteralPath $png -Algorithm SHA256).Hash.ToLowerInvariant() -cne $scene.pngSha256) {
            throw 'Reader UI screenshot digest differs from its report.'
        }
        if ($scene.name -like '*docked*' -and (-not $scene.dockedTableControlsAndRowVerified -or $scene.dockedTable -cnotin @('annotations','quantities'))) {
            throw 'Docked table evidence is incomplete.'
        }
        if ($scene.name -like '*split-*') {
            $split = $scene.splitReader
            $synchronized = $scene.name -ceq 'reader-en-glass-light-split-synchronized'
            if (-not $split.enabled -or [bool]$split.synchronized -ne $synchronized -or
                -not $split.independentBeforeSynchronization -or -not $split.combinedRenderQueueVerified -or
                -not $split.sharedDocumentEpochVerified -or $null -eq $split.settleState.documentEpoch -or
                -not $split.bothPanesPaintExpectedPdf -or -not $split.paneZoomsVerified -or
                -not $split.controlsVisibleAndAccessibleVerified -or
                @($split.panes).Count -ne 2 -or @($split.settleState.panes).Count -ne 2 -or
                $split.activePane -ne 0) {
                throw 'Split reader evidence is incomplete or does not match its synchronization scenario.'
            }
            for ($index = 0; $index -lt 2; $index++) {
                $pane = $split.panes[$index]
                $settled = $split.settleState.panes[$index]
                $expectedPage = if ($synchronized) { 0 } else { $index }
                $expectedInk = if ($expectedPage -eq 0) { 'blue' } else { 'red' }
                if ($pane.index -ne $index -or $pane.pageIndex -ne $expectedPage -or
                    $settled.pageIndex -ne $expectedPage -or $settled.rendering -or
                    $pane.fixtureInk -cne $expectedInk -or $pane.fixtureInkPixels -le 20 -or
                    @($pane.visiblePdfRegions).Count -lt 1 -or
                    [Math]::Abs($pane.effectiveZoom - $settled.effectiveZoom) -ge 0.001) {
                    throw 'Split reader pane did not retain its expected page, zoom and actual PDF fixture pixels.'
                }
            }
        } elseif ($null -ne $scene.splitReader) {
            throw 'Non-split scene unexpectedly reports split reader qualification.'
        }
    }
    if (@($report.scenes.name | Select-Object -Unique).Count -ne 13) { throw 'Reader UI scene names are not unique.' }
} finally { Pop-Location }
