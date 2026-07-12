[CmdletBinding()]
param()

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$target='x86_64-pc-windows-msvc'
$utf8=[Text.UTF8Encoding]::new($false)
. (Join-Path $PSScriptRoot 'build-input-binding.ps1')
if(-not(Test-Path -LiteralPath (Join-Path $root 'Cargo.lock') -PathType Leaf)){throw 'Cargo.lock is required for the Core build.'}
Assert-F2sDeterministicCargoEnvironment $root
Assert-F2sDeterministicNodeEnvironment $root

$buildMutex=[Threading.Mutex]::new($false,'Local\FlashToSpine-Core-BuildPackage-v1')
$mutexAcquired=$false
try{$mutexAcquired=$buildMutex.WaitOne(0)}catch [Threading.AbandonedMutexException]{$mutexAcquired=$true}
if(-not $mutexAcquired){$buildMutex.Dispose();throw 'Another FlashToSpine build/package operation is already running.'}

$snapshotBase=[IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) 'FlashToSpine-build-snapshots'))
$snapshot=[IO.Path]::GetFullPath((Join-Path $snapshotBase ([Guid]::NewGuid().ToString('N'))))
$rootUi=Join-Path $root 'apps\desktop-ui'
$rootUiDist=Join-Path $rootUi 'dist'
$uiStage=Join-Path $rootUi ('.dist-build-staging-'+[Guid]::NewGuid().ToString('N'))
$uiBackup=Join-Path $rootUi '.dist-build-backup'
$releaseDir=Join-Path $root "target\$target\release"
$destinationExe=Join-Path $releaseDir 'FlashToSpine.exe'
$destinationBinding=Join-Path $releaseDir 'FlashToSpine.build-binding.json'
$exeBackup=$destinationExe+'.f2s-backup'
$bindingBackup=$destinationBinding+'.f2s-backup'
$transactionMarker=Join-Path $releaseDir '.f2s-build-transaction.json'
$newToken=[Guid]::NewGuid().ToString('N')
$exeNew=$destinationExe+'.f2s-new-'+$newToken
$bindingNew=$destinationBinding+'.f2s-new-'+$newToken
$result=$null

function Copy-SnapshotItem([string]$Relative){
    $source=Join-Path $root $Relative;$destination=Join-Path $snapshot $Relative
    if(-not(Test-Path -LiteralPath $source)){throw "Snapshot source is missing: $Relative"}
    $parent=Split-Path -Parent $destination
    if(-not(Test-Path -LiteralPath $parent)){New-Item -ItemType Directory -Path $parent -Force|Out-Null}
    Copy-Item -LiteralPath $source -Destination $destination -Recurse -Force
}

function Assert-BuiltExeBinding([string]$Exe,[string]$ExpectedBuildInput){
    $smokePath=Join-Path ([IO.Path]::GetTempPath()) ("f2s-build-smoke-$([Guid]::NewGuid().ToString('N')).json")
    try{
        $process=Start-Process -FilePath $Exe -ArgumentList @('--smoke',('"{0}"' -f $smokePath)) -Wait -PassThru -WindowStyle Hidden
        if($process.ExitCode -ne 0){throw 'Built EXE smoke rejected the embedded build input.'}
        $smoke=Get-Content -LiteralPath $smokePath -Raw -Encoding UTF8|ConvertFrom-Json
        if($smoke.status -ne 'PASS' -or $smoke.buildInputSha256 -ne $ExpectedBuildInput){throw 'Built EXE embedded build input mismatch.'}
    }finally{
        if(Test-Path -LiteralPath $smokePath){Remove-Item -LiteralPath $smokePath -Force}
    }
}

