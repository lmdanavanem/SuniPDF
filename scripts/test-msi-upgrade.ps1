# Real Windows Installer regression on a disposable, elevated Windows runner.
param(
    [Parameter(Mandatory)] [string]$MsiPath,
    [ValidateSet('0.8.0', '0.9.1', '0.10.0', '0.11.0', '0.12.0', '0.13.0', '0.14.0', '0.15.0', '0.16.0')] [string]$BaselineVersion = '0.8.0'
)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'MSI regression is restricted to disposable GitHub Actions runners.' }
$root = Split-Path -Parent $PSScriptRoot
$msi = (Resolve-Path -LiteralPath $MsiPath).Path
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { throw 'MSI regression requires an elevated disposable Windows runner.' }
$uninstallRoots = @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall', 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall')
function Get-SuniProducts {
    foreach ($registryRoot in $uninstallRoots) {
        Get-ItemProperty "$registryRoot\*" -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'SuniPDF' }
    }
}
if (@(Get-SuniProducts).Count) { throw 'An existing SuniPDF installation must not be touched by this test.' }
$menu = Join-Path ([Environment]::GetFolderPath('CommonPrograms')) 'SuniPDF'
if (Test-Path -LiteralPath $menu) { throw 'An existing SuniPDF Start Menu folder must not be touched.' }
$work = Join-Path $root ('target\msi-upgrade-' + $BaselineVersion + '-' + [guid]::NewGuid().ToString('N'))
$target = Join-Path $work 'application'
New-Item -ItemType Directory -Path $work | Out-Null
$baselines = @{
    '0.8.0' = @{ hash='3535AF575441F05DB2A718D069FA2007379915C9CED8229CB86E453A3804E9A4'; language='1036'; productCode='{89AB70C1-3F41-4981-93EE-F828176C4FA3}' }
    '0.9.1' = @{ hash='90F686D7ACEF7251F97D026F2340316D108CA06CC8A687F63D926FF3EAFD3AE5'; language='1033'; productCode='{3E48052E-E69B-4AD2-9761-41EEFEC9A859}' }
    '0.10.0' = @{ hash='5D603B4445B843CD0E42EB77348FF7C1FAD5F0A36ADFD3B7DF1E79BBE102DC73'; language='1033'; productCode='{F261F774-4BF2-4EC4-A23E-77D6B6A0AB4A}' }
    '0.11.0' = @{ hash='E99FE4416DA5E55086059884CCBE770EA1D9AB437D8E3BE26D49D696B5C9C4BD'; language='1033'; productCode='{2331BC3A-07F0-4AD0-BAF6-E953A103EB8B}' }
    '0.12.0' = @{ hash='390A20B7B5999635FDC6F2FE99ED4BD25DE53EAB4623B820FEC2F9AF04AA0C89'; language='1033'; productCode='{1B2F3E70-A482-4F8C-948E-9E9830EA2333}' }
    '0.13.0' = @{ hash='0727828749D235CCCD52BE6440C34F980F59FCABC7AB2B1027D2A77228D65C63'; language='1033'; productCode='{EAA2A957-4EE5-474D-874D-336DD1228D03}' }
    '0.14.0' = @{ hash='838DA744E862C840B4000678FC6E4757B3EFD85DBDB5910764178334EFFB5FBA'; language='1033'; productCode='{BDE7F7D8-FCB7-4A93-8364-0731ACC94D4E}' }
    '0.15.0' = @{ hash='119C4725324AF01FC1245D685604C9A0927773BB2D08E48C4FEA70AFF230CCE7'; language='1033'; productCode='{0782C5A5-D309-4F93-879A-8197CFB63084}' }
    '0.16.0' = @{ hash='5B4043B271C36C2CBBFBFD3FAA419ED96CB2D151CD32250C43C8DC121FB1328D'; language='1033'; productCode='{18C36B58-D424-487E-88BB-FEE6077A91B8}' }
}
$baseline = $baselines[$BaselineVersion]
$oldName = "SuniPDF-$BaselineVersion-win-x64.msi"
$old = Join-Path $work $oldName
& curl.exe --location --fail --silent --show-error --output $old "https://github.com/lmdanavanem/SuniPDF/releases/download/v$BaselineVersion/$oldName"
if ($LASTEXITCODE -ne 0 -or (Get-FileHash -LiteralPath $old).Hash -ne $baseline.hash) { throw 'Previous release download/hash failed.' }
$installer = New-Object -ComObject WindowsInstaller.Installer
function Read-Property([string]$Path, [string]$Name) {
    $database = $installer.OpenDatabase($Path, 0)
    $view = $database.OpenView("SELECT Value FROM Property WHERE Property = '$Name'")
    try {
        $null = $view.Execute()
        $record = $view.Fetch()
        if ($null -eq $record) { throw "MSI property is missing: $Name" }
        return [string]$record.StringData(1)
    } finally {
        $null = $view.Close()
        [Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
        [Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) | Out-Null
    }
}
$oldCode = Read-Property $old 'ProductCode'
$newCode = Read-Property $msi 'ProductCode'
$version = Read-Property $msi 'ProductVersion'
$oldVersion = Read-Property $old 'ProductVersion'
$oldLanguage = Read-Property $old 'ProductLanguage'
$newLanguage = Read-Property $msi 'ProductLanguage'
Write-Host "MSI metadata: old=$oldCode ($oldVersion, $oldLanguage), new=$newCode ($version, $newLanguage)"
if ($oldCode -eq $newCode -or $version -notmatch '^\d+\.\d+\.\d+$') { throw 'Upgrade requires distinct products and a valid version.' }
if ($oldCode -ne $baseline.productCode) { throw 'The baseline MSI ProductCode does not match the published metadata.' }
if ($oldVersion -ne $BaselineVersion -or [version]$version -le [version]$BaselineVersion) { throw 'The MSI versions do not match the selected baseline and a newer upgrade.' }
if ($oldLanguage -ne $baseline.language -or $newLanguage -ne '1033') { throw 'The baseline/current MSI languages do not match the published metadata.' }
if ((Read-Property $old 'UpgradeCode') -ne (Read-Property $msi 'UpgradeCode')) { throw 'The baseline/current MSI UpgradeCode differs.' }
$sourceHash = (Get-FileHash -LiteralPath $msi).Hash
function Invoke-Msi([string]$Package, [string]$Label, [switch]$Remove, [switch]$ExpectFailure) {
    $operation = if ($Remove) { '/x' } else { '/i' }
    $arguments = "$operation `"$Package`" /qn /norestart REBOOT=ReallySuppress /l*v `"$work\$Label.log`""
    if (-not $Remove) { $arguments += " INSTALLFOLDER=`"$target`"" }
    $process = Start-Process -FilePath "$env:SystemRoot\System32\msiexec.exe" -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru
    if ($ExpectFailure) {
        if ($process.ExitCode -ne 1603) { throw "$Label expected rollback/rejection (1603), got $($process.ExitCode)." }
    } elseif ($process.ExitCode -notin @(0, 3010)) { throw "$Label failed ($($process.ExitCode)); see $work\$Label.log." }
    Write-Host "$Label : $($process.ExitCode)"
}
function Assert-Version([string]$Expected, [string]$Code) {
    $products = @(Get-SuniProducts)
    if ($products.Count -ne 1 -or $products[0].PSChildName -ne $Code -or $products[0].DisplayVersion -ne $Expected) { throw "Registered product does not match $Expected." }
    $actual = & (Join-Path $target 'sunipdf-cli.exe') --version
    if ($LASTEXITCODE -ne 0 -or "$actual" -ne "sunipdf-cli $Expected") { throw "Installed CLI does not match $Expected : $actual" }
}
$profile = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'SuniPDF'
New-Item -ItemType Directory -Force -Path $profile | Out-Null
$sentinel = Join-Path $profile ('upgrade-regression-' + [guid]::NewGuid().ToString('N') + '.json')
[IO.File]::WriteAllText($sentinel, '{"unsaved_document":"must survive upgrade and uninstall"}')
$sentinelHash = (Get-FileHash -LiteralPath $sentinel).Hash
$document = Join-Path $target 'user-document.pdf'
$payloadNames = @('sunipdf.exe', 'sunipdf-cli.exe', 'pdfium.dll', 'licenses\tesseract.exe', 'licenses\eng.traineddata', 'licenses\fra.traineddata')
try {
    Invoke-Msi $old "install-$BaselineVersion"
    Assert-Version $BaselineVersion $oldCode
    Copy-Item -LiteralPath (Join-Path $root 'crates\sunipdf-core\tests\fixtures\two-pages.pdf') -Destination $document
    $documentHash = (Get-FileHash -LiteralPath $document).Hash
    $baselineFiles = @{}
    foreach ($name in $payloadNames) {
        $file = Join-Path $target $name
        $baselineFiles[$name] = if (Test-Path -LiteralPath $file) { (Get-FileHash -LiteralPath $file).Hash } else { $null }
    }
    # Fail after files are copied, inside the actual major-upgrade transaction.
    $broken = Join-Path $work 'intentional-rollback.msi'
    Copy-Item -LiteralPath $msi -Destination $broken
    $database = $installer.OpenDatabase($broken, 1)
    try {
        $view = $database.OpenView("SELECT Name FROM _Tables WHERE Name = 'CustomAction'")
        try { $view.Execute(); $hasCustomActions = $null -ne $view.Fetch() }
        finally { $view.Close(); [Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null }
        $sqlStatements = @()
        if (-not $hasCustomActions) { $sqlStatements += 'CREATE TABLE CustomAction (Action CHAR(72) NOT NULL, Type SHORT NOT NULL, Source CHAR(72), Target CHAR(255) LOCALIZABLE PRIMARY KEY Action)' }
        $view = $database.OpenView("SELECT Action FROM InstallExecuteSequence WHERE Action = 'InstallExecute'")
        try { $view.Execute(); $hasInstallExecute = $null -ne $view.Fetch() }
        finally { $view.Close(); [Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null }
        $sqlStatements += if ($hasInstallExecute) { "UPDATE InstallExecuteSequence SET Sequence = 6500 WHERE Action = 'InstallExecute'" } else { "INSERT INTO InstallExecuteSequence (Action, Sequence) VALUES ('InstallExecute', 6500)" }
        $sqlStatements += @("INSERT INTO CustomAction (Action, Type, Target) VALUES ('SuniPDFRegressionFailure', 19, 'Intentional upgrade regression rollback')", "INSERT INTO InstallExecuteSequence (Action, Condition, Sequence) VALUES ('SuniPDFRegressionFailure', 'NOT Installed', 6501)")
        foreach ($sql in $sqlStatements) {
            $view = $database.OpenView($sql)
            try { $view.Execute() } finally { $view.Close(); [Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null }
        }
        $database.Commit()
    } finally { [Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) | Out-Null }
    Invoke-Msi $broken 'failed-upgrade-rollback' -ExpectFailure
    $rollbackLog = [IO.File]::ReadAllText((Join-Path $work 'failed-upgrade-rollback.log'))
    if ($rollbackLog -notmatch '(?s)Action ended[^\r\n]*: InstallExecute\. Return value 1\..*Action ended[^\r\n]*: SuniPDFRegressionFailure\. Return value 3\.') { throw 'The injected failure did not occur after the installation script executed.' }
    Assert-Version $BaselineVersion $oldCode
    foreach ($name in $payloadNames) {
        $file = Join-Path $target $name
        if ($baselineFiles[$name]) {
            if (-not (Test-Path -LiteralPath $file) -or (Get-FileHash -LiteralPath $file).Hash -ne $baselineFiles[$name]) { throw "Rollback did not restore the baseline payload: $name" }
        } elseif (Test-Path -LiteralPath $file) { throw "Rollback left a newly installed payload: $name" }
    }
    if ((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash -or (Get-FileHash -LiteralPath $document).Hash -ne $documentHash) { throw 'Rollback modified user data.' }
    Invoke-Msi $msi 'upgrade-current'
    Assert-Version $version $newCode
    $portable = Join-Path $root "dist\SuniPDF-$version-win-x64-portable"
    foreach ($name in $payloadNames) {
        if ((Get-FileHash -LiteralPath (Join-Path $target $name)).Hash -ne (Get-FileHash -LiteralPath (Join-Path $portable $name)).Hash) { throw "Installed payload differs: $name" }
    }
    Invoke-Msi $old 'downgrade-rejected' -ExpectFailure
    Assert-Version $version $newCode
    if ((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash -or (Get-FileHash -LiteralPath $document).Hash -ne $documentHash) { throw 'Upgrade modified user data.' }
    Invoke-Msi $newCode 'uninstall-current' -Remove
    if (@(Get-SuniProducts).Count -or (Test-Path -LiteralPath (Join-Path $target 'sunipdf.exe'))) { throw 'Uninstall left an installed product.' }
    if ((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash -or (Get-FileHash -LiteralPath $document).Hash -ne $documentHash) { throw 'Uninstall modified user data.' }
    if ((Get-FileHash -LiteralPath $msi).Hash -ne $sourceHash) { throw 'Regression modified the release MSI.' }
    Write-Host "OK: real MSI install, transactional rollback, $BaselineVersion-to-$version upgrade ($oldLanguage-to-$newLanguage), downgrade rejection, payload hashes, user-data preservation and uninstall."
} finally {
    foreach ($product in @(Get-SuniProducts)) {
        if ($product.PSChildName -in @($oldCode, $newCode)) { Invoke-Msi $product.PSChildName 'cleanup' -Remove }
    }
    Remove-Item -LiteralPath $sentinel -Force
    [Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer) | Out-Null
}
