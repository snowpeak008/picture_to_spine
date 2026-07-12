[CmdletBinding()]
param([string]$PackagePath='dist\FlashToSpine-Core')

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$package=[IO.Path]::GetFullPath((Join-Path $root $PackagePath))
$dist=[IO.Path]::GetFullPath((Join-Path $root 'dist')).TrimEnd('\')+'\'
if(-not $package.StartsWith($dist,[StringComparison]::OrdinalIgnoreCase)){throw 'Package path must remain below dist.'}
$rootLauncher=Join-Path $root 'FlashToSpineLauncher.exe'
. (Join-Path $PSScriptRoot 'core-package-validation.ps1')
$validated=Assert-F2sPackageContents $package $root $rootLauncher $true

$result=[ordered]@{
    schemaVersion='1.0.0'
    status='PASS'
    packagePath='dist/FlashToSpine-Core'
    executableSha256=$validated.executableSha256
    buildInputSha256=$validated.buildInputSha256
    rootEntrypoint='FlashToSpineLauncher.exe'
    smoke='PASS_WITHOUT_WEBVIEW2_PROBE'
    signatureStatus='NOT_RUN_EXTERNAL'
}
$result|ConvertTo-Json -Depth 5
