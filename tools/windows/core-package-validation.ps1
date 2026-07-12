Set-StrictMode -Version Latest
$script:F2sWindowsToolsRoot=$PSScriptRoot
. (Join-Path $script:F2sWindowsToolsRoot 'build-input-binding.ps1')

function Get-F2sExpectedCoreReadme(){
    $value=@'
FlashToSpine Windows Portable Core (internal candidate)

- Double-click FlashToSpine.exe to start the direct native WebView2 host.
- The system Evergreen WebView2 Runtime is required and is not bundled or installed.
- Spine Editor/Professional CLI is user-owned, external, and not bundled or downloaded.
- Private remote GPU transport remains NOT_RUN/EXTERNAL; no public image service is included.
- AppContainer AI Worker is physically excluded from this Core package.
- This package is not code-signed and is not authorized for public release or distribution.
- The package performs no installation, elevation, dependency download, or tool activation.
'@
    $value.Trim()+[Environment]::NewLine
}

function Invoke-F2sJsonContract([string]$Schema,[string]$Value){
    & node (Join-Path $script:F2sWindowsToolsRoot 'validate-json-contract.mjs') $Schema $Value | Out-Null
    if($LASTEXITCODE -ne 0){throw "JSON contract validation failed: $Value"}
}

function Assert-F2sPackagePhysicalTree([string]$Package){
    $packageFull=[IO.Path]::GetFullPath($Package)
    $packageItem=Get-Item -LiteralPath $packageFull -Force
    if(($packageItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Core package root must not be a reparse point.'}
    $reparseEntries=@(Get-ChildItem -LiteralPath $packageFull -Recurse -Force | Where-Object {($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0})
    if($reparseEntries.Count){throw 'Core package must not contain reparse points.'}
    $nestedDirectories=@(Get-ChildItem -LiteralPath $packageFull -Recurse -Directory -Force)
    if($nestedDirectories.Count){throw 'Core package must not contain nested directories or hidden capability payloads.'}
    $actualNames=@(Get-ChildItem -LiteralPath $packageFull -Recurse -File -Force|ForEach-Object {
        $_.FullName.Substring($packageFull.TrimEnd('\').Length+1).Replace('\','/')
    }|Sort-Object)
    $expectedNames=@('FlashToSpine.exe','README.txt','build-binding.json','checksums.sha256','package-manifest.json')|Sort-Object
    if(Compare-Object $expectedNames $actualNames){throw 'Unexpected or missing files are present in the Core package.'}
    foreach($name in $expectedNames){
        $path=Join-Path $packageFull $name
        $extraStreams=@(Get-Item -LiteralPath $path -Stream * -ErrorAction Stop | Where-Object {$_.Stream -ne ':$DATA'})
        if($extraStreams.Count){throw "Core package file contains an alternate data stream: $name"}
    }
}

function Assert-F2sNoInterruptedCoreBuild([string]$Root){
    $marker=Join-Path ([IO.Path]::GetFullPath($Root)) 'target\x86_64-pc-windows-msvc\release\.f2s-build-transaction.json'
    if(Test-Path -LiteralPath $marker -PathType Leaf){
        throw 'An interrupted Core build transaction must be recovered by build-core before packaging or verification.'
    }
}

function Assert-F2sPackageContents(
    [string]$Package,
    [string]$Root,
    [string]$RootLauncher,
    [bool]$RequireRootLauncher
){
    $packageFull=[IO.Path]::GetFullPath($Package)
    $rootFull=[IO.Path]::GetFullPath($Root)
    Assert-F2sNoInterruptedCoreBuild $rootFull
    Assert-F2sDeterministicCargoEnvironment $rootFull
    Assert-F2sDeterministicNodeEnvironment $rootFull
    $manifestPath=Join-Path $packageFull 'package-manifest.json'
    $checksumPath=Join-Path $packageFull 'checksums.sha256'
    $exe=Join-Path $packageFull 'FlashToSpine.exe'
    $buildBindingPath=Join-Path $packageFull 'build-binding.json'
    foreach($path in @($manifestPath,$checksumPath,$exe,$buildBindingPath)){
        if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "Required package artifact is missing: $path"}
    }
    Assert-F2sPackagePhysicalTree $packageFull
    if($RequireRootLauncher -and -not(Test-Path -LiteralPath $RootLauncher -PathType Leaf)){
        throw 'Root launcher is missing.'
    }

    Invoke-F2sJsonContract (Join-Path $script:F2sWindowsToolsRoot 'core-package-manifest.schema.json') $manifestPath
    Invoke-F2sJsonContract (Join-Path $script:F2sWindowsToolsRoot 'core-build-binding.schema.json') $buildBindingPath
    $manifest=Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8|ConvertFrom-Json
    $buildAttestation=Get-Content -LiteralPath $buildBindingPath -Raw -Encoding UTF8|ConvertFrom-Json
    $rootPackage=Get-Content -LiteralPath (Join-Path $rootFull 'package.json') -Raw -Encoding UTF8|ConvertFrom-Json
    if($manifest.version -ne [string]$rootPackage.version){throw 'Package manifest version differs from the source package version.'}
    if((Get-Content -LiteralPath (Join-Path $packageFull 'README.txt') -Raw -Encoding UTF8) -cne (Get-F2sExpectedCoreReadme)){
        throw 'Package README differs from the fixed external-capability boundary text.'
    }

    $sourceBinding=Get-F2sSourceInputBinding $rootFull
    $uiBinding=Get-F2sUiBundleBinding $rootFull
    $toolchainFingerprint=Get-F2sToolchainFingerprintSha256
    $combined=Get-F2sCombinedBuildInputSha256 $sourceBinding $uiBinding $toolchainFingerprint
    Assert-F2sSourceInputBindingEqual $buildAttestation $sourceBinding 'Packaged binary is not bound to the current source inputs'
    Assert-F2sUiBundleBindingEqual $buildAttestation $uiBinding 'Packaged binary is not bound to the current UI bundle'
    if($buildAttestation.toolchainFingerprintSha256 -ne $toolchainFingerprint){throw 'Packaged binary is not bound to the current toolchain fingerprint.'}
    if($buildAttestation.buildInputSha256 -ne $combined){throw 'Build receipt combined input digest is invalid.'}
    Assert-F2sSourceInputBindingEqual $manifest.deterministicInputs $buildAttestation 'Package manifest differs from build-time source inputs'
    Assert-F2sUiBundleBindingEqual $manifest.deterministicInputs $buildAttestation 'Package manifest differs from build-time UI bundle'
    if($manifest.deterministicInputs.buildInputSha256 -ne $buildAttestation.buildInputSha256){throw 'Package manifest combined build input mismatch.'}
    if($manifest.deterministicInputs.toolchainFingerprintSha256 -ne $buildAttestation.toolchainFingerprintSha256){throw 'Package manifest toolchain fingerprint mismatch.'}
    if($manifest.deterministicInputs.buildBindingSha256 -ne (Get-FileHash -LiteralPath $buildBindingPath -Algorithm SHA256).Hash.ToLowerInvariant()){
        throw 'Package manifest does not bind the staged build receipt.'
    }

    $manifestNames=@($manifest.files|ForEach-Object path)
    if(Compare-Object @('FlashToSpine.exe','README.txt','build-binding.json') $manifestNames -SyncWindow 0){throw 'Manifest file list must be exact, unique, and ordered.'}
    foreach($file in $manifest.files){
        $path=Join-Path $packageFull $file.path
        $hash=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if($hash -ne $file.sha256 -or (Get-Item -LiteralPath $path).Length -ne $file.bytes){throw "Manifest mismatch: $($file.path)"}
    }

    $checksumLines=@(Get-Content -LiteralPath $checksumPath -Encoding UTF8)
    if($checksumLines.Count -ne 4){throw 'Checksum file must contain exactly four entries.'}
    $checksumNames=New-Object 'System.Collections.Generic.List[string]'
    foreach($line in $checksumLines){
        if($line -notmatch '^([0-9a-f]{64})  ([A-Za-z0-9._-]+)$'){throw "Invalid checksum line: $line"}
        $checksumNames.Add($Matches[2])
        $path=Join-Path $packageFull $Matches[2]
        if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "Checksum target missing: $($Matches[2])"}
        if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Matches[1]){throw "Checksum mismatch: $($Matches[2])"}
    }
    if(Compare-Object @('FlashToSpine.exe','README.txt','build-binding.json','package-manifest.json') $checksumNames.ToArray() -SyncWindow 0){
        throw 'Checksum target list must be exact, unique, and ordered.'
    }

    $exeHash=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    if($buildAttestation.executableSha256 -ne $exeHash){throw 'Packaged EXE does not match its build-time receipt.'}
    if($RequireRootLauncher -and (Get-FileHash -LiteralPath $RootLauncher -Algorithm SHA256).Hash.ToLowerInvariant() -ne $exeHash){
        throw 'Root launcher is not the packaged Core binary.'
    }
    $header=[IO.File]::ReadAllBytes($exe)
    if($header.Length -lt 2 -or $header[0] -ne 0x4d -or $header[1] -ne 0x5a){throw 'Packaged entrypoint is not a PE executable.'}

    $smokePath=Join-Path ([IO.Path]::GetTempPath()) ("f2s-core-smoke-$([Guid]::NewGuid().ToString('N')).json")
    try{
        $smokeProcess=Start-Process -FilePath $exe -ArgumentList @('--smoke',('"{0}"' -f $smokePath)) -Wait -PassThru -WindowStyle Hidden
        if($smokeProcess.ExitCode -ne 0){throw "Packaged smoke exited with code $($smokeProcess.ExitCode)"}
        $smoke=Get-Content -LiteralPath $smokePath -Raw -Encoding UTF8|ConvertFrom-Json
        if($smoke.status -ne 'PASS' -or -not $smoke.uiEmbedded -or $smoke.networkAllowed){throw 'Packaged smoke report failed.'}
        if($smoke.buildInputSha256 -ne $buildAttestation.buildInputSha256){throw 'Packaged smoke build-input binding mismatch.'}
        if($smoke.signature -ne 'NOT_RUN_EXTERNAL' -or $smoke.webView2Runtime -ne 'NOT_PROBED_SYSTEM_PREREQUISITE'){
            throw 'Smoke report overstates an external capability.'
        }
    }finally{
        if(Test-Path -LiteralPath $smokePath){Remove-Item -LiteralPath $smokePath -Force}
    }
    [ordered]@{executableSha256=$exeHash;buildInputSha256=$combined;manifest=$manifest;buildAttestation=$buildAttestation}
}
