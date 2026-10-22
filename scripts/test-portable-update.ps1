# Real portable updates in temporary copies, never the installed app or the user's profile.
param(
    [Parameter(Mandatory = $true)] [string]$Package,
    [string[]]$FromPackage = @(),
    [string]$ReportPath = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$repo = Split-Path -Parent $PSScriptRoot
$helperSource = Join-Path $repo 'apps/sunipdf-desktop/src/update-helper.ps1'
$powershell = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
& (Join-Path $PSScriptRoot 'test-update-helper-host.ps1')

function Assert([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function ChildPath([string]$Path, [string]$Root) {
    $full = [IO.Path]::GetFullPath($Path)
    $prefix = [IO.Path]::GetFullPath($Root).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    Assert ($full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) 'Test path escaped its temporary workspace.'
    return $full
}
function PackageInfo([string]$Path) {
    $full = (Get-Item -LiteralPath $Path).FullName
    Assert ((Split-Path -Leaf $full) -match '^SuniPDF-(\d+\.\d+\.\d+)-win-x64-portable\.zip$') 'An x64 portable release ZIP is required.'
    $version = $Matches[1]
    $hash = (Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash
    $sidecar = [IO.File]::ReadAllText("$full.sha256").Trim() -split '\s+', 2
    Assert ($hash -ieq $sidecar[0]) 'Release ZIP SHA256 sidecar mismatch.'
    return @{ path = $full; version = $version; hash = $hash }
}
function PreviousPackages([string]$Workspace) {
    $pins = @{
        '0.8.2' = 'B3A311CBB62803F9637E664C317999C1D0162358D7152E8C37223B3BF3C9EA4C'
        '0.8.3' = '46AB7FC675DC1A43A955D629718924B717C7B935E0C5ECDBDDFC9E46578BA018'
        '0.9.0' = 'E935A02D5DC98AB2AFC67C3CAC856AB10ACA6D4A090212CE084AA87A9398A9A2'
        '0.9.1' = '601306861BE0C80BDA62E568473172504B530D4344635D3BFA28CBF6ADBFC082'
        '0.10.0' = 'ADD8EF3F1F385B47AA4E3FB9061F8735C7E31004D408ABD853924A8BD9A35742'
        '0.11.0' = '6A08A518619D89D450332F8AE1BA31FB336C3DF88910D5471183B25C173B2421'
        '0.12.0' = 'AD1ECB72A999F1A43602E58AD2634F9E80FD7925C90A90B9AD25FB9B6BDC1B83'
        '0.13.0' = 'B5CC0CBAB1A1ABE4CFF346093A984760ECE458C2986CBF650C6EC3769C2A3FDD'
        '0.14.0' = '9A2A085FDCC0E30852A2F72755705CB42EE2F5C866EF9A4E00ADCEEA9C6D6960'
        '0.15.0' = 'CFC093316717596EE4BD3944DB6108A5512AC191F2D37782F55B4A389CE7E733'
        '0.16.0' = 'F03B7BB283A58414F1F26FD707FE120231B1B9502762B5588F75C97DC4526DD0'
        '0.17.0' = '55D8CBC224D1D6D9582AD9402F51B7F76CEC84C93A0FC90AFE91C43471ACF1D4'
        '0.18.0' = '8D1C76930C7D929D289745DC20897C91418DF63873B6E66B2566CA6AF289A08C'
        '0.19.0' = 'C713DC4495D024E0BCBC715A29832FD5C49B164EFE165907496E8A1F9690E3BB'
        '0.20.0' = 'CBAB56A4FD8AAF15F798B94EDDF88C62785D78C11ED7E26CDC4D81266AD77EAD'
        '0.21.0' = '7D5DC36F467970891E5BD094791A8E0B0D8AFFDD7BFC36B106E9EE0409B2A956'
        '0.21.1' = 'F00AD8124906B4259C0F4EF96CDECE8A1C28788609384DD151C340AFBDF80BBE'
        '0.30.0' = '99C37B81191CB97A3F7821594DCAED0E11E1FF1180F0BBAFC5FAE41B1B53620F'
        '0.30.1' = '6B743641F41DC57416C3044FCFF48839072A9C1C178A686A7E1B8A7E86761DAF'
        '0.31.0' = 'DB62F18AD6AA96FB8D59E04B87049C08A2F630616375FFEF2104091134D35BB9'
        '0.32.0' = '08A65D843329D67F98372E0DD7A448AF364DCD19AF184748618D70D2D61C714D'
        '0.33.0' = '64FEED1F90F746BB9A15C877FB74F69FFD5BF22C83746D729401945EC39C2B61'
        '0.34.0' = '2BB5985D9916621A353ED988567246B7CAEA67BDEC34B66634CB16E33E7EAC94'
        '0.35.0' = '359BBBA8A45D9B96D56DD79845F3EB5E4EF0CA97522330783CB4B594FC11B255'
        '0.36.0' = 'F94DE5EA66D660941B8582B19BF85B5BE45CBE54758E307A257CBA6A62ED8BB5'
        '0.37.0' = '2FCD3B76114CBFC6E6ED2A96309737A39A0DBCDAD3A8AD069B7E99EA2DD67E30'
        '0.38.0' = 'C52C239AFCDDB0CCA468877DD15BFCCAF1AA317E08AC7A35D216DC27595393BA'
        '0.39.0' = '745E426B07EE9CE359EC3CA2BCEE159E335F0F46741729BF1A832BAF8F169B1C'
        '0.40.0' = 'CD8AFB4F2F5982884F9D2A1B57E266A9D64643527EA3CDB93E6F52C2D7C5C636'
        '0.41.0' = '711EFCA4257AE46C46755A5D2ABFBDE1FCAFF0AFC81FC905750CC3EC65714295'
        '0.42.0' = '642FA3C8B663C9E5A649988E72CDA1DE6BD5F1219D33F6CB32F9ADC5E720E65C'
        '0.43.0' = 'D236A8DDA567A82A39481A056B4E551225C06FDC20B0E8F28620EE6FF2A6DC21'
        '0.44.0' = 'E7C90E9A7168358D4E8FC91ACAA802F370A9A8086E493CB55808E6647D45D250'
        '0.45.0' = 'C362042B6ADA777E50632A9FFEB8F0965C07141AE31FF08E57EC7CD4ECB22D54'
        '0.46.0' = '1BC8A9880905E6B18421F9083928220EC74B9E419CDBD106F8C0699347AD6C79'
        '0.47.0' = 'C29BB35759B98715824B4EC72B2503C8027E58CCDDB156CEC0A4BC9E63B13262'
        '0.48.0' = '84B8FDD49E11A3CBDA6B0D19B2755D2B3EC487B485378005792048849B0AF13F'
    }
    foreach ($version in @('0.8.2', '0.8.3', '0.9.0', '0.9.1', '0.10.0', '0.11.0', '0.12.0', '0.13.0', '0.14.0', '0.15.0', '0.16.0', '0.17.0', '0.18.0', '0.19.0', '0.20.0', '0.21.0', '0.21.1', '0.30.0', '0.30.1', '0.31.0', '0.32.0', '0.33.0', '0.34.0', '0.35.0', '0.36.0', '0.37.0', '0.38.0', '0.39.0', '0.40.0', '0.41.0', '0.42.0', '0.43.0', '0.44.0', '0.45.0', '0.46.0', '0.47.0', '0.48.0')) {
        if ([version]$version -ge [version]$incoming.version) { continue }
        $name = "SuniPDF-$version-win-x64-portable.zip"
        $sidecarBody = (Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/lmdanavanem/SuniPDF/releases/download/v$version/$name.sha256" -TimeoutSec 60).Content
        if ($sidecarBody -is [byte[]]) { $sidecarBody = [Text.Encoding]::ASCII.GetString($sidecarBody) }
        $sidecar = $sidecarBody.Trim() -split '\s+', 2
        Assert ($sidecar[0] -ieq $pins[$version] -and $sidecar[1] -eq $name) "Pinned release digest disagrees with the published checksum: $version"
        $path = Join-Path $repo "dist/$name"
        if (-not (Test-Path -LiteralPath $path) -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $pins[$version]) {
            $directory = Join-Path $Workspace 'previous'
            [IO.Directory]::CreateDirectory($directory) | Out-Null
            $path = Join-Path $directory $name
            Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/lmdanavanem/SuniPDF/releases/download/v$version/$name" -OutFile $path -TimeoutSec 300
            Assert ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ieq $pins[$version]) "Pinned previous public release checksum mismatch: $version"
            [IO.File]::WriteAllText("$path.sha256", "$($pins[$version])  $name", [Text.Encoding]::ASCII)
        }
        Write-Output $path
    }
}
function ExpandTestCopy([string]$Archive, [string]$Destination) {
    [IO.Directory]::CreateDirectory($Destination) | Out-Null
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        foreach ($entry in $zip.Entries) {
            $path = ChildPath (Join-Path $Destination $entry.FullName) $Destination
            if ($entry.FullName.EndsWith('/')) { [IO.Directory]::CreateDirectory($path) | Out-Null; continue }
            [IO.Directory]::CreateDirectory((Split-Path -Parent $path)) | Out-Null
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $path, $false)
        }
    } finally { $zip.Dispose() }
}
function FileMap([string]$Directory) {
    $files = @{}
    foreach ($file in Get-ChildItem -LiteralPath $Directory -Recurse -File) {
        $name = $file.FullName.Substring($Directory.TrimEnd('\').Length + 1)
        $files[$name] = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    }
    return $files
}
function AssertMap($Expected, [string]$Directory) {
    $actual = FileMap $Directory
    Assert ($actual.Count -eq $Expected.Count) 'Update failure changed the target file set.'
    foreach ($name in $Expected.Keys) { Assert ($actual[$name] -eq $Expected[$name]) "Update failure changed $name." }
}
function StopOwned($Process, [string]$Executable) {
    if ($null -eq $Process) { return }
    try {
        $Process.Refresh()
        if (-not $Process.HasExited) {
            Assert ($Process.Path -ieq $Executable) 'Refusing to stop a process outside the test application.'
            Stop-Process -Id $Process.Id -Force
            $Process.WaitForExit()
        }
    } catch {
        if ($_.Exception -is [InvalidOperationException] -and $Process.HasExited) { return }
        throw
    }
}
function PrepareTestOpenGl([string]$Target) {
    if (-not $env:SUNIPDF_TEST_OPENGL_DIR) { return @() }
    $hashes = @{
        'opengl32.dll' = '75B295FC80DF0EAA8F1264C28663F6932C8B72D36513EB092556608858B7F5E0'
        'libgallium_wgl.dll' = '53E49ADF87590D5B0CAB8553B1E98084812040CC1FCA8C487695B17B7AF3702E'
    }
    foreach ($name in $hashes.Keys) {
        $source = Join-Path $env:SUNIPDF_TEST_OPENGL_DIR $name
        $destination = ChildPath (Join-Path $Target $name) $Target
        Assert (-not (Test-Path -LiteralPath $destination)) 'Test OpenGL would replace a packaged application file.'
        Assert ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -eq $hashes[$name]) 'Test OpenGL source checksum mismatch.'
        $reader = [IO.BinaryReader]::new([IO.File]::OpenRead($source))
        try {
            Assert ($reader.BaseStream.Length -ge 64 -and $reader.ReadUInt16() -eq 0x5A4D) 'Invalid test OpenGL binary.'
            $reader.BaseStream.Position = 60
            $header = $reader.ReadInt32()
            Assert ($header -ge 64 -and $header -le $reader.BaseStream.Length - 6) 'Invalid test OpenGL PE header.'
            $reader.BaseStream.Position = $header
            Assert ($reader.ReadUInt32() -eq 0x00004550 -and $reader.ReadUInt16() -eq 0x8664) 'Test OpenGL source is not x64.'
        } finally { $reader.Dispose() }
        Copy-Item -LiteralPath $source -Destination $destination
        Assert ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -eq $hashes[$name]) 'Test OpenGL application copy checksum mismatch.'
    }
    # WGL creation and statically imported OpenGL calls must use the same per-application DLL.
    $env:GLUTIN_WGL_OPENGL_DLL = Join-Path $Target 'opengl32.dll'
    return @($hashes.Keys)
}
function StartApp([string]$Target) {
    $exe = Join-Path $Target 'sunipdf.exe'
    $logs = Split-Path -Parent $Target
    $stdout = Join-Path $logs 'application.stdout.log'
    $stderr = Join-Path $logs 'application.stderr.log'
    $process = Start-Process -FilePath $exe -WorkingDirectory $Target -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    $process.Handle | Out-Null
    Start-Sleep -Milliseconds 1000
    $process.Refresh()
    if ($process.HasExited) {
        $process.WaitForExit()
        $exitCode = $process.ExitCode
        $output = [IO.File]::ReadAllText($stdout)
        $errorOutput = [IO.File]::ReadAllText($stderr)
        $process.Dispose()
        throw "The isolated released application did not stay running: $exe; exit=$exitCode (0x$('{0:X8}' -f $exitCode)); stdout=$output; stderr=$errorOutput"
    }
    return $process
}
function PrepareHelper([string]$Case, $TargetPackage) {
    $work = Join-Path $Case 'helper'
    [IO.Directory]::CreateDirectory($work) | Out-Null
    Copy-Item -LiteralPath $helperSource -Destination (Join-Path $work 'update-helper.ps1')
    Copy-Item -LiteralPath $TargetPackage.path -Destination (Join-Path $work (Split-Path -Leaf $TargetPackage.path))
    return $work
}
function StartHelper([string]$Work, [string]$Target, [int]$OwnerPid, [string]$Hash, [string]$Architecture = 'x64', [bool]$Restart = $false) {
    $zip = Join-Path $Work (Split-Path -Leaf $incoming.path)
    $arguments = '-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "' + (Join-Path $Work 'update-helper.ps1') +
        '" -Mode Portable -NonInteractive -ParentPid ' + $OwnerPid + ' -PackagePath "' + $zip + '" -ExpectedSha256 ' + $Hash +
        ' -TargetDirectory "' + $Target + '" -Arch ' + $Architecture
    if ($Restart) { $arguments += ' -Restart' }
    $process = Start-Process -FilePath $powershell -ArgumentList $arguments -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $Work 'stdout.log') -RedirectStandardError (Join-Path $Work 'stderr.log')
    $process.Handle | Out-Null
    return $process
}
function WaitHelper($Process, [string]$Work) {
    $deadline = [DateTime]::UtcNow.AddSeconds(180)
    while (-not $Process.WaitForExit(1000)) {
        if ([DateTime]::UtcNow -gt $deadline) { throw 'Update helper did not finish within three minutes.' }
    }
    $Process.Refresh()
    $result = Get-Content -LiteralPath (Join-Path $Work 'result.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    Assert (($Process.ExitCode -eq 0) -eq [bool]$result.success) "Helper exit code $($Process.ExitCode) disagrees with result.json."
    return $result
}
function WaitUpdateLock([string]$Target, $Process) {
    $digest = [Security.Cryptography.SHA256]::Create()
    try { $targetId = [BitConverter]::ToString($digest.ComputeHash([Text.Encoding]::UTF8.GetBytes($Target.ToLowerInvariant()))).Replace('-', '') }
    finally { $digest.Dispose() }
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $probe = $null
        try {
            $probe = [Threading.Mutex]::OpenExisting("Local\SuniPDF-update-$targetId")
            if (-not $probe.WaitOne(0)) { return }
            $probe.ReleaseMutex()
        } catch [Threading.WaitHandleCannotBeOpenedException] {
            # The helper has not reached its update lock yet.
        } finally { if ($null -ne $probe) { $probe.Dispose() } }
        Assert (-not $Process.HasExited) 'The helper exited before acquiring its update lock.'
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'The helper did not acquire its update lock.'
}

$incoming = PackageInfo $Package
if (-not $ReportPath) { $ReportPath = Join-Path $repo "target/portable-update-$($incoming.version).json" }
$workspace = Join-Path ([IO.Path]::GetTempPath()) ('SuniPDF-update-regression-' + [guid]::NewGuid())
ChildPath $workspace ([IO.Path]::GetTempPath()) | Out-Null
[IO.Directory]::CreateDirectory($workspace) | Out-Null
$originalEnvironment = @{}
foreach ($name in @('APPDATA', 'LOCALAPPDATA', 'SUNIPDF_RECOVERY_DIR', 'GLUTIN_WGL_OPENGL_DLL')) { $originalEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
$results = [Collections.Generic.List[object]]::new()
$owned = [Collections.Generic.List[object]]::new()
$passed = $false
$failure = $null
$diagnostics = Join-Path $repo "target/portable-update-diagnostics-$($incoming.version)"
try {
    if ($FromPackage.Count -eq 0) { $FromPackage = @(PreviousPackages $workspace) }
    $oldPackages = @($FromPackage | ForEach-Object { PackageInfo $_ })
    Assert ($oldPackages.Count -gt 0) 'At least one previous portable release is required.'
    $env:APPDATA = Join-Path $workspace 'profile/roaming'
    $env:LOCALAPPDATA = Join-Path $workspace 'profile/local'
    $env:SUNIPDF_RECOVERY_DIR = Join-Path $workspace 'profile/recovery'
    [IO.Directory]::CreateDirectory((Join-Path $env:APPDATA 'SuniPDF')) | Out-Null
    [IO.File]::WriteAllText((Join-Path $env:APPDATA 'SuniPDF/settings.json'), '{"language":"en-US","check_updates":false,"autosave":false}')
    $expected = Join-Path $workspace 'expected'
    ExpandTestCopy $incoming.path $expected
    $incomingFiles = FileMap $expected
    $sequence = 0
    foreach ($old in $oldPackages) {
        foreach ($scenario in @('upgrade') + $(if ($old -eq $oldPackages[-1]) { @('bad-hash', 'wrong-architecture', 'rollback', 'close-timeout') } else { @() })) {
            $sequence++
            $case = Join-Path $workspace ("case-$sequence-$scenario")
            $target = Join-Path $case ('SuniPDF space ' + [char]0x00E9)
            ExpandTestCopy $old.path $target
            $testFiles = @(PrepareTestOpenGl $target)
            Copy-Item -LiteralPath (Join-Path $repo 'crates/sunipdf-core/tests/fixtures/two-pages.pdf') -Destination (Join-Path $target 'document.pdf')
            [IO.File]::WriteAllText((Join-Path $target 'settings.json'), 'unrelated portable settings')
            [IO.File]::WriteAllText((Join-Path $target 'licenses/keep-user-license.txt'), 'unrelated license')
            $before = FileMap $target
            $work = PrepareHelper $case $incoming
            $app = StartApp $target
            $owned.Add(@{process=$app; exe=(Join-Path $target 'sunipdf.exe')})
            $ownerPid = $app.Id
            $helper = $null
            $lock = $null
            try {
                if ($scenario -notin @('upgrade', 'close-timeout')) { StopOwned $app (Join-Path $target 'sunipdf.exe'); $app=$null }
                if ($scenario -eq 'rollback') { $lock=[IO.File]::Open((Join-Path $target 'pdfium.dll'),[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read) }
                $hash = if ($scenario -eq 'bad-hash') { '0' * 64 } else { $incoming.hash }
                $architecture = if ($scenario -eq 'wrong-architecture') { 'arm64' } else { 'x64' }
                $helper = StartHelper $work $target $ownerPid $hash $architecture ($scenario -eq 'upgrade')
                $owned.Add(@{process=$helper; exe=$powershell})
                if ($scenario -eq 'upgrade') {
                    Start-Sleep -Milliseconds 1500
                    AssertMap $before $target
                    Assert (-not $helper.HasExited) 'Helper did not wait for the running application.'
                    StopOwned $app (Join-Path $target 'sunipdf.exe'); $app=$null
                }
                if ($scenario -eq 'close-timeout') {
                    WaitUpdateLock $target $helper
                    $otherWork = Join-Path $case 'other'
                    [IO.Directory]::CreateDirectory($otherWork) | Out-Null
                    $otherWork = PrepareHelper $otherWork $incoming
                    $other = StartHelper $otherWork $target $ownerPid $incoming.hash
                    $owned.Add(@{process=$other; exe=$powershell})
                    $busy = WaitHelper $other $otherWork
                    Assert (-not $busy.success -and $busy.error -match 'already running') 'Concurrent update mutex did not block the second helper.'
                    Write-Output 'Waiting for the real 120-second application-close cancellation guard...'
                }
                $result = WaitHelper $helper $work
                if ($scenario -eq 'upgrade') {
                    Assert ([bool]$result.success) "Real upgrade failed: $($result.error)"
                    $after = FileMap $target
                    foreach ($name in $incomingFiles.Keys) { Assert ($after[$name] -eq $incomingFiles[$name]) "Updated payload differs: $name" }
                    foreach ($name in (@('document.pdf','settings.json','licenses\keep-user-license.txt') + $testFiles)) { Assert ($after[$name] -eq $before[$name]) "Update changed $name." }
                    $backupFiles = @{}
                    foreach ($name in $incomingFiles.Keys) {
                        if ($before.ContainsKey($name)) { $backupFiles[$name] = $before[$name] }
                    }
                    AssertMap $backupFiles $result.backup
                    $restarted = Get-Process -Id $result.restart_pid -ErrorAction Stop
                    $owned.Add(@{process=$restarted; exe=(Join-Path $target 'sunipdf.exe')})
                    Start-Sleep -Milliseconds 1000
                    Assert ($restarted.Path -ieq (Join-Path $target 'sunipdf.exe')) 'The updated application was not relaunched from its target.'
                    Assert ((Get-Item -LiteralPath $restarted.Path).VersionInfo.ProductVersion -like "$($incoming.version)*") 'Relaunch has the wrong product version.'
                    $cliVersion = & (Join-Path $target 'sunipdf-cli.exe') --version
                    Assert ($LASTEXITCODE -eq 0 -and $cliVersion -like "*$($incoming.version)*") 'Updated CLI version check failed.'
                    StopOwned $restarted (Join-Path $target 'sunipdf.exe')
                } else {
                    Assert (-not $result.success) 'A rejected/failed upgrade reported success.'
                    AssertMap $before $target
                    if ($scenario -eq 'rollback') {
                        Assert ($result.error -match 'previous application files were restored') 'Failed copy did not report a completed rollback.'
                        foreach ($name in $incomingFiles.Keys) {
                            if ($before.ContainsKey($name)) { Assert ((Get-FileHash -LiteralPath (Join-Path $result.backup $name)).Hash -eq $before[$name]) "Rollback backup differs: $name" }
                        }
                    }
                    if ($scenario -eq 'close-timeout') { Assert ($result.error -match 'did not close') 'Close cancellation did not stop installation.' }
                }
                $results.Add(@{from=$old.version; to=$incoming.version; scenario=$scenario; success=$true; helper_success=[bool]$result.success; restart_pid=$result.restart_pid; source_sha256=$old.hash; package_sha256=$incoming.hash; target_files=$before.Count})
                Write-Output "Passed: $($old.version) -> $($incoming.version), $scenario"
            } finally {
                if ($null -ne $lock) { $lock.Dispose() }
                if ($null -ne $app) { StopOwned $app (Join-Path $target 'sunipdf.exe') }
                if ($null -ne $helper) { StopOwned $helper $powershell }
            }
        }
    }
    $passed = $true
} catch {
    $failure = $_.Exception.Message
    throw
} finally {
    foreach ($process in Get-Process -Name sunipdf -ErrorAction SilentlyContinue) {
        try {
            $path = $process.Path
            if ($path -and $path.StartsWith($workspace + '\', [StringComparison]::OrdinalIgnoreCase)) { StopOwned $process $path }
        } finally { $process.Dispose() }
    }
    foreach ($item in $owned) {
        try { StopOwned $item.process $item.exe } catch { Write-Warning $_.Exception.Message }
        finally { $item.process.Dispose() }
    }
    foreach ($name in $originalEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name,$originalEnvironment[$name],'Process') }
    if (-not $passed) {
        foreach ($file in Get-ChildItem -LiteralPath $workspace -Recurse -File | Where-Object { $_.Extension -eq '.log' -or $_.Name -eq 'result.json' }) {
            $destination = ChildPath (Join-Path $diagnostics $file.FullName.Substring($workspace.Length + 1)) $diagnostics
            [IO.Directory]::CreateDirectory((Split-Path -Parent $destination)) | Out-Null
            Copy-Item -LiteralPath $file.FullName -Destination $destination -Force
        }
    }
    [IO.Directory]::CreateDirectory((Split-Path -Parent ([IO.Path]::GetFullPath($ReportPath)))) | Out-Null
    @{success=$passed; error=$failure; package=$incoming.version; cases=$results.ToArray(); isolated_profile=$true; installed_app_untouched=$true; test_workspace=$workspace; diagnostics=$diagnostics; test_opengl_overlay=$(if ($env:SUNIPDF_TEST_OPENGL_DIR) { 'Mesa 26.2.4 llvmpipe' } else { 'system OpenGL' })} |
        ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
    if ($passed) { $checked=ChildPath $workspace ([IO.Path]::GetTempPath()); Remove-Item -LiteralPath $checked -Recurse -Force }
}
Write-Output "Portable update regression passed. Report: $ReportPath"
