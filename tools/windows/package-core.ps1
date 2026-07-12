[CmdletBinding()]
param([switch]$SkipBuild)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$target='x86_64-pc-windows-msvc'
$dist=[IO.Path]::GetFullPath((Join-Path $root 'dist'))
$stage=[IO.Path]::GetFullPath((Join-Path $dist '.FlashToSpine-Core.staging'))
$final=[IO.Path]::GetFullPath((Join-Path $dist 'FlashToSpine-Core'))
$backup=[IO.Path]::GetFullPath((Join-Path $dist '.FlashToSpine-Core.backup'))
$rootLauncher=[IO.Path]::GetFullPath((Join-Path $root 'FlashToSpineLauncher.exe'))
$launcherTemp=$rootLauncher+'.f2s-tmp'
$launcherBackup=$rootLauncher+'.f2s-backup'
$utf8=[Text.UTF8Encoding]::new($false)
. (Join-Path $PSScriptRoot 'core-package-validation.ps1')
if(-not $SkipBuild){
    & powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'build-core.ps1')
    if($LASTEXITCODE -ne 0){throw "Core build failed with exit code $LASTEXITCODE"}
}
$packageMutex=[Threading.Mutex]::new($false,'Local\FlashToSpine-Core-BuildPackage-v1')
$mutexAcquired=$false
try{$mutexAcquired=$packageMutex.WaitOne(0)}catch [Threading.AbandonedMutexException]{$mutexAcquired=$true}
if(-not $mutexAcquired){$packageMutex.Dispose();throw 'Another FlashToSpine package operation is already running.'}

