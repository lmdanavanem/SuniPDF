param(
    [ValidateSet("Msi", "Portable")] [string]$Mode,
    [int]$ParentPid,
    [string]$PackagePath,
    [ValidatePattern('^[A-Fa-f0-9]{64}$')] [string]$ExpectedSha256,
    [string]$TargetDirectory,
    [ValidateSet("x64", "arm64")] [string]$Arch,
    [ValidateSet("sunipdf.exe")] [string]$ExecutableName = "sunipdf.exe",
    [switch]$Restart,
    [switch]$NonInteractive,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-AbsolutePath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or $Path -match '^\\\\[?.]\\' -or
        $Path -notmatch '^(?:[A-Za-z]:[\\/]|[\\/]{2}[^\\/]+[\\/][^\\/]+)') {
        throw "An absolute path is required."
    }
    $full = [IO.Path]::GetFullPath($Path)
    if ($full -eq [IO.Path]::GetPathRoot($full)) { return $full }
    return $full.TrimEnd('\', '/')
}

function Assert-ChildPath([string]$Path, [string]$Root) {
    $absolute = Get-AbsolutePath $Path
    $parent = (Get-AbsolutePath $Root) + [IO.Path]::DirectorySeparatorChar
    if (-not $absolute.StartsWith($parent, [StringComparison]::OrdinalIgnoreCase)) {
        throw "The path is outside its allowed directory: $absolute"
    }
    return $absolute
}

function Assert-NoReparsePoint([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    while ($null -ne $item) {
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Linked directories and files cannot be updated: $Path"
        }
        $item = $item.Parent
    }
}

function Get-Sha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $digest = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($digest.ComputeHash($stream)).Replace('-', '') }
    finally { $digest.Dispose(); $stream.Dispose() }
}

function Assert-PackageHash([string]$Path, [string]$Expected) {
    if ($Expected -notmatch '^[A-Fa-f0-9]{64}$' -or
        (Get-Sha256 $Path) -ine $Expected) {
        throw "The downloaded package failed its SHA256 check."
    }
}

function Assert-TargetDirectory([string]$Directory, [string]$ExeName) {
    $target = Get-AbsolutePath $Directory
    $roots = @([IO.Path]::GetPathRoot($target), $env:SystemRoot, $env:USERPROFILE,
        $env:ProgramFiles, [Environment]::GetEnvironmentVariable('ProgramFiles(x86)'), [IO.Path]::GetTempPath())
    foreach ($root in $roots) {
        if ($root -and $target -ieq (Get-AbsolutePath $root)) {
            throw "An application subdirectory is required for the update."
        }
    }
    Assert-NoReparsePoint $target
    if (-not (Test-Path -LiteralPath (Join-Path $target $ExeName) -PathType Leaf)) {
        throw "The application directory does not contain $ExeName."
    }
    return $target
}

function Expand-Portable([string]$ArchivePath, [string]$Stage) {
    $allowed = @("sunipdf.exe", "sunipdf-cli.exe", "pdfium.dll", "README.txt", "LISEZMOI.txt",
        "USER_GUIDE.en.md", "USER_GUIDE.fr.md", "CHANGELOG.md", "CHANGELOG.fr.md")
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $files = [Collections.Generic.List[string]]::new()
    $archive = [IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        if ($archive.Entries.Count -gt 512) { throw "The update contains too many files." }
        [long]$total = 0
        foreach ($entry in $archive.Entries) {
            $name = $entry.FullName.Replace('\', '/')
            if ($name -match '(^/|(^|/)\.\.?(/|$)|//)' -or $name -match '[\x00-\x1f<>:"|?*]') {
                throw "The update contains an unsafe archive path."
            }
            if ($name -eq 'licenses/') { continue }
            if (($allowed -notcontains $name) -and $name -notmatch '^licenses/[^/]+$') {
                throw "The update contains an unexpected file: $name"
            }
            if ($name.EndsWith('/') -or $name -match '[. ]$' -or
                (Split-Path -Leaf $name) -match '^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\.|$)' -or
                -not $names.Add($name)) {
                throw "The update contains an invalid or duplicate filename."
            }
            $total += $entry.Length
            if ($entry.Length -gt 268435456 -or $total -gt 536870912) {
                throw "The expanded update is too large."
            }
            $files.Add($name)
        }
        foreach ($required in @("sunipdf.exe", "sunipdf-cli.exe", "pdfium.dll")) {
            if (-not $names.Contains($required)) { throw "The update is missing $required." }
        }
        [IO.Directory]::CreateDirectory($Stage) | Out-Null
        foreach ($entry in $archive.Entries) {
            $name = $entry.FullName.Replace('\', '/')
            if ($name -eq 'licenses/') { continue }
            $destination = Assert-ChildPath (Join-Path $Stage $name) $Stage
            [IO.Directory]::CreateDirectory((Split-Path -Parent $destination)) | Out-Null
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $destination, $false)
        }
    } finally {
        $archive.Dispose()
    }
    return $files.ToArray()
}

function Assert-PortableArchitecture([string]$Stage, [string]$Architecture) {
    $expected = if ($Architecture -eq 'arm64') { 0xAA64 } else { 0x8664 }
    foreach ($name in @('sunipdf.exe', 'sunipdf-cli.exe', 'pdfium.dll')) {
        $stream = [IO.File]::OpenRead((Join-Path $Stage $name))
        $reader = [IO.BinaryReader]::new($stream)
        try {
            if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) {
                throw "The update contains an invalid Windows binary: $name"
            }
            $stream.Position = 60
            $header = $reader.ReadInt32()
            if ($header -lt 64 -or $header -gt ($stream.Length - 6)) {
                throw "The update contains an invalid PE header: $name"
            }
            $stream.Position = $header
            if ($reader.ReadUInt32() -ne 0x00004550 -or $reader.ReadUInt16() -ne $expected) {
                throw "The update architecture does not match this application: $name"
            }
        } finally { $reader.Dispose(); $stream.Dispose() }
    }
}

