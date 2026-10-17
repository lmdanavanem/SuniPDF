# Extract an MSI administratively and verify the actual payload without installing the application.
param(
    [Parameter(Mandatory = $true)] [string]$MsiPath,
    [Parameter(Mandatory = $true)] [string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$msi = (Resolve-Path -LiteralPath $MsiPath).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
$extracted = Join-Path $output ('license-inspection-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $extracted | Out-Null
$log = Join-Path $extracted 'extraction.log'
$arguments = @('/a', ('"' + $msi + '"'), '/qn', ('TARGETDIR="' + $extracted + '"'), '/l*v', ('"' + $log + '"'))
$process = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32/msiexec.exe') -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
if ($process.ExitCode -notin @(0, 3010)) { throw "MSI license inspection extraction failed: $($process.ExitCode); see $log" }
$applications = @(Get-ChildItem -LiteralPath $extracted -File -Recurse -Filter 'sunipdf.exe')
if ($applications.Count -ne 1) { throw 'The extracted MSI application payload is missing or ambiguous.' }
& python (Join-Path $PSScriptRoot 'license-inventory.py') --verify --msi --stage $applications[0].DirectoryName
if ($LASTEXITCODE -ne 0) { throw 'The extracted MSI notices or payload differ from the generated inventory.' }
