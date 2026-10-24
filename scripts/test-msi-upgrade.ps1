# Real Windows Installer regression on a disposable, elevated Windows runner.
param(
    [Parameter(Mandatory)] [string]$MsiPath,
    [ValidateSet('0.8.0', '0.9.1', '0.10.0', '0.11.0', '0.12.0', '0.13.0', '0.14.0', '0.15.0', '0.16.0', '0.17.0', '0.18.0', '0.19.0', '0.20.0', '0.21.0', '0.21.1', '0.30.0', '0.30.1', '0.31.0', '0.32.0', '0.33.0', '0.34.0', '0.35.0', '0.36.0', '0.37.0', '0.38.0', '0.39.0', '0.40.0', '0.41.0', '0.42.0', '0.43.0', '0.44.0', '0.45.0', '0.46.0', '0.47.0', '0.48.0', '0.49.0', '0.50.0', '0.51.0', '0.60.0', '0.61.0', '0.62.0', '0.63.0', '0.64.0', '0.67.0')] [string]$BaselineVersion = '0.8.0'
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
    '0.17.0' = @{ hash='C59DE003132EFC077B6D5F0C70B7FC8DAE06616D6F4E4B12DDD45BCB479A603E'; language='1033'; productCode='{33E9519A-CCCF-4FBE-876F-430130D5503B}' }
    '0.18.0' = @{ hash='D3DABCB44438E4E005D33A5D4544ACD6C3514DE10BF7BB07B745123E4CD7DDC4'; language='1033'; productCode='{2C188A13-8DBB-475A-B48D-78BCC8D058EA}' }
    '0.19.0' = @{ hash='E1CD1E6894B09A107238F96DFEFB5F7075E94D31DCC4FDFE3CCDF01100EE0EEE'; language='1033'; productCode='{AA82CB16-BEC5-40FD-83D3-C86852E44676}' }
    '0.20.0' = @{ hash='9D87F80B559EA76DBCF92F6EE8867994B72C82AE8AFC690EE6425ECAB6551B75'; language='1033'; productCode='{90AA23B4-8929-4B93-A77B-2995BD483DC3}' }
    '0.21.0' = @{ hash='4FEAA78595E289D381DAEBEFE0E036E36142A0CCDD54DCC31F34C36730BD6E85'; language='1033'; productCode='{BFFBDEB1-7C9A-4D74-B97C-5C9E69684FE9}' }
    '0.21.1' = @{ hash='67AA35BD8FE0C6F36715F9EBEDCF2ADE079361BF46604595AEEABFF05874B806'; language='1033'; productCode='{BF9778FF-18B3-4915-9F24-39AC0EED8AF0}' }
    '0.30.0' = @{ hash='0EADBC78C66624F0E8BACE5BDA3B2F3A5C8D85A93BD851132A4B0CA53C028C4B'; language='1033'; productCode='{7F41EB46-D35F-4682-9580-6D9055A1E315}' }
    '0.30.1' = @{ hash='69EDBF1FD663CA3D207865E57D21982D4EA80A44F6A9B6FAFB33E866A688DA87'; language='1033'; productCode='{C10A12A3-E267-4D70-B016-F1908A023E43}' }
    '0.31.0' = @{ hash='299DAD159D721F53E1049582B8A5C14288F85C57B2BC1B3FCB529CFCA547383E'; language='1033'; productCode='{AB3CD26F-9E7A-4F21-97EF-3174861D0241}' }
    '0.32.0' = @{ hash='82E075EE44A4EF639424984A66277A2B8E85ED7E7C2DBD0EECBCA4B9BD3B96CD'; language='1033'; productCode='{6AF0C5DC-304B-4263-9114-1E4B7788B0EB}' }
    '0.33.0' = @{ hash='B3E8AD799D49A061C1C87AC3D3A2FB30FA9FC8DB1882D524668441D00B79285D'; language='1033'; productCode='{3B806FB2-422A-4DB7-A51F-A7A19252EB26}' }
    '0.34.0' = @{ hash='58A2CC17794A0130BCED6276257767B2C820C9551F4F53D68BC526C24DAB7482'; language='1033'; productCode='{F44751B4-16CA-4587-A115-F6B54FF02933}' }
    '0.35.0' = @{ hash='F9F9C2277210D8B4069B609E9D6E5DBD1CC967054032400EB04FFC39D65427B8'; language='1033'; productCode='{2E9A3767-E5A9-41E1-B352-91D5B1F996CA}' }
    '0.36.0' = @{ hash='A7DF40B4D6A3C8BD3F80D694850E4CF48D9B7A0B951A4C09A0A65F8411C5F63E'; language='1033'; productCode='{095B3028-A59F-45C7-AB6B-8EC306DADF2E}' }
    '0.37.0' = @{ hash='092A80CCFC2D0BF22864152D22D341B4CA06ABC703B8B30960FE5A158D0924DB'; language='1033'; productCode='{7DC095F2-331B-4450-9672-E080E891465F}' }
    '0.38.0' = @{ hash='A7BDB4FAAD6FE44BB54E0F3E786CCB4ECC5249FE3B66713EC694F1A42F86DAC5'; language='1033'; productCode='{6C3E00F1-203E-4990-9293-D689AADB54E0}' }
    '0.39.0' = @{ hash='91B5C2D64D67B8AC452D3DA0BB50FF20D08D85613406D1726AD32E8E62D88954'; language='1033'; productCode='{105334F7-9811-4FF7-B3B2-5CD8D9C2170E}' }
    '0.40.0' = @{ hash='B67F7FD0039DD7EFBD7A54E866E04FD1E6D278F42B10191798CC812715F4A0DB'; language='1033'; productCode='{6536F66C-0664-4276-AD73-1560BB88D531}' }
    '0.41.0' = @{ hash='4604214EACCCD79A6DBE85DC988F11DE2F374E3A2EB92E5341D910252C8A1EDD'; language='1033'; productCode='{DA9DB3CC-9CF2-4947-8B9A-00A59E793CC8}' }
    '0.42.0' = @{ hash='CE685BB4807B438928834A7F5E37D12ED2DAA9273D74AA0645E9B6C50C734313'; language='1033'; productCode='{E2D888C2-8C8A-4439-88FB-9FA6735E18E3}' }
    '0.43.0' = @{ hash='26B234F6A400D1B02772CE8CE9068CA1A5A09ADFC2EDF416FD7BA2FFC32035C4'; language='1033'; productCode='{3DA584C1-771B-41EB-B5D1-588C898F2384}' }
    '0.44.0' = @{ hash='7F063E0F22408B650A7CC2AEA0F3B76DA8AE9E7E8653D3453B85A4253FD28CA5'; language='1033'; productCode='{25D48AAA-86BC-4C43-818A-5584D856255C}' }
    '0.45.0' = @{ hash='911D0A30D2E98CD99B646424721B9B97C827DA545BF42DF238766168184B2534'; language='1033'; productCode='{C2464472-90C6-49E6-8E25-800ACE6CE841}' }
    '0.46.0' = @{ hash='1A41AD4823EA8048C23C33B5419BC8023DBB192E67DC6F822A5FBC2D0E86FBB6'; language='1033'; productCode='{95438867-25EF-4A61-A2F3-6C04358FB0D1}' }
    '0.47.0' = @{ hash='8E1B329815120B4A14B1691C6D6A8EEC9AA930176694A40D0CE64DF4E66E02A1'; language='1033'; productCode='{BFC05EA8-31C3-4097-9807-0FE106F61E65}' }
    '0.48.0' = @{ hash='1E05715982DDA4BEFBAE931DF3F75303D308AB4F915D61D241108F51EA793F9F'; language='1033'; productCode='{777CE116-0687-4AFF-B549-19F728CBA552}' }
    '0.49.0' = @{ hash='451ED450F8B9190A2B9C8D8894888497A8C18088245389537FF4EB57B3269FFF'; language='1033'; productCode='{B964C043-DFCC-4869-BC20-2EBA5C67BCFA}' }
    '0.50.0' = @{ hash='911530221C107CB2295E44F5F4C43172C85F5FCED3E8F8AC619888DE304F4E1B'; language='1033'; productCode='{7CB85285-5FDE-42FC-B05C-8191C870E0B0}' }
    '0.51.0' = @{ hash='B8DCC8050A6150EF95BA5C2B3955EA851768BFD42B38238D95B158ED5A469553'; language='1033'; productCode='{5947080C-425F-4E55-9D79-DB903B4FC844}' }
    '0.60.0' = @{ hash='7BC83B5CAE907B178DDF09422FD2A301BBBEBDD2BD09898B47CF2F3248190650'; language='1033'; productCode='{99C3CDF4-D9A1-4B9F-A82E-FAF7DE2DA893}' }
    '0.61.0' = @{ hash='FBAE1ED7DD913617F0F0DBED3BA3A2DE48F1BC4A0EE57D254737FA80A4948139'; language='1033'; productCode='{958B7D7E-AB5F-4EF5-AD23-81194D4C8BC2}' }
    '0.62.0' = @{ hash='F3C921528C4DCC2B6459E7A3CE073868DA0D42E8B2AE72BCF8FFED356601CDB2'; language='1033'; productCode='{1A099BDD-D724-403E-A137-5994010E00FB}' }
    '0.63.0' = @{ hash='81FF4D71778663B2CEEF4930BFA5C9C1B79FB673E8F73C3A640CA637BB21830F'; language='1033'; productCode='{36B4D600-48C3-4D3A-ADA9-8C3460A69F58}' }
    '0.64.0' = @{ hash='D60C0C4EB684267E46F6B4EE83E95969F505FAA79EE6C92E0EC079C70DA7A0A3'; language='1033'; productCode='{4DB49977-D93C-4B1F-AC70-6E2EF121228E}' }
    '0.67.0' = @{ hash='488BCA66581D4DA047D0998CFCB0946CDDB3D3D257187F7219A25AE3B76B8B83'; language='1033'; productCode='{980584EA-2247-43A6-848C-F0F1B13AA8F8}' }
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