function Install-Portable([string]$Stage, [string]$Target, [string]$Backup, [string[]]$Files) {
    $changed = [Collections.Generic.List[string]]::new()
    [IO.Directory]::CreateDirectory($Backup) | Out-Null
    foreach ($name in $Files) {
        $destination = Assert-ChildPath (Join-Path $Target $name) $Target
        Assert-NoReparsePoint $destination
        Assert-NoReparsePoint (Split-Path -Parent $destination)
        if (Test-Path -LiteralPath $destination) {
            if (-not (Test-Path -LiteralPath $destination -PathType Leaf)) {
                throw "An update destination is not a regular file: $destination"
            }
            $saved = Assert-ChildPath (Join-Path $Backup $name) $Backup
            [IO.Directory]::CreateDirectory((Split-Path -Parent $saved)) | Out-Null
            [IO.File]::Copy($destination, $saved, $false)
        }
    }
    try {
        foreach ($name in $Files) {
            $destination = Assert-ChildPath (Join-Path $Target $name) $Target
            [IO.Directory]::CreateDirectory((Split-Path -Parent $destination)) | Out-Null
            $changed.Add($name)
            [IO.File]::Copy((Join-Path $Stage $name), $destination, $true)
        }
    } catch {
        $failure = $_.Exception.Message
        $rollbackErrors = [Collections.Generic.List[string]]::new()
        for ($i = $changed.Count - 1; $i -ge 0; $i--) {
            $name = $changed[$i]
            $destination = Assert-ChildPath (Join-Path $Target $name) $Target
            $saved = Assert-ChildPath (Join-Path $Backup $name) $Backup
            try {
                if (Test-Path -LiteralPath $saved -PathType Leaf) {
                    if (-not (Test-Path -LiteralPath $destination -PathType Leaf) -or
                        (Get-Sha256 $destination) -ne (Get-Sha256 $saved)) {
                        [IO.File]::Copy($saved, $destination, $true)
                    }
                } elseif (Test-Path -LiteralPath $destination -PathType Leaf) {
                    Remove-Item -LiteralPath $destination -Force
                }
            } catch { $rollbackErrors.Add($name) }
        }
        if ($rollbackErrors.Count -gt 0) {
            throw "$failure Recovery files remain in $Backup. Restore: $($rollbackErrors -join ', ')."
        }
        throw "$failure The previous application files were restored. Recovery files: $Backup"
    }
}

