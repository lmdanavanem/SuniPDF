# Lance l'application, capture sa fenêtre en PNG, puis la ferme. Outil de développement/QA visuelle.
# Usage : .\scripts\dev-screenshot.ps1 -Out capture.png [-Pdf fichier.pdf] [-Delay 4] [-Exe target\debug\sunipdf.exe]
param(
    [string]$Out = "capture.png",
    [string]$Pdf = "",
    [double]$Delay = 4,
    [string]$Exe = "",
    [string]$Keys = "",
    # Identifiants de commandes exécutés après l'ouverture (ex. "view.organize,edit.find").
    [string]$Commands = "",
    # Glissement souris relatif à la fenêtre : "x1,y1,x2,y2" (points logiques).
    [string]$Drag = ""
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
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out RECT r, int size);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
if ($Commands) { $env:SUNIPDF_STARTUP_COMMANDS = $Commands } else { Remove-Item Env:SUNIPDF_STARTUP_COMMANDS -ErrorAction SilentlyContinue }
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
    $r = New-Object Win+RECT
    # Le passage au premier plan peut produire un clic parasite : placer la souris au centre (zone document).
    [Win]::GetWindowRect($h, [ref]$r) | Out-Null
    [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point((($r.Left + $r.Right) / 2), (($r.Top + $r.Bottom) / 2))
    Start-Sleep -Milliseconds 300
    if ($Keys) { Start-Sleep -Milliseconds 500; [System.Windows.Forms.SendKeys]::SendWait($Keys); Start-Sleep -Seconds 1.5 }
    if ($Drag) {
        $d = $Drag.Split(",") | ForEach-Object { [int]$_ }
        $scale = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width / [System.Windows.Forms.SystemInformation]::VirtualScreen.Width
        $x1 = $r.Left + $d[0]; $y1 = $r.Top + $d[1]; $x2 = $r.Left + $d[2]; $y2 = $r.Top + $d[3]
        [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point($x1, $y1)
        Start-Sleep -Milliseconds 200
        [Win]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)   # LEFTDOWN
        Start-Sleep -Milliseconds 150
        $steps = 12
        for ($i = 1; $i -le $steps; $i++) {
            $x = $x1 + ($x2 - $x1) * $i / $steps; $y = $y1 + ($y2 - $y1) * $i / $steps
            [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point([int]$x, [int]$y)
            Start-Sleep -Milliseconds 30
        }
        [Win]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)   # LEFTUP
        Start-Sleep -Seconds 1.5
    }
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