try{
function Assert-ChildPath([string]$Path,[string]$Parent){
    $parentFull=[IO.Path]::GetFullPath($Parent).TrimEnd('\')+'\'
    $pathFull=[IO.Path]::GetFullPath($Path)
    if(-not $pathFull.StartsWith($parentFull,[StringComparison]::OrdinalIgnoreCase)){
        throw "Unsafe package path: $pathFull"
    }
}
foreach($path in @($stage,$final,$backup)){Assert-ChildPath $path $dist}

# Recover only deterministic package/launcher backups left by an interrupted prior commit.
function Get-Sha256OrNull([string]$Path){
    if(Test-Path -LiteralPath $Path -PathType Leaf){
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    }else{$null}
}
if((Test-Path -LiteralPath $final) -and (Test-Path -LiteralPath $backup)){
    $finalExeHash=Get-Sha256OrNull (Join-Path $final 'FlashToSpine.exe')
    $launcherHash=Get-Sha256OrNull $rootLauncher
    if($null -ne $finalExeHash -and $launcherHash -eq $finalExeHash){
        # The final package and launcher pair committed; only cleanup was interrupted.
        Remove-Item -LiteralPath $backup -Recurse -Force
    }else{
        # The pair did not commit. Restore the previous package before any new build work.
        Remove-Item -LiteralPath $final -Recurse -Force
        Move-Item -LiteralPath $backup -Destination $final
    }
}elseif(-not(Test-Path -LiteralPath $final) -and (Test-Path -LiteralPath $backup)){
    Move-Item -LiteralPath $backup -Destination $final
}
if(Test-Path -LiteralPath $final){
    $restoredExe=Join-Path $final 'FlashToSpine.exe'
    $restoredHash=Get-Sha256OrNull $restoredExe
    if($null -ne $restoredHash -and (Get-Sha256OrNull $rootLauncher) -ne $restoredHash){
        if(Test-Path -LiteralPath $rootLauncher){Remove-Item -LiteralPath $rootLauncher -Force}
        if((Get-Sha256OrNull $launcherBackup) -eq $restoredHash){
            Move-Item -LiteralPath $launcherBackup -Destination $rootLauncher
        }else{
            Copy-Item -LiteralPath $restoredExe -Destination $launcherTemp
            Move-Item -LiteralPath $launcherTemp -Destination $rootLauncher
        }
    }
}
if(Test-Path -LiteralPath $stage){Remove-Item -LiteralPath $stage -Recurse -Force}
if(Test-Path -LiteralPath $launcherBackup){Remove-Item -LiteralPath $launcherBackup -Force}
if(Test-Path -LiteralPath $launcherTemp){Remove-Item -LiteralPath $launcherTemp -Force}

$sourceExe=Join-Path $root "target\$target\release\FlashToSpine.exe"
$sourceBinding=Join-Path $root "target\$target\release\FlashToSpine.build-binding.json"
if(-not(Test-Path -LiteralPath $sourceExe -PathType Leaf)){throw 'Release EXE is missing. Run npm run build:core first.'}
if(-not(Test-Path -LiteralPath $sourceBinding -PathType Leaf)){
    throw 'Release build binding is missing. Run npm run build:core; stale binaries cannot be packaged.'
}

New-Item -ItemType Directory -Path $stage|Out-Null
Copy-Item -LiteralPath $sourceExe -Destination (Join-Path $stage 'FlashToSpine.exe')
Copy-Item -LiteralPath $sourceBinding -Destination (Join-Path $stage 'build-binding.json')
[IO.File]::WriteAllText((Join-Path $stage 'README.txt'),(Get-F2sExpectedCoreReadme),$utf8)

$stagedExe=Join-Path $stage 'FlashToSpine.exe'
$stagedBinding=Join-Path $stage 'build-binding.json'
Invoke-F2sJsonContract (Join-Path $PSScriptRoot 'core-build-binding.schema.json') $stagedBinding
$buildAttestation=Get-Content -LiteralPath $stagedBinding -Raw -Encoding UTF8|ConvertFrom-Json
$currentSource=Get-F2sSourceInputBinding $root
$currentUi=Get-F2sUiBundleBinding $root
$currentToolchain=Get-F2sToolchainFingerprintSha256
$currentCombined=Get-F2sCombinedBuildInputSha256 $currentSource $currentUi $currentToolchain
Assert-F2sSourceInputBindingEqual $buildAttestation $currentSource 'Release EXE was built from different source inputs'
Assert-F2sUiBundleBindingEqual $buildAttestation $currentUi 'Release EXE was built with a different UI bundle'
if($buildAttestation.toolchainFingerprintSha256 -ne $currentToolchain){throw 'Release EXE was built with a different toolchain fingerprint.'}
if($buildAttestation.buildInputSha256 -ne $currentCombined){throw 'Release build receipt combined input digest is invalid.'}
if($buildAttestation.executableSha256 -ne (Get-FileHash -LiteralPath $stagedExe -Algorithm SHA256).Hash.ToLowerInvariant()){
    throw 'Staged Release EXE does not match its build-time receipt.'
}

$packageJson=Get-Content -LiteralPath (Join-Path $root 'package.json') -Raw -Encoding UTF8|ConvertFrom-Json
$buildBindingSha256=(Get-FileHash -LiteralPath $stagedBinding -Algorithm SHA256).Hash.ToLowerInvariant()
$files=@('FlashToSpine.exe','README.txt','build-binding.json')|ForEach-Object{
    $path=Join-Path $stage $_
    [ordered]@{path=$_;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant();bytes=(Get-Item -LiteralPath $path).Length}
}
$manifest=[ordered]@{
    schemaVersion='1.0.0'
    product='FlashToSpine'
    version=[string]$packageJson.version
    packageKind='windows-portable-core-internal'
    target=$target
    entrypoint='FlashToSpine.exe'
    deterministicInputs=[ordered]@{
        cargoLocked=$true
        cargoOffline=$true
        nodeLock='package-lock.json'
        cargoLock='Cargo.lock'
        buildInputSha256=$buildAttestation.buildInputSha256
        toolchainFingerprintSha256=$buildAttestation.toolchainFingerprintSha256
        sourceTreeSha256=$buildAttestation.sourceTreeSha256
        sourceFileCount=$buildAttestation.sourceFileCount
        cargoLockSha256=$buildAttestation.cargoLockSha256
        nodeLockSha256=$buildAttestation.nodeLockSha256
        uiBundleSha256=$buildAttestation.uiBundleSha256
        uiBundleFileCount=$buildAttestation.uiBundleFileCount
        buildBindingSha256=$buildBindingSha256
    }
    prerequisites=[ordered]@{webView2='SYSTEM_EVERGREEN_REQUIRED_UNVERIFIED'}
    capabilities=[ordered]@{
        coreBinary='BUILT_UNVERIFIED_CLEAN_VM'
        uiEmbedded='BUILT'
        spineEditor='EXTERNAL_NOT_INCLUDED'
        appContainerWorker='NOT_INCLUDED_UNVERIFIED'
        codeSignature='NOT_RUN_EXTERNAL'
    }
    security=[ordered]@{networkInstaller=$false;elevationRequired=$false;downloadsDependencies=$false}
    files=$files
}
$manifestPath=Join-Path $stage 'package-manifest.json'
[IO.File]::WriteAllText($manifestPath,(($manifest|ConvertTo-Json -Depth 10)+[Environment]::NewLine),$utf8)
Invoke-F2sJsonContract (Join-Path $PSScriptRoot 'core-package-manifest.schema.json') $manifestPath
$checksumFiles=@('FlashToSpine.exe','README.txt','build-binding.json','package-manifest.json')
$checksums=($checksumFiles|ForEach-Object{"$((Get-FileHash -LiteralPath (Join-Path $stage $_) -Algorithm SHA256).Hash.ToLowerInvariant())  $_"})-join[Environment]::NewLine
[IO.File]::WriteAllText((Join-Path $stage 'checksums.sha256'),$checksums+[Environment]::NewLine,$utf8)

$preCommit=Assert-F2sPackageContents $stage $root $rootLauncher $false
$hadPrevious=Test-Path -LiteralPath $final
$hadLauncher=Test-Path -LiteralPath $rootLauncher -PathType Leaf
$previousBackedUp=$false
$packageCommitted=$false
$launcherBackedUp=$false
$newLauncherInstalled=$false
try{
    if($hadPrevious){Move-Item -LiteralPath $final -Destination $backup; $previousBackedUp=$true}
    Move-Item -LiteralPath $stage -Destination $final
    $packageCommitted=$true
    $postCommit=Assert-F2sPackageContents $final $root $rootLauncher $false
    if($postCommit.executableSha256 -ne $preCommit.executableSha256){throw 'Package changed during final directory commit.'}

    if($hadLauncher){Move-Item -LiteralPath $rootLauncher -Destination $launcherBackup; $launcherBackedUp=$true}
    Copy-Item -LiteralPath (Join-Path $final 'FlashToSpine.exe') -Destination $launcherTemp
    Move-Item -LiteralPath $launcherTemp -Destination $rootLauncher
    $newLauncherInstalled=$true
    $finalValidation=Assert-F2sPackageContents $final $root $rootLauncher $true
}catch{
    if(Test-Path -LiteralPath $launcherTemp){Remove-Item -LiteralPath $launcherTemp -Force}
    if($newLauncherInstalled -and (Test-Path -LiteralPath $rootLauncher)){Remove-Item -LiteralPath $rootLauncher -Force}
    if($launcherBackedUp -and (Test-Path -LiteralPath $launcherBackup)){Move-Item -LiteralPath $launcherBackup -Destination $rootLauncher}
    if($packageCommitted -and (Test-Path -LiteralPath $final)){Remove-Item -LiteralPath $final -Recurse -Force}
    if($previousBackedUp -and (Test-Path -LiteralPath $backup)){Move-Item -LiteralPath $backup -Destination $final}
    throw
}
$cleanupPending=$false
try{if(Test-Path -LiteralPath $backup){Remove-Item -LiteralPath $backup -Recurse -Force}}catch{$cleanupPending=$true}
try{if(Test-Path -LiteralPath $launcherBackup){Remove-Item -LiteralPath $launcherBackup -Force}}catch{$cleanupPending=$true}

$result=[ordered]@{
    schemaVersion='1.0.0'
    status='PACKAGED_INTERNAL_UNSIGNED'
    packagePath='dist/FlashToSpine-Core'
    rootEntrypoint='FlashToSpineLauncher.exe'
    executableSha256=$finalValidation.executableSha256
    buildInputSha256=$finalValidation.buildInputSha256
    manifestSha256=(Get-FileHash -LiteralPath (Join-Path $final 'package-manifest.json') -Algorithm SHA256).Hash.ToLowerInvariant()
    signatureStatus='NOT_RUN_EXTERNAL'
    releaseAuthorized=$false
    webView2Status='SYSTEM_PREREQUISITE_UNVERIFIED'
    appContainerStatus='NOT_INCLUDED_UNVERIFIED'
    cleanupStatus=if($cleanupPending){'DEFERRED_RECOVERABLE'}else{'COMPLETE'}
}
}finally{
    if($mutexAcquired){
        try{$packageMutex.ReleaseMutex()}catch{}
    }
    $packageMutex.Dispose()
}
$result|ConvertTo-Json -Depth 5
