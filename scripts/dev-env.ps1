# Prépare la session PowerShell pour compiler SuniPDF sur ce poste.
# À « sourcer » :  . .\scripts\dev-env.ps1
#
# Cible de référence : x86_64-pc-windows-msvc (Visual Studio Build Tools + Windows SDK).
# Repli sans droits administrateur : toolchain rustup GNU + MinGW-w64 portable (WinLibs via winget),
# nécessaire car les crates `raw-dylib` exigent `dlltool` + `as`, absents des binutils livrés par rustup.
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }

$hasMsvc = $false
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vswhere) {
    $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vs) { $hasMsvc = $true }
}

if (-not $hasMsvc) {
    $winlibs = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter "BrechtSanders.WinLibs*" -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($winlibs) {
        $env:PATH = (Join-Path $winlibs.FullName "mingw64\bin") + ";" + $env:PATH
        $env:SUNIPDF_TOOLCHAIN = "gnu"
        Write-Host "MSVC absent : utilisation de MinGW-w64 ($($winlibs.Name)) + toolchain rustup GNU."
        Write-Host "Vérifier l'override : rustup override list  (attendu : stable-x86_64-pc-windows-gnu pour ce dossier)"
    } else {
        Write-Warning "Ni MSVC ni WinLibs trouvés. Installer : winget install -e --id BrechtSanders.WinLibs.POSIX.UCRT --scope user"
        Write-Warning "ou Visual Studio Build Tools (C++ Desktop) pour la cible MSVC de référence."
    }
} else {
    $env:SUNIPDF_TOOLCHAIN = "msvc"
    Write-Host "MSVC détecté : cible x86_64-pc-windows-msvc."
}
