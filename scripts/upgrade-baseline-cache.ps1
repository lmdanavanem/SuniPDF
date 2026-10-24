# Historical package bytes are reusable; every use still verifies the pinned hash and live sidecar.
function Read-SuniBaselinePins([string]$ScriptPath, [string]$VariableName) {
    $tokens = $null
    $errors = $null
    $tree = [System.Management.Automation.Language.Parser]::ParseFile($ScriptPath, [ref]$tokens, [ref]$errors)
    if ($errors.Count) { throw 'Cannot read baseline pins from an invalid PowerShell script.' }
    $assignments = @($tree.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.AssignmentStatementAst] -and
        $node.Left -is [System.Management.Automation.Language.VariableExpressionAst] -and
        $node.Left.VariablePath.UserPath -eq $VariableName
    }, $true))
    if ($assignments.Count -ne 1) { throw 'Expected exactly one literal baseline pin table.' }
    # Only constant data is accepted. Never dot-source a test that can start/install applications.
    $value = $assignments[0].Right
    if ($value -isnot [System.Management.Automation.Language.CommandExpressionAst] -or
        $value.Redirections.Count -ne 0 -or
        $value.Expression -isnot [System.Management.Automation.Language.HashtableAst]) {
        throw 'Baseline pins must be assigned directly from a literal table.'
    }
    $pins = $value.Expression.SafeGetValue()
    if ($pins -isnot [Collections.IDictionary] -or $pins.Count -eq 0) { throw 'Baseline pins are not a nonempty literal table.' }
    return $pins
}

function Assert-SuniBaselinePath([string]$Path, [string]$CacheRoot) {
    $full = [IO.Path]::GetFullPath($Path)
    $root = [IO.Path]::GetFullPath($CacheRoot).TrimEnd('\', '/')
    if ($full -ne $root -and -not $full.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Baseline cache path escaped its root.'
    }
    # A restored cache must not redirect reads/writes through junctions or symbolic links.
    $ancestor = $full
    while ($ancestor) {
        if (Test-Path -LiteralPath $ancestor) {
            $item = Get-Item -LiteralPath $ancestor -Force
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw 'Baseline cache paths must not contain reparse points.'
            }
        }
        $parent = [IO.Path]::GetDirectoryName($ancestor)
        if ($parent -eq $ancestor) { break }
        $ancestor = $parent
    }
    return $full
}

function Get-SuniBaselineDigest([string]$Path) {
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($algorithm.ComputeHash($stream)).Replace('-', '') }
    finally { $algorithm.Dispose(); $stream.Dispose() }
}

function Read-SuniBaselineSidecar([string]$Uri) {
    $body = (Invoke-WebRequest -UseBasicParsing -Uri $Uri -TimeoutSec 60 -ErrorAction Stop).Content
    if ($body -is [byte[]]) { return [Text.Encoding]::ASCII.GetString($body) }
    return [string]$body
}

function Save-SuniBaselineArchive([string]$Uri, [string]$Destination) {
    Invoke-WebRequest -UseBasicParsing -Uri $Uri -OutFile $Destination -TimeoutSec 300 -ErrorAction Stop | Out-Null
}

function Set-SuniBaselineFile([string]$Temporary, [string]$Destination, [string]$CacheRoot) {
    $null = Assert-SuniBaselinePath $Temporary $CacheRoot
    $null = Assert-SuniBaselinePath $Destination $CacheRoot
    # Both paths are siblings: promote only complete, validated bytes on the same volume.
    if ([IO.File]::Exists($Destination)) {
        [IO.File]::Replace($Temporary, $Destination, [System.Management.Automation.Language.NullString]::Value)
    }
    else { [IO.File]::Move($Temporary, $Destination) }
}

function Get-SuniUpgradeBaseline {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)] [ValidatePattern('^\d+\.\d+\.\d+$')] [string]$Version,
        [Parameter(Mandatory)] [ValidatePattern('^\d+\.\d+\.\d+$')] [string]$CurrentVersion,
        [Parameter(Mandatory)] [ValidatePattern('^[A-Fa-f0-9]{64}$')] [string]$ExpectedSha256,
        [Parameter(Mandatory)] [ValidateSet('portable', 'msi')] [string]$Kind,
        [string]$CacheDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'target/upgrade-baseline-cache/v1')
    )
    if ([version]$Version -ge [version]$CurrentVersion) { throw 'Only older released packages belong in the baseline cache.' }
    $hash = $ExpectedSha256.ToUpperInvariant()
    $suffix = if ($Kind -eq 'portable') { 'win-x64-portable.zip' } else { 'win-x64.msi' }
    $name = "SuniPDF-$Version-$suffix"
    $uri = "https://github.com/lmdanavanem/SuniPDF/releases/download/v$Version/$name"
    $cacheRoot = [IO.Path]::GetFullPath($CacheDirectory)
    $directory = Assert-SuniBaselinePath (Join-Path (Join-Path $cacheRoot $Version) $hash) $cacheRoot
    $path = Assert-SuniBaselinePath (Join-Path $directory $name) $cacheRoot
    $sidecarPath = Assert-SuniBaselinePath "$path.sha256" $cacheRoot
    # A cache hit never skips the current published checksum/filename verification.
    $sidecar = (Read-SuniBaselineSidecar "$uri.sha256").Trim() -split '\s+'
    if ($sidecar.Count -ne 2 -or $sidecar[0] -ine $hash -or $sidecar[1] -cne $name) {
        throw "Pinned release digest disagrees with the published checksum: $Version ($Kind)"
    }
    [IO.Directory]::CreateDirectory($directory) | Out-Null
    $null = Assert-SuniBaselinePath $path $cacheRoot
    $temporary = Join-Path $directory ('.download-' + [guid]::NewGuid().ToString('N'))
    $sidecarTemporary = Join-Path $directory ('.checksum-' + [guid]::NewGuid().ToString('N'))
    try {
        if (-not [IO.File]::Exists($path) -or (Get-SuniBaselineDigest $path) -ine $hash) {
            Save-SuniBaselineArchive $uri $temporary
            $null = Assert-SuniBaselinePath $temporary $cacheRoot
            if ((Get-SuniBaselineDigest $temporary) -ine $hash) {
                throw "Pinned previous release checksum mismatch: $Version ($Kind)"
            }
            Set-SuniBaselineFile $temporary $path $cacheRoot
            Write-Host "Downloaded and verified historical $Kind baseline $Version."
        } else {
            Write-Host "Reused and rehashed historical $Kind baseline $Version."
        }
        # Local checksum files are output metadata, never a source of trust.
        [IO.File]::WriteAllText($sidecarTemporary, "$hash  $name", [Text.Encoding]::ASCII)
        Set-SuniBaselineFile $sidecarTemporary $sidecarPath $cacheRoot
        return $path
    } finally {
        foreach ($pending in @($temporary, $sidecarTemporary)) {
            $null = Assert-SuniBaselinePath $pending $cacheRoot
            if ([IO.File]::Exists($pending)) { [IO.File]::Delete($pending) }
        }
    }
}