function Invoke-SelfTest {
    $workspace = Join-Path ([IO.Path]::GetTempPath()) ("SuniPDF-update-test-" + [guid]::NewGuid())
    [IO.Directory]::CreateDirectory($workspace) | Out-Null
    try {
        $knownHash = Join-Path $workspace 'known-hash.txt'
        [IO.File]::WriteAllBytes($knownHash, [byte[]]@(97, 98, 99))
        if ((Get-Sha256 $knownHash) -ne 'BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD') {
            throw 'SHA256 known-value check failed.'
        }
        $outside = Join-Path (Split-Path -Parent $workspace) 'outside.pdf'
        try { Assert-ChildPath $outside $workspace | Out-Null; throw "Boundary check failed." }
        catch { if ($_.Exception.Message -eq 'Boundary check failed.') { throw } }
        $source = Join-Path $workspace 'source'
        [IO.Directory]::CreateDirectory((Join-Path $source 'licenses')) | Out-Null
        $payload = @('sunipdf.exe', 'sunipdf-cli.exe', 'README.txt', 'licenses/LICENSE.txt',
            'licenses/tesseract.exe', 'licenses/eng.traineddata', 'licenses/fra.traineddata',
            'licenses/ocr-NOTICE.txt', 'licenses/ocr-AUTHORS.txt', 'pdfium.dll')
        foreach ($name in $payload) {
            [IO.File]::WriteAllText((Join-Path $source $name), "new:$name")
        }
        $package = Join-Path $workspace 'package.zip'
        $zip = [IO.Compression.ZipFile]::Open($package, [IO.Compression.ZipArchiveMode]::Create)
        foreach ($name in $payload) {
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $source $name), $name) | Out-Null
        }
        $zip.Dispose()
        $hash = Get-Sha256 $package
        Assert-PackageHash $package $hash
        try { Assert-PackageHash $package ('0' * 64); throw "Hash check failed." }
        catch { if ($_.Exception.Message -eq 'Hash check failed.') { throw } }
        $stage = Join-Path $workspace 'stage'
        $files = @(Expand-Portable $package $stage)
        $target = Join-Path $workspace 'target'
        [IO.Directory]::CreateDirectory($target) | Out-Null
        [IO.Directory]::CreateDirectory((Join-Path $target 'licenses')) | Out-Null
        foreach ($name in @('sunipdf.exe', 'sunipdf-cli.exe', 'pdfium.dll', 'document.pdf', 'settings.json', 'licenses/tesseract.exe', 'licenses/eng.traineddata')) {
            [IO.File]::WriteAllText((Join-Path $target $name), "old:$name")
        }
        Assert-TargetDirectory $target 'sunipdf.exe' | Out-Null
        try { Assert-TargetDirectory ([IO.Path]::GetPathRoot($target)) 'sunipdf.exe' | Out-Null; throw "Target root check failed." }
        catch { if ($_.Exception.Message -eq 'Target root check failed.') { throw } }
        try { Assert-PackageHash $package ('0' * 64); throw "Hash check failed." }
        catch { if ($_.Exception.Message -eq 'Hash check failed.') { throw } }
        foreach ($name in @('sunipdf.exe', 'document.pdf', 'settings.json')) {
            if ([IO.File]::ReadAllText((Join-Path $target $name)) -ne "old:$name") {
                throw "A failed checksum changed the application directory."
            }
        }
        $lock = [IO.File]::Open((Join-Path $target 'pdfium.dll'), [IO.FileMode]::Open,
            [IO.FileAccess]::Read, [IO.FileShare]::Read)
        try {
            try {
                Install-Portable $stage $target (Join-Path $workspace 'rollback') $files
                throw "Rollback check failed."
            } catch { if ($_.Exception.Message -eq 'Rollback check failed.') { throw } }
        } finally { $lock.Dispose() }
        foreach ($name in @('sunipdf.exe', 'sunipdf-cli.exe', 'pdfium.dll', 'document.pdf', 'settings.json', 'licenses/tesseract.exe', 'licenses/eng.traineddata')) {
            if ([IO.File]::ReadAllText((Join-Path $target $name)) -ne "old:$name") {
                throw "Rollback did not preserve $name."
            }
        }
        foreach ($name in @('README.txt', 'licenses/LICENSE.txt', 'licenses/fra.traineddata', 'licenses/ocr-NOTICE.txt', 'licenses/ocr-AUTHORS.txt')) {
            if (Test-Path -LiteralPath (Join-Path $target $name)) { throw "Rollback left a new file." }
        }
        if ([IO.File]::ReadAllText((Join-Path $workspace 'rollback/sunipdf.exe')) -ne 'old:sunipdf.exe') {
            throw "The recovery copy was not preserved."
        }
        if ([IO.File]::ReadAllText((Join-Path $workspace 'rollback/licenses/tesseract.exe')) -ne 'old:licenses/tesseract.exe') {
            throw 'The previous OCR runtime recovery copy was not preserved.'
        }
        Install-Portable $stage $target (Join-Path $workspace 'backup') $files
        foreach ($name in $files) {
            if ([IO.File]::ReadAllText((Join-Path $target $name)) -ne "new:$name") {
                throw "The update did not replace $name."
            }
        }
        foreach ($name in @('document.pdf', 'settings.json')) {
            if ([IO.File]::ReadAllText((Join-Path $target $name)) -ne "old:$name") {
                throw "The update changed an unrelated file."
            }
        }
        $unsafe = Join-Path $workspace 'unsafe.zip'
        $zip = [IO.Compression.ZipFile]::Open($unsafe, [IO.Compression.ZipArchiveMode]::Create)
        $zip.CreateEntry('../outside.exe') | Out-Null
        $zip.Dispose()
        try { Expand-Portable $unsafe (Join-Path $workspace 'unsafe') | Out-Null; throw "Traversal check failed." }
        catch { if ($_.Exception.Message -eq 'Traversal check failed.') { throw } }
        if (Test-Path -LiteralPath (Join-Path $workspace 'unsafe')) { throw "An unsafe archive created output." }
        $nested = Join-Path $workspace 'unexpected-ocr.zip'
        $zip = [IO.Compression.ZipFile]::Open($nested, [IO.Compression.ZipArchiveMode]::Create)
        $zip.CreateEntry('licenses/ocr/tesseract.exe') | Out-Null
        $zip.Dispose()
        try { Expand-Portable $nested (Join-Path $workspace 'unexpected-ocr') | Out-Null; throw 'Nested payload check failed.' }
        catch { if ($_.Exception.Message -eq 'Nested payload check failed.') { throw } }
        if (Test-Path -LiteralPath (Join-Path $workspace 'unexpected-ocr')) { throw 'An unexpected OCR subtree created output.' }
        $oversized = Join-Path $workspace 'too-many.zip'
        $zip = [IO.Compression.ZipFile]::Open($oversized, [IO.Compression.ZipArchiveMode]::Create)
        for ($i = 0; $i -lt 513; $i++) { $zip.CreateEntry("licenses/$i.txt") | Out-Null }
        $zip.Dispose()
        try { Expand-Portable $oversized (Join-Path $workspace 'too-many') | Out-Null; throw "Archive limit check failed." }
        catch { if ($_.Exception.Message -eq 'Archive limit check failed.') { throw } }
        $pe = Join-Path $workspace 'pe'
        [IO.Directory]::CreateDirectory($pe) | Out-Null
        $header = [byte[]]::new(70)
        $header[0] = 0x4D; $header[1] = 0x5A; $header[60] = 64
        $header[64] = 0x50; $header[65] = 0x45; $header[68] = 0x64; $header[69] = 0x86
        foreach ($name in @('sunipdf.exe', 'sunipdf-cli.exe', 'pdfium.dll')) {
            [IO.File]::WriteAllBytes((Join-Path $pe $name), $header)
        }
        Assert-PortableArchitecture $pe 'x64'
        try { Assert-PortableArchitecture $pe 'arm64'; throw "Architecture check failed." }
        catch { if ($_.Exception.Message -eq 'Architecture check failed.') { throw } }
        Write-Output 'Update helper self-test passed: path bounds, checksum, archive traversal/limits, architecture, full copy and rollback including flat OCR payloads.'
    } finally {
        $checked = Assert-ChildPath $workspace ([IO.Path]::GetTempPath())
        Remove-Item -LiteralPath $checked -Recurse -Force
    }
}