function Restore-InterruptedBuild(){
    if(Test-Path -LiteralPath $transactionMarker -PathType Leaf){
        $transaction=Get-Content -LiteralPath $transactionMarker -Raw -Encoding UTF8|ConvertFrom-Json
        if($transaction.schemaVersion -ne 'f2s-build-transaction/1.0.0'){throw 'Build transaction recovery marker is invalid.'}
        foreach($item in @(
            @{destination=$destinationExe;backup=$exeBackup;had=[bool]$transaction.hadExe},
            @{destination=$destinationBinding;backup=$bindingBackup;had=[bool]$transaction.hadBinding}
        )){
            if($item.had){
                if(Test-Path -LiteralPath $item.backup -PathType Leaf){
                    if(Test-Path -LiteralPath $item.destination){Remove-Item -LiteralPath $item.destination -Force}
                    Move-Item -LiteralPath $item.backup -Destination $item.destination
                }elseif(-not(Test-Path -LiteralPath $item.destination -PathType Leaf)){throw 'Build transaction backup and original are both missing.'}
            }else{
                if(Test-Path -LiteralPath $item.destination){Remove-Item -LiteralPath $item.destination -Force}
                if(Test-Path -LiteralPath $item.backup){Remove-Item -LiteralPath $item.backup -Force}
            }
        }
        if([bool]$transaction.hadUi){
            if(Test-Path -LiteralPath $uiBackup -PathType Container){
                if(Test-Path -LiteralPath $rootUiDist){Remove-Item -LiteralPath $rootUiDist -Recurse -Force}
                Move-Item -LiteralPath $uiBackup -Destination $rootUiDist
            }elseif(-not(Test-Path -LiteralPath $rootUiDist -PathType Container)){throw 'UI build transaction backup and original are both missing.'}
        }else{
            if(Test-Path -LiteralPath $rootUiDist){Remove-Item -LiteralPath $rootUiDist -Recurse -Force}
            if(Test-Path -LiteralPath $uiBackup){Remove-Item -LiteralPath $uiBackup -Recurse -Force}
        }
        Remove-Item -LiteralPath $transactionMarker -Force
    }else{
        foreach($item in @(@{destination=$destinationExe;backup=$exeBackup},@{destination=$destinationBinding;backup=$bindingBackup})){
            if(Test-Path -LiteralPath $item.backup){
                if(Test-Path -LiteralPath $item.destination){Remove-Item -LiteralPath $item.backup -Force}else{Move-Item -LiteralPath $item.backup -Destination $item.destination}
            }
        }
        if(Test-Path -LiteralPath $uiBackup){
            if(Test-Path -LiteralPath $rootUiDist){Remove-Item -LiteralPath $uiBackup -Recurse -Force}else{Move-Item -LiteralPath $uiBackup -Destination $rootUiDist}
        }
    }
    if(Test-Path -LiteralPath $releaseDir){
        foreach($stale in Get-ChildItem -LiteralPath $releaseDir -File -Force | Where-Object {$_.Name -like '*.f2s-new-*'}){Remove-Item -LiteralPath $stale.FullName -Force}
    }
    foreach($stale in Get-ChildItem -LiteralPath $rootUi -Directory -Force | Where-Object {$_.Name -like '.dist-build-staging-*'}){Remove-Item -LiteralPath $stale.FullName -Recurse -Force}
}

