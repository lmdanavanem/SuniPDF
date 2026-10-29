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
        -not $report.fixtureUnchanged -or @($report.scenes).Count -ne 20) { throw 'Reader UI report did not qualify the current source.' }
    $expected = @('reader-en-dark-measurements', 'reader-fr-light-720-zoom149', 'reader-en-light-pages', 'reader-en-dark-docked-register', 'reader-fr-light-720-docked-quantities', 'reader-en-glass-light-pages', 'reader-en-glass-dark-measurements', 'reader-en-glass-dark-docked-register', 'settings-en-glass-light', 'settings-fr-glass-dark-720-reduced', 'reader-en-dark-split-independent', 'reader-fr-light-720-split-independent', 'reader-en-glass-light-split-synchronized', 'home-en-glass-light-tools', 'home-en-glass-dark', 'home-en-glass-light-560')
    $expected += @('register-en-light-bulk-preview', 'register-fr-dark-720-bulk-preview', 'register-en-light-saved-filters', 'register-fr-dark-720-saved-filters')
    $registerFixture = Join-Path $root 'target/reader-ui-evidence/register-dialog-fixture.pdf'
    if ($report.registerFixture.pdf -cne 'register-dialog-fixture.pdf' -or
        $report.registerFixture.annotationCount -ne 2 -or -not $report.registerFixture.sourceUnchanged -or
        $report.registerFixture.sha256 -cnotmatch '^[a-f0-9]{64}$' -or
        (Get-FileHash -LiteralPath $registerFixture -Algorithm SHA256).Hash.ToLowerInvariant() -cne $report.registerFixture.sha256) {
        throw 'Register dialog source PDF is missing or differs from the actual computed fixture.'
    }
    foreach ($scene in $report.scenes) {
        if ($scene.name -cnotin $expected -or $scene.png -cne "$($scene.name).png" -or
            -not $scene.actualApplicationRenderUi -or $scene.unsavedChanges -cne $false -or $null -ne $scene.appError -or
            $scene.pngSha256 -cnotmatch '^[a-f0-9]{64}$') {
            throw 'Reader UI scene metadata is invalid.'
        }
        $expectedSurface = if ($scene.name -like 'home-*') { 'home' } elseif ($scene.name -like 'settings-*') { 'settings' } elseif ($scene.name -like 'register-*') { 'register-dialog' } else { 'reader' }
        if ($scene.surface -cne $expectedSurface) { throw 'UI scene surface differs from its scenario.' }
        if ($scene.surface -ceq 'home') {
            if ($scene.documentOpen -cne $false -or $scene.actualPdfiumTiles -ne 0 -or
                @($scene.visiblePdfRegions).Count -ne 0 -or $scene.pdfFixtureBlueInkVerified -or
                $scene.pdfFixtureBlueInkPixels -ne 0 -or $scene.readerLabelsVerified -or
                $scene.themeSettingsOpen -or $scene.themeSettingsLabelsVerified -or
                -not $scene.homeLabelsVerified -or $scene.measurementToolsOpen -or
                $null -ne $scene.dockedTable -or $scene.dockedTableControlsAndRowVerified -or
                [bool]$scene.toolsSidebarOpen -ne ($scene.name -ceq 'home-en-glass-light-tools') -or
                [bool]$scene.darkTheme -ne ($scene.name -ceq 'home-en-glass-dark')) {
                throw 'Home evidence retained document claims, unexpected panels or the wrong appearance.'
            }
            $homeEvidence = $scene.homeLabels
            $labels = @($homeEvidence.verified)
            if ($labels.Count -lt 4 -or @($labels.key | Select-Object -Unique).Count -ne $labels.Count -or
                -not $homeEvidence.compactOpenRowVerified -or $homeEvidence.openRowAccentFraction -lt 0 -or
                $homeEvidence.openRowAccentFraction -ge 0.15 -or @($homeEvidence.openRowBounds).Count -ne 4 -or
                ($homeEvidence.openRowBounds[3] - $homeEvidence.openRowBounds[1]) -gt 80) {
                throw 'Home evidence lacks its compact neutral Open file row or distinct readable labels.'
            }
            foreach ($required in @('home.welcome', 'home.open_file', 'home.recent', 'tools.group.review')) {
                if ($required -cnotin $labels.key) { throw 'Home evidence is missing a required visible heading or action.' }
            }
            foreach ($label in $labels) {
                if (-not $label.accessible -or $label.readablePixels -lt 20 -or $label.minimumContrast -ne 4.5 -or
                    @($label.bounds).Count -ne 4 -or $label.bounds[0] -lt 0 -or $label.bounds[1] -lt 0 -or
                    $label.bounds[2] -le $label.bounds[0] -or $label.bounds[3] -le $label.bounds[1] -or
                    $label.bounds[2] -gt $scene.outputPixels[0] -or $label.bounds[3] -gt $scene.outputPixels[1]) {
                    throw 'Home text is clipped, inaccessible or lacks actual readable pixels.'
                }
            }
        } elseif (-not $scene.documentOpen -or $scene.actualPdfiumTiles -lt 1 -or $scene.homeLabelsVerified) {
            throw 'Reader/settings evidence must retain actual PDF tiles and cannot claim Home coverage.'
        }
        if ($scene.surface -ceq 'settings') {
            if (-not $scene.themeSettingsOpen -or -not $scene.themeSettingsLabelsVerified -or $scene.pdfFixtureBlueInkVerified) {
                throw 'Settings evidence is missing its visible accessible controls or overstates obscured PDF coverage.'
            }
        } elseif ($scene.surface -ceq 'reader' -and ($scene.themeSettingsOpen -or -not $scene.pdfFixtureBlueInkVerified -or $scene.pdfFixtureBlueInkPixels -le 20)) {
            throw 'Reader evidence must retain visible actual PDF ink.'
        }
        if ($scene.surface -ceq 'register-dialog') {
            $proof = $scene.registerDialog
            $bulk = $scene.name -like '*bulk-preview'
            $french = $scene.name -like 'register-fr-*'
            $kind = if ($bulk) { 'bulk-preview' } else { 'saved-filters' }
            $width = if ($french) { 720 } else { 1280 }
            $height = if ($french) { 640 } else { 780 }
            if ($proof.kind -cne $kind -or $scene.outputPixels[0] -ne $width -or $scene.outputPixels[1] -ne $height -or
                [bool]$scene.darkTheme -ne $french -or $scene.language -cne $(if ($french) { 'fr-FR' } else { 'en-US' }) -or
                $scene.themeSettingsOpen -or $scene.themeSettingsLabelsVerified -or $scene.pdfFixtureBlueInkVerified -or
                $scene.measurementToolsOpen -or $null -ne $scene.dockedTable -or $scene.dockedTableControlsAndRowVerified -or
                -not $proof.actualRegisterComputed -or -not $proof.documentRegisterUnchanged -or -not $proof.sourcePdfUnchanged -or
                $proof.sourcePdf -cne 'register-dialog-fixture.pdf' -or
                $proof.sourcePdfSha256Before -cne $report.registerFixture.sha256 -or
                $proof.sourcePdfSha256After -cne $report.registerFixture.sha256 -or
                $proof.registerSourceSha256 -cnotmatch '^[a-f0-9]{64}$') {
                throw 'Register dialog does not match its real source, language, size or modal scene.'
            }
            $labels = @($proof.labels)
            $required = if ($bulk) {
                @('register.bulk.title','register.bulk.summary','register.bulk.before','register.bulk.after','register.bulk.reviewed','before.business_status','after.business_status','register.bulk.preview','register.bulk.back')
            } else {
                @('register.presets.title','register.presets.scope','register.presets.name','register.presets.saved','register.presets.import_help','savedPresetName','register.presets.save','register.presets.load','register.presets.delete','register.presets.import','register.presets.export','register.close')
            }
            if ($labels.Count -ne $required.Count -or @($labels.key | Select-Object -Unique).Count -ne $required.Count) { throw 'Register dialog label inventory differs.' }
            foreach ($key in $required) { if ($key -cnotin $labels.key) { throw 'Register dialog required label is missing.' } }
            foreach ($label in $labels) {
                if (-not $label.accessible -or $label.disabled -or $label.readablePixels -lt 20 -or $label.minimumContrast -ne 4.5 -or
                    @($label.bounds).Count -ne 4 -or @($label.clip).Count -ne 4 -or
                    $label.bounds[0] -lt 0 -or $label.bounds[1] -lt 0 -or
                    $label.bounds[2] -le $label.bounds[0] -or $label.bounds[3] -le $label.bounds[1] -or
                    $label.bounds[2] -gt $width -or $label.bounds[3] -gt $height -or
                    $label.clip[0] -gt $label.bounds[0] -or $label.clip[1] -gt $label.bounds[1] -or
                    $label.clip[2] -lt $label.bounds[2] -or $label.clip[3] -lt $label.bounds[3]) {
                    throw 'Register dialog required label is clipped, disabled or unreadable.'
                }
            }
            if ($bulk) {
                $apply = $proof.applyControl
                if (-not $proof.actualPreviewComputed -or $proof.reviewed -or -not $proof.applyDisabled -or
                    $proof.selectionCount -ne 2 -or $proof.report.source_sha256 -cne $proof.registerSourceSha256 -or
                    $proof.report.selected_count -ne 2 -or $proof.report.changed_count -ne 2 -or @($proof.report.changes).Count -ne 2 -or
                    $apply.key -cne 'register.bulk.apply' -or -not $apply.accessible -or -not $apply.button -or -not $apply.disabled -or
                    $apply.readablePixels -lt 20 -or $apply.minimumContrast -ne 1.5 -or
                    @($apply.bounds).Count -ne 4 -or @($apply.clip).Count -ne 4 -or
                    $apply.bounds[0] -lt 0 -or $apply.bounds[1] -lt 0 -or
                    $apply.bounds[2] -le $apply.bounds[0] -or $apply.bounds[3] -le $apply.bounds[1] -or
                    $apply.bounds[2] -gt $width -or $apply.bounds[3] -gt $height -or
                    $apply.clip[0] -gt $apply.bounds[0] -or $apply.clip[1] -gt $apply.bounds[1] -or
                    $apply.clip[2] -lt $apply.bounds[2] -or $apply.clip[3] -lt $apply.bounds[3] -or
                    @($proof.report.changes.index | Select-Object -Unique).Count -ne 2) {
                    throw 'Bulk preview is absent, invented, already reviewed or permits Apply before review.'
                }
                foreach ($change in $proof.report.changes) {
                    if ($change.page -ne 1 -or $change.index -notin @(0,1) -or
                        $change.expected_row_sha256 -cnotmatch '^[a-f0-9]{64}$' -or
                        $change.before.business_status -cne 'Open' -or $change.after.business_status -cne 'Ready for site' -or
                        $change.before.discipline -cne 'Electrical' -or $change.after.discipline -cne 'Electrical') {
                        throw 'Bulk preview before/after differs from its actual fixture and patch.'
                    }
                }
            } elseif ($proof.selectionCount -ne 0 -or $proof.library.format -cne 'sunipdf-register-filter-presets' -or
                $proof.library.version -ne 1 -or @($proof.library.presets).Count -ne 1 -or
                $proof.library.presets[0].criteria.business_status -cne 'Open' -or
                $proof.library.presets[0].criteria.discipline -cne 'Electrical') {
                throw 'Saved filters lost their actual captured criteria or selected annotations automatically.'
            }
        } elseif ($null -ne $scene.registerDialog) {
            throw 'Other scenes unexpectedly claim register dialog evidence.'
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
    if (@($report.scenes.name | Select-Object -Unique).Count -ne 20) { throw 'Reader UI scene names are not unique.' }

    $aboutTest = 'reader_ui_evidence::about_ui_dx12_warp_evidence'
    if (@($listing | Where-Object { $_ -ceq "${aboutTest}: test" }).Count -ne 1) {
        throw 'The exact About UI test is absent from the qualified executable.'
    }
    $aboutStarted = [DateTime]::UtcNow
    & $executable --ignored --exact $aboutTest --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The actual About UI evidence gate failed.' }
    $aboutRoot = Join-Path $root 'target/about-visibility-qa'
    $aboutReportPath = Join-Path $aboutRoot 'about-ui-report.json'
    $aboutFile = Get-Item -LiteralPath $aboutReportPath
    if ($aboutFile.Length -gt 32KB -or $aboutFile.LastWriteTimeUtc -lt $aboutStarted) {
        throw 'About UI report is oversized or stale.'
    }
    $about = [IO.File]::ReadAllText($aboutReportPath) | ConvertFrom-Json
    $aboutExpected = @('about-fr-light', 'about-en-dark', 'about-fr-light-small')
    if ($about.status -cne 'passed' -or @($about.scenes).Count -ne 3 -or
        @($about.scenes.name | Select-Object -Unique).Count -ne 3 -or
        $about.updateStatus -cne 'Default unchecked state; no live network request') {
        throw 'About UI report did not qualify its three expected scenes.'
    }
    $aboutImages = @()
    foreach ($scene in $about.scenes) {
        $small = $scene.name -ceq 'about-fr-light-small'
        $english = $scene.name -ceq 'about-en-dark'
        $width = if ($small) { 720 } else { 1280 }
        $height = if ($small) { 480 } else { 1000 }
        $required = @('about.title', 'about.close', 'about.tagline')
        if (-not $small) {
            $required += @('about.privacy_title', 'about.updates_recovery', 'updates.startup',
                'about.recovery_local', 'about.licenses', 'about.credit', 'about.author', 'about.website')
        }
        if ($scene.name -cnotin $aboutExpected -or $scene.png -cne "$($scene.name).png" -or
            $scene.language -cne $(if ($english) { 'en-US' } else { 'fr-FR' }) -or
            [bool]$scene.dark -ne $english -or @($scene.pixels).Count -ne 2 -or
            $scene.pixels[0] -ne $width -or $scene.pixels[1] -ne $height -or
            @($scene.visibleLabels).Count -ne $required.Count -or
            @($scene.visibleLabels | Select-Object -Unique).Count -ne $required.Count) {
            throw 'About UI scene metadata differs from its required scenario.'
        }
        foreach ($key in $required) {
            if ($key -cnotin $scene.visibleLabels) { throw 'About UI scene is missing a required visible label.' }
        }
        $png = Get-Item -LiteralPath (Join-Path $aboutRoot $scene.png)
        if ($png.Length -lt 1000 -or $png.Length -gt 8MB -or $png.LastWriteTimeUtc -lt $aboutStarted) {
            throw 'About UI image is missing, stale or outside its expected size bound.'
        }
        $aboutImages += @{ name=$scene.png; sha256=(Get-FileHash -LiteralPath $png.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    }
    @{ sourceSha=$env:GITHUB_SHA; test=$aboutTest;
       executableSha256=(Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant();
       reportSha256=(Get-FileHash -LiteralPath $aboutReportPath -Algorithm SHA256).Hash.ToLowerInvariant();
       images=$aboutImages; status='passed' } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $aboutRoot 'about-ci-provenance.json') -Encoding utf8
} finally { Pop-Location }
