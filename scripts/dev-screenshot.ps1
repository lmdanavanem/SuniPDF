# Lance l'application, capture sa fenêtre en PNG, puis la ferme. Outil de développement/QA visuelle.
# Usage : .\scripts\dev-screenshot.ps1 -Out capture.png [-Pdf fichier.pdf] [-Delay 4] [-Exe target\debug\sunipdf.exe]
param(
    [string]$Out = "capture.png",
    [string]$Pdf = "",
    [double]$Delay = 4,
    [string]$Exe = "",
    [string]$Keys = ""
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $root "target\debug\sunipdf.exe" }
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out RECT r, int size);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$args_ = @(); if ($Pdf) { $args_ = @($Pdf) }
$proc = if ($args_.Count -gt 0) { Start-Process -FilePath $Exe -ArgumentList $args_ -PassThru } else { Start-Process -FilePath $Exe -PassThru }
try {
    Start-Sleep -Seconds $Delay
    $proc.Refresh()
    $h = $proc.MainWindowHandle
    if ($h -eq [IntPtr]::Zero) { throw "Fenêtre introuvable (processus $($proc.Id))." }
    # Premier plan + position (0,0) sans redimensionner (SWP_NOSIZE=1, SWP_SHOWWINDOW=0x40), HWND_TOPMOST=-1.
    [Win]::ShowWindow($h, 5) | Out-Null
    [Win]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x41) | Out-Null
    [Win]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 700
    if ($Keys) { Start-Sleep -Milliseconds 500; [System.Windows.Forms.SendKeys]::SendWait($Keys); Start-Sleep -Seconds 1.5 }
    $r = New-Object Win+RECT
    # DWMWA_EXTENDED_FRAME_BOUNDS = 9 : rectangle visible sans l'ombre invisible de Windows 10/11.
    if ([Win]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) -ne 0) { [Win]::GetWindowRect($h, [ref]$r) | Out-Null }
    $w = $r.Right - $r.Left; $hgt = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $hgt
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Host "Capture : $Out ($w x $hgt)"
} finally {
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
}
