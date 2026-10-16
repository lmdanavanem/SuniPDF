# Test runner only. Third-party Mesa binaries stay under ignored target/, never application packages.
# Distributor: https://github.com/pal1000/mesa-dist-win/releases/tag/26.2.4
# Upstream deployment: https://docs.mesa3d.org/drivers/llvmpipe.html#windows
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$version = '26.2.4'
$root = Split-Path -Parent $PSScriptRoot
$stage = Join-Path $root "target/test-opengl/mesa-$version-x64"
$archive = Join-Path $stage "mesa3d-$version-release-msvc.7z"
$expectedArchive = '351FC8C8B695878FFB3EAA044B3EAD08672A48B1A045E3C3E3975811DF0F6695'
$dllHashes = @{
    'opengl32.dll' = '75B295FC80DF0EAA8F1264C28663F6932C8B72D36513EB092556608858B7F5E0'
    'libgallium_wgl.dll' = '53E49ADF87590D5B0CAB8553B1E98084812040CC1FCA8C487695B17B7AF3702E'
}
function Assert-TestOpenGlHash([string]$Path, [string]$Expected) {
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ine $Expected) { throw 'Test OpenGL checksum mismatch.' }
}
function Assert-TestOpenGlArchitecture([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $reader = [IO.BinaryReader]::new($stream)
    try {
        if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) { throw 'Invalid test OpenGL Windows binary.' }
        $stream.Position = 60
        $header = $reader.ReadInt32()
        if ($header -lt 64 -or $header -gt $stream.Length - 6) { throw 'Invalid test OpenGL PE header.' }
        $stream.Position = $header
        if ($reader.ReadUInt32() -ne 0x00004550 -or $reader.ReadUInt16() -ne 0x8664) { throw 'Test OpenGL requires x64 binaries.' }
    } finally { $reader.Dispose(); $stream.Dispose() }
}
if ($SelfTest) {
    $temp = [IO.Path]::GetTempFileName()
    try {
        [IO.File]::WriteAllBytes($temp, [byte[]]@(97, 98, 99))
        Assert-TestOpenGlHash $temp 'BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD'
        $rejected = $false
        try { Assert-TestOpenGlHash $temp ('0' * 64) } catch { $rejected = $true }
        if (-not $rejected) { throw 'Incorrect test OpenGL hash was accepted.' }
        Write-Host 'Test OpenGL checksum self-test passed.'
    } finally { Remove-Item -LiteralPath $temp }
    exit 0
}
[IO.Directory]::CreateDirectory($stage) | Out-Null
if (-not (Test-Path -LiteralPath $archive)) {
    $url = "https://github.com/pal1000/mesa-dist-win/releases/download/$version/mesa3d-$version-release-msvc.7z"
    & curl.exe --location --fail --silent --show-error --output $archive $url
    if ($LASTEXITCODE -ne 0) { throw 'Test OpenGL download failed.' }
}
Assert-TestOpenGlHash $archive $expectedArchive
$bin = Join-Path $stage 'bin'
[IO.Directory]::CreateDirectory($bin) | Out-Null
$sevenZip = Join-Path $env:ProgramFiles '7-Zip/7z.exe'
if (-not (Test-Path -LiteralPath $sevenZip)) { throw '7-Zip is required to extract the pinned test OpenGL distribution.' }
& $sevenZip e $archive 'x64/opengl32.dll' 'x64/libgallium_wgl.dll' "-o$bin" -y | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Test OpenGL extraction failed.' }
foreach ($name in $dllHashes.Keys) {
    $path = Join-Path $bin $name
    Assert-TestOpenGlHash $path $dllHashes[$name]
    Assert-TestOpenGlArchitecture $path
}
$env:GLUTIN_WGL_OPENGL_DLL = Join-Path $bin 'opengl32.dll'
$env:GALLIUM_DRIVER = 'llvmpipe'
$env:WGL_DISABLE_ERROR_DIALOGS = '1'
$env:PATH = "$bin;$env:PATH"
if ($env:GITHUB_ENV) {
    $values = "GLUTIN_WGL_OPENGL_DLL=$env:GLUTIN_WGL_OPENGL_DLL`nGALLIUM_DRIVER=llvmpipe`nWGL_DISABLE_ERROR_DIALOGS=1`n"
    [IO.File]::AppendAllText($env:GITHUB_ENV, $values, [Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText($env:GITHUB_PATH, "$bin`n", [Text.UTF8Encoding]::new($false))
}
Write-Host "Verified test-only Mesa $version x64 software OpenGL: $bin"