try{
    New-Item -ItemType Directory -Path $releaseDir -Force|Out-Null
    Restore-InterruptedBuild
    $rootSourceBefore=Get-F2sSourceInputBinding $root
    New-Item -ItemType Directory -Path $snapshot -Force|Out-Null
    foreach($relative in @(
        'Cargo.toml','Cargo.lock','rust-toolchain.toml','.node-version','package.json','package-lock.json',
        'crates','apps\desktop\src-tauri','apps\desktop-ui\src','apps\desktop-ui\package.json',
        'apps\desktop-ui\tsconfig.json','schemas','fixtures\m00\spine42-probe','tools\frontend',
        'tools\windows\build-core.ps1','tools\windows\build-input-binding.ps1'
    )){Copy-SnapshotItem $relative}
    if(Test-Path -LiteralPath (Join-Path $root '.cargo')){Copy-SnapshotItem '.cargo'}

    $rootSourceAfterCopy=Get-F2sSourceInputBinding $root
    $snapshotSource=Get-F2sSourceInputBinding $snapshot
    Assert-F2sSourceInputBindingEqual $rootSourceBefore $rootSourceAfterCopy 'Source inputs changed while the isolated snapshot was copied'
    Assert-F2sSourceInputBindingEqual $rootSourceBefore $snapshotSource 'Isolated source snapshot differs from its source digest'
    Assert-F2sDeterministicCargoEnvironment $snapshot
    Assert-F2sDeterministicNodeEnvironment $snapshot

    Push-Location $snapshot
    try{
        & npm.cmd ci --offline --ignore-scripts --no-audit --no-fund
        if($LASTEXITCODE -ne 0){throw "Offline locked npm materialization failed with exit code $LASTEXITCODE"}
        & npm.cmd run typecheck -w @f2s/desktop-ui
        if($LASTEXITCODE -ne 0){throw "UI typecheck failed with exit code $LASTEXITCODE"}
        & node tools/frontend/build-ui.mjs
        if($LASTEXITCODE -ne 0){throw "UI build failed with exit code $LASTEXITCODE"}
        $snapshotSourceAfterUi=Get-F2sSourceInputBinding $snapshot
        Assert-F2sSourceInputBindingEqual $snapshotSource $snapshotSourceAfterUi 'Isolated source inputs changed while the UI was building'
        $uiBundle=Get-F2sUiBundleBinding $snapshot
        $toolchainFingerprintSha256=Get-F2sToolchainFingerprintSha256
        $buildInputSha256=Get-F2sCombinedBuildInputSha256 $snapshotSource $uiBundle $toolchainFingerprintSha256
        $previousBuildInput=[Environment]::GetEnvironmentVariable('F2S_BUILD_INPUT_SHA256')
        $previousTargetDir=[Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR')
        try{
            $env:F2S_BUILD_INPUT_SHA256=$buildInputSha256
            $env:CARGO_TARGET_DIR=Join-Path $snapshot 'cargo-target'
            & cargo build -p flash-to-spine-desktop --release --locked --offline --target $target
            if($LASTEXITCODE -ne 0){throw "Cargo build failed with exit code $LASTEXITCODE"}
        }finally{
            if($null -eq $previousBuildInput){Remove-Item Env:F2S_BUILD_INPUT_SHA256 -ErrorAction SilentlyContinue}else{$env:F2S_BUILD_INPUT_SHA256=$previousBuildInput}
            if($null -eq $previousTargetDir){Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue}else{$env:CARGO_TARGET_DIR=$previousTargetDir}
        }
        Assert-F2sSourceInputBindingEqual $snapshotSource (Get-F2sSourceInputBinding $snapshot) 'Isolated source inputs changed while cargo was compiling'
        Assert-F2sUiBundleBindingEqual $uiBundle (Get-F2sUiBundleBinding $snapshot) 'Built UI changed while cargo was compiling'
    }finally{Pop-Location}

    Assert-F2sSourceInputBindingEqual $rootSourceBefore (Get-F2sSourceInputBinding $root) 'Workspace source changed during the isolated build'
    $stagedExe=Join-Path $snapshot "cargo-target\$target\release\FlashToSpine.exe"
    if(-not(Test-Path -LiteralPath $stagedExe -PathType Leaf)){throw 'Isolated release EXE was not produced.'}
    $exeSha256=(Get-FileHash -LiteralPath $stagedExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $buildBinding=[ordered]@{
        schemaVersion='f2s-core-build-binding/1.0.0';target=$target;profile='release';cargoLocked=$true;cargoOffline=$true
        executableSha256=$exeSha256;buildInputSha256=$buildInputSha256;toolchainFingerprintSha256=$toolchainFingerprintSha256
        sourceTreeSha256=$rootSourceBefore.sourceTreeSha256;sourceFileCount=$rootSourceBefore.sourceFileCount
        cargoLockSha256=$rootSourceBefore.cargoLockSha256;nodeLockSha256=$rootSourceBefore.nodeLockSha256
        uiBundleSha256=$uiBundle.uiBundleSha256;uiBundleFileCount=$uiBundle.uiBundleFileCount
    }
    $snapshotBinding=Join-Path $snapshot 'FlashToSpine.build-binding.json'
    [IO.File]::WriteAllText($snapshotBinding,(($buildBinding|ConvertTo-Json -Depth 5)+[Environment]::NewLine),$utf8)
    & node (Join-Path $PSScriptRoot 'validate-json-contract.mjs') (Join-Path $PSScriptRoot 'core-build-binding.schema.json') $snapshotBinding
    if($LASTEXITCODE -ne 0){throw 'Generated build binding failed its strict schema.'}
    Assert-BuiltExeBinding $stagedExe $buildInputSha256

    Copy-Item -LiteralPath $stagedExe -Destination $exeNew
    Copy-Item -LiteralPath $snapshotBinding -Destination $bindingNew
    Copy-Item -LiteralPath (Join-Path $snapshot 'apps\desktop-ui\dist') -Destination $uiStage -Recurse
    if((Get-FileHash -LiteralPath $exeNew -Algorithm SHA256).Hash.ToLowerInvariant() -ne $exeSha256){throw 'Staged release EXE copy changed.'}
    Assert-F2sUiBundleBindingEqual $uiBundle (Get-F2sUiBundleBindingAtDirectory $uiStage) 'Staged UI copy changed'

    $transaction=[ordered]@{schemaVersion='f2s-build-transaction/1.0.0';hadExe=(Test-Path -LiteralPath $destinationExe);hadBinding=(Test-Path -LiteralPath $destinationBinding);hadUi=(Test-Path -LiteralPath $rootUiDist)}
    $markerTemp=$transactionMarker+'.tmp'
    [IO.File]::WriteAllText($markerTemp,(($transaction|ConvertTo-Json -Compress)+[Environment]::NewLine),$utf8)
    Move-Item -LiteralPath $markerTemp -Destination $transactionMarker -Force
    try{
        if($transaction.hadExe){Move-Item -LiteralPath $destinationExe -Destination $exeBackup}
        if($transaction.hadBinding){Move-Item -LiteralPath $destinationBinding -Destination $bindingBackup}
        if($transaction.hadUi){Move-Item -LiteralPath $rootUiDist -Destination $uiBackup}
        Move-Item -LiteralPath $exeNew -Destination $destinationExe
        Move-Item -LiteralPath $bindingNew -Destination $destinationBinding
        Move-Item -LiteralPath $uiStage -Destination $rootUiDist
        if((Get-FileHash -LiteralPath $destinationExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $exeSha256){throw 'Committed release EXE hash mismatch.'}
        & node (Join-Path $PSScriptRoot 'validate-json-contract.mjs') (Join-Path $PSScriptRoot 'core-build-binding.schema.json') $destinationBinding
        if($LASTEXITCODE -ne 0){throw 'Committed build binding schema mismatch.'}
        Assert-F2sUiBundleBindingEqual $uiBundle (Get-F2sUiBundleBinding $root) 'Committed UI differs from the embedded UI'
        Assert-BuiltExeBinding $destinationExe $buildInputSha256
        Assert-F2sSourceInputBindingEqual $rootSourceBefore (Get-F2sSourceInputBinding $root) 'Workspace source changed before build commit'
        Remove-Item -LiteralPath $transactionMarker -Force
    }catch{
        Restore-InterruptedBuild
        throw
    }
    foreach($path in @($exeBackup,$bindingBackup)){if(Test-Path -LiteralPath $path){Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue}}
    if(Test-Path -LiteralPath $uiBackup){Remove-Item -LiteralPath $uiBackup -Recurse -Force -ErrorAction SilentlyContinue}
    $result=[ordered]@{
        schemaVersion='1.0.0';status='BUILT';profile='release';offline=$true;locked=$true;target=$target
        executable="target/$target/release/FlashToSpine.exe";sha256=$exeSha256
        buildBinding="target/$target/release/FlashToSpine.build-binding.json";buildInputSha256=$buildInputSha256
        signatureStatus='NOT_RUN_EXTERNAL';cleanVmStatus='UNVERIFIED';sourceIsolation='EPHEMERAL_SNAPSHOT_VERIFIED'
        dependencyMaterialization='NPM_CI_OFFLINE_LOCKED_IGNORE_SCRIPTS';artifactCommit='RECOVERABLE_MULTI_ARTIFACT_TRANSACTION'
    }
}finally{
    foreach($path in @($exeNew,$bindingNew)){if(Test-Path -LiteralPath $path){Remove-Item -LiteralPath $path -Force}}
    if(Test-Path -LiteralPath $uiStage){Remove-Item -LiteralPath $uiStage -Recurse -Force}
    if(Test-Path -LiteralPath $snapshot){
        $safePrefix=$snapshotBase.TrimEnd('\')+'\'
        if(-not $snapshot.StartsWith($safePrefix,[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe build snapshot cleanup path.'}
        Remove-Item -LiteralPath $snapshot -Recurse -Force
    }
    if($mutexAcquired){$buildMutex.ReleaseMutex()}
    $buildMutex.Dispose()
}
$result|ConvertTo-Json -Depth 5
