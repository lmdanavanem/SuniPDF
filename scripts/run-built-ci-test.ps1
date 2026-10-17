# Run the exact ignored integration-test executable recorded by the workspace test gate.
param(
    [ValidateSet('native_wgpu','qpdf_corpus','real_corpus','office_fidelity_corpus')]
    [string]$TestName = 'native_wgpu',
    [switch]$SelfTest
)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
function Get-TestExecutable([string]$Text, [string]$Name) {
    $plain = [regex]::Replace($Text, '\x1B\[[0-?]*[ -/]*[@-~]', '')
    $pattern = '(?m)^\s*Running\s+tests[\\/]' + [regex]::Escape($Name) + '\.rs\s+\((?<exe>[^\r\n)]+)\)\s*$'
    $found = [regex]::Matches($plain, $pattern)
    if ($found.Count -ne 1) { throw 'Workspace log must identify exactly one requested test executable.' }
    $relative = $found[0].Groups['exe'].Value
    $allowed = '^target[\\/]debug[\\/]deps[\\/]' + [regex]::Escape($Name) + '-[a-f0-9]{16}\.exe$'
    if ($relative -cnotmatch $allowed) { throw 'Workspace test executable is outside its expected directory.' }
    return [IO.Path]::GetFullPath((Join-Path $root $relative))
}
if ($SelfTest) {
    $valid = "Running tests\native_wgpu.rs (target\debug\deps\native_wgpu-0123456789abcdef.exe)"
    $expected = Join-Path $root 'target\debug\deps\native_wgpu-0123456789abcdef.exe'
    if ((Get-TestExecutable $valid 'native_wgpu') -cne $expected) { throw 'Workspace executable selection failed.' }
    foreach ($invalid in @('', "$valid`n$valid", $valid.Replace('target\debug\deps', '..\outside'), $valid.Replace('native_wgpu-0123456789abcdef.exe', 'other.exe'))) {
        $refused = $false
        try { Get-TestExecutable $invalid 'native_wgpu' | Out-Null } catch { $refused = $true }
        if (-not $refused) { throw 'Unsafe or ambiguous workspace executable was accepted.' }
    }
    Write-Host 'Built-test selection and path bounds passed. No executable was run.'
    exit 0
}
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:GITHUB_REPOSITORY -ne 'lmdanavanem/SuniPDF' -or
    $env:RUNNER_OS -ne 'Windows' -or $env:RUNNER_ARCH -ne 'X64') { throw 'Built integration gates run only in this repository Windows x64 CI.' }
$log = Join-Path $root 'target/workspace-tests.log'
if (-not (Test-Path -LiteralPath $log -PathType Leaf) -or (Get-Item -LiteralPath $log).Length -gt 8MB) { throw 'Workspace test log is missing or exceeds its size bound.' }
$executable = Get-TestExecutable ([IO.File]::ReadAllText($log)) $TestName
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw 'The workspace-built test executable is missing.' }
$package = switch ($TestName) {
    'native_wgpu' { 'apps/sunipdf-desktop' }
    'office_fidelity_corpus' { 'crates/sunipdf-core' }
    default { 'apps/sunipdf-cli' }
}
Push-Location (Join-Path $root $package)
try {
    & $executable --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw "The workspace-built integration gate failed: $TestName" }
} finally { Pop-Location }
