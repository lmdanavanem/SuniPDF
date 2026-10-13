$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$fr = Get-Content (Join-Path $root "assets/i18n/fr-FR.json") -Raw | ConvertFrom-Json
$en = Get-Content (Join-Path $root "assets/i18n/en-US.json") -Raw | ConvertFrom-Json
$fk = @($fr.PSObject.Properties.Name | Sort-Object)
$ek = @($en.PSObject.Properties.Name | Sort-Object)
if (($fk -join '|') -ne ($ek -join '|')) { throw "Translation keys differ between FR and EN" }
Write-Host "OK: $($fk.Count) bilingual keys."