if ($SelfTest) { Invoke-SelfTest; exit 0 }

$resultPath = $null
$mutex = $null
$ownsMutex = $false
$packageLock = $null
$backup = $null
$applied = $false
$restartPid = $null
try {
    if (-not $Mode -or -not $Arch -or $ParentPid -le 0 -or $ParentPid -eq $PID) {
        throw "An update mode and a valid parent process are required."
    }
    $workspace = Assert-ChildPath $PSScriptRoot ([IO.Path]::GetTempPath())
    Assert-NoReparsePoint $workspace
    $package = Assert-ChildPath $PackagePath $workspace
    if ((Split-Path -Parent $package) -ine $workspace -or
        -not (Test-Path -LiteralPath $package -PathType Leaf)) {
        throw "The package must be a regular file beside the update helper."
    }
    Assert-NoReparsePoint $package
    $target = Assert-TargetDirectory $TargetDirectory $ExecutableName
    if (($Mode -eq 'Msi' -and [IO.Path]::GetExtension($package) -ine '.msi') -or
        ($Mode -eq 'Portable' -and [IO.Path]::GetExtension($package) -ine '.zip')) {
        throw "The package extension does not match the update mode."
    }
    $resultPath = Join-Path $workspace 'result.json'
    $digest = [Security.Cryptography.SHA256]::Create()
    try { $targetId = [BitConverter]::ToString($digest.ComputeHash([Text.Encoding]::UTF8.GetBytes($target.ToLowerInvariant()))).Replace('-', '') }
    finally { $digest.Dispose() }
    $mutex = [Threading.Mutex]::new($false, "Local\SuniPDF-update-$targetId")
    $ownsMutex = $mutex.WaitOne(0)
    if (-not $ownsMutex) { throw "Another update is already running for this application." }
    $parent = Get-Process -Id $ParentPid -ErrorAction SilentlyContinue
    if ($null -ne $parent) {
        try {
            $parentPath = $null
            try { $parentPath = $parent.Path } catch { if (-not $parent.HasExited) { throw } }
            if (-not $parent.HasExited -and (-not $parentPath -or
                (Get-AbsolutePath $parentPath) -ine (Join-Path $target $ExecutableName))) {
                throw "The parent process does not belong to the application being updated."
            }
            if (-not $parent.WaitForExit(120000)) { throw "The application did not close; the update was not applied." }
        } finally { $parent.Dispose() }
    }
    $packageLock = [IO.File]::Open($package, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    Assert-PackageHash $package $ExpectedSha256
    Assert-NoReparsePoint $target
    $rebootRequired = $false
    if ($Mode -eq 'Portable') {
        $stage = Join-Path $workspace ('stage-' + [guid]::NewGuid())
        $backup = Join-Path $workspace ('backup-' + [guid]::NewGuid())
        $files = @(Expand-Portable $package $stage)
        Assert-PortableArchitecture $stage $Arch
        Install-Portable $stage $target $backup $files
    } else {
        $log = Join-Path $workspace 'installer.log'
        $arguments = '/i "' + $package + '" /norestart INSTALLFOLDER="' + $target + '" /L*v "' + $log + '"'
        $installer = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\msiexec.exe') -ArgumentList $arguments -Wait -PassThru
        try {
            if ($installer.ExitCode -notin @(0, 3010)) { throw "Windows Installer returned $($installer.ExitCode). See $log." }
            $rebootRequired = $installer.ExitCode -eq 3010
        } finally { $installer.Dispose() }
    }
    $applied = $true
    if ($rebootRequired -and -not $NonInteractive) {
        Add-Type -AssemblyName System.Windows.Forms
        [Windows.Forms.MessageBox]::Show('SuniPDF was updated. Restart Windows before opening it again.',
            'SuniPDF update', 'OK', 'Information') | Out-Null
    } elseif ($Restart -and -not $rebootRequired) {
        $windowStyle = if ($NonInteractive) { 'Hidden' } else { 'Normal' }
        $restarted = Start-Process -FilePath (Join-Path $target $ExecutableName) -WorkingDirectory $target -WindowStyle $windowStyle -PassThru
        try { $restartPid = $restarted.Id } finally { $restarted.Dispose() }
    }
    @{ success = $true; reboot_required = $rebootRequired; backup = $backup; restart_pid = $restartPid } |
        ConvertTo-Json | Set-Content -LiteralPath $resultPath -Encoding UTF8
} catch {
    $message = $_.Exception.Message
    if ($applied) { $message = "SuniPDF was updated, but could not finish restarting. $message Previous files: $backup" }
    if ($resultPath) {
        @{ success = $false; error = $message; backup = $backup } | ConvertTo-Json |
            Set-Content -LiteralPath $resultPath -Encoding UTF8 -ErrorAction SilentlyContinue
    }
    if ($NonInteractive) {
        Write-Error -Message $message -ErrorAction Continue
    } else {
        Add-Type -AssemblyName System.Windows.Forms
        [Windows.Forms.MessageBox]::Show($message, 'SuniPDF update', 'OK', 'Error') | Out-Null
    }
    exit 1
} finally {
    if ($null -ne $packageLock) { $packageLock.Dispose() }
    if ($ownsMutex) { $mutex.ReleaseMutex() }
    if ($null -ne $mutex) { $mutex.Dispose() }
}
