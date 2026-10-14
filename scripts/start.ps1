# Vérifie, teste puis lance SuniPDF.
param([string]$Pdf)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. (Join-Path $PSScriptRoot "dev-env.ps1")
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust/Cargo introuvable. Installer rustup depuis https://rustup.rs/ puis relancer PowerShell."
}
& (Join-Path $PSScriptRoot "fetch-pdfium.ps1")
& (Join-Path $PSScriptRoot "check-i18n.ps1")
Write-Host "cargo test --workspace -j 2"
cargo test --workspace -j 2
if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
Write-Host "cargo run -p sunipdf-desktop"
if ($Pdf) { cargo run -j 2 -p sunipdf-desktop -- $Pdf } else { cargo run -j 2 -p sunipdf-desktop }
