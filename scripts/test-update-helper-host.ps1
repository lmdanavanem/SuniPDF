# Windows PowerShell children must work without the parent's PowerShell module paths.
$ErrorActionPreference = 'Stop'
$helper = Join-Path (Split-Path -Parent $PSScriptRoot) 'apps/sunipdf-desktop/src/update-helper.ps1'
$powershell = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
$workspace = Join-Path ([IO.Path]::GetTempPath()) ('SuniPDF-helper-host-test-' + [guid]::NewGuid())
[IO.Directory]::CreateDirectory($workspace) | Out-Null
try {
    $bootstrap = Join-Path $workspace 'bootstrap.ps1'
    [IO.File]::WriteAllText($bootstrap, @'
$ErrorActionPreference = 'Stop'
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Management\Microsoft.PowerShell.Management.psd1"
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1"
$env:PSModulePath = $PSScriptRoot
$PSModuleAutoLoadingPreference = 'None'
Remove-Item -LiteralPath Function:\Get-FileHash -ErrorAction SilentlyContinue
if ($PSVersionTable.PSVersion.Major -ne 5) { throw 'The regression requires Windows PowerShell 5.' }
if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) { throw 'The regression requires Get-FileHash to be unavailable.' }
$probe = Join-Path $PSScriptRoot 'result-probe.json'
@{ success = $true } | ConvertTo-Json | Set-Content -LiteralPath $probe -Encoding UTF8
if ([IO.File]::ReadAllText($probe) -notmatch '"success"\s*:\s*true') { throw 'Result serialization is unavailable.' }
Get-Process -Id $PID | Out-Null
& $args[0] -SelfTest
'@, [Text.Encoding]::ASCII)
    $stdout = Join-Path $workspace 'stdout.log'
    $stderr = Join-Path $workspace 'stderr.log'
    $arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + $bootstrap + '" "' + $helper + '"'
    $process = Start-Process -FilePath $powershell -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    try {
        if ($process.ExitCode -ne 0) { throw "Windows PowerShell module-path regression failed: $([IO.File]::ReadAllText($stderr))" }
    } finally { $process.Dispose() }
    Write-Output ([IO.File]::ReadAllText($stdout).Trim())
    Write-Output 'Windows PowerShell 5 helper passed with Get-FileHash unavailable and stripped module paths.'
} finally {
    $checked = [IO.Path]::GetFullPath($workspace)
    $prefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if (-not $checked.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe test workspace cleanup path.' }
    Remove-Item -LiteralPath $checked -Recurse -Force
}
