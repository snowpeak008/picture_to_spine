Set-StrictMode -Version Latest

function Get-F2sSha256Text([string]$Text){
    $utf8=[Text.UTF8Encoding]::new($false)
    $sha=[Security.Cryptography.SHA256]::Create()
    try{
        ([BitConverter]::ToString($sha.ComputeHash($utf8.GetBytes($Text)))).Replace('-','').ToLowerInvariant()
    }finally{
        $sha.Dispose()
    }
}

function Get-F2sFileSetBinding([string]$Root,[string[]]$Files){
    $rootFull=[IO.Path]::GetFullPath($Root).TrimEnd('\')
    $uniqueFiles=@($Files | ForEach-Object {[IO.Path]::GetFullPath($_)} | Sort-Object -Unique)
    $canonical=($uniqueFiles | ForEach-Object {
        if(-not(Test-Path -LiteralPath $_ -PathType Leaf)){throw "Required binding input is missing: $_"}
        $item=Get-Item -LiteralPath $_ -Force
        if(($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw "Build input must not be a reparse point: $_"}
        if(-not $_.StartsWith($rootFull+'\',[StringComparison]::OrdinalIgnoreCase)){
            throw "Build input escaped the project root: $_"
        }
        $relative=$_.Substring($rootFull.Length+1).Replace('\','/')
        $hash=(Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant()
        $bytes=$item.Length
        "$relative|$hash|$bytes"
    }) -join "`n"
    [ordered]@{sha256=(Get-F2sSha256Text $canonical);fileCount=$uniqueFiles.Count}
}

function Get-F2sSourceInputBinding([string]$Root){
    $rootFull=[IO.Path]::GetFullPath($Root).TrimEnd('\')
    $files=New-Object 'System.Collections.Generic.List[string]'
    foreach($relative in @(
        'Cargo.toml',
        'Cargo.lock',
        'rust-toolchain.toml',
        '.node-version',
        'package.json',
        'package-lock.json',
        'apps\desktop-ui\package.json',
        'apps\desktop-ui\tsconfig.json',
        'tools\frontend\build-ui.mjs',
        'tools\windows\build-core.ps1',
        'tools\windows\build-input-binding.ps1'
    )){
        $path=Join-Path $rootFull $relative
        if(-not(Test-Path -LiteralPath $path -PathType Leaf)){
            throw "Required source input is missing: $relative"
        }
        $files.Add($path)
    }
    foreach($relative in @(
        'crates',
        'apps\desktop\src-tauri',
        'apps\desktop-ui\src',
        'schemas',
        'fixtures\m00\spine42-probe'
    )){
        $directory=Join-Path $rootFull $relative
        if(-not(Test-Path -LiteralPath $directory -PathType Container)){
            throw "Required source input directory is missing: $relative"
        }
        $directoryItem=Get-Item -LiteralPath $directory -Force
        if(($directoryItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw "Source input directory must not be a reparse point: $relative"}
        if(@(Get-ChildItem -LiteralPath $directory -Recurse -Directory -Force | Where-Object {($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0}).Count){
            throw "Source input tree contains a reparse directory: $relative"
        }
        foreach($item in Get-ChildItem -LiteralPath $directory -Recurse -File -Force){
            $files.Add($item.FullName)
        }
    }
    $repoCargoState='ABSENT'
    $repoCargo=Join-Path $rootFull '.cargo'
    if(Test-Path -LiteralPath $repoCargo -PathType Container){
        $repoCargoState='PRESENT'
        $repoCargoItem=Get-Item -LiteralPath $repoCargo -Force
        if(($repoCargoItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Repository .cargo directory must not be a reparse point.'}
        if(@(Get-ChildItem -LiteralPath $repoCargo -Recurse -Directory -Force | Where-Object {($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0}).Count){throw 'Repository .cargo tree contains a reparse directory.'}
        foreach($item in Get-ChildItem -LiteralPath $repoCargo -Recurse -File -Force){$files.Add($item.FullName)}
    }
    $binding=Get-F2sFileSetBinding $rootFull $files.ToArray()
    [ordered]@{
        sourceTreeSha256=Get-F2sSha256Text ($binding.sha256+'|repoCargoDirectory='+$repoCargoState)
        sourceFileCount=$binding.fileCount
        cargoLockSha256=(Get-FileHash -LiteralPath (Join-Path $rootFull 'Cargo.lock') -Algorithm SHA256).Hash.ToLowerInvariant()
        nodeLockSha256=(Get-FileHash -LiteralPath (Join-Path $rootFull 'package-lock.json') -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Assert-F2sDeterministicCargoEnvironment([string]$Root){
    $rootFull=[IO.Path]::GetFullPath($Root).TrimEnd('\')
    $externalConfigs=New-Object 'System.Collections.Generic.List[string]'
    $cursor=[IO.DirectoryInfo]::new($rootFull).Parent
    while($null -ne $cursor){
        foreach($name in @('config','config.toml')){
            $candidate=Join-Path $cursor.FullName ('.cargo\'+$name)
            if(Test-Path -LiteralPath $candidate -PathType Leaf){$externalConfigs.Add($candidate)}
        }
        $cursor=$cursor.Parent
    }
    $cargoHome=[Environment]::GetEnvironmentVariable('CARGO_HOME')
    if([string]::IsNullOrWhiteSpace($cargoHome)){$cargoHome=Join-Path $HOME '.cargo'}
    foreach($name in @('config','config.toml')){
        $candidate=Join-Path $cargoHome $name
        if((Test-Path -LiteralPath $candidate -PathType Leaf) -and -not([IO.Path]::GetFullPath($candidate).StartsWith($rootFull+'\',[StringComparison]::OrdinalIgnoreCase))){
            $externalConfigs.Add([IO.Path]::GetFullPath($candidate))
        }
    }
    $external=@($externalConfigs|Sort-Object -Unique)
    if($external.Count){throw 'External Cargo config is not allowed for a source-bound Core build.'}
    $forbidden=@(Get-ChildItem Env: | Where-Object {
        $_.Name -in @(
            'RUSTC','RUSTDOC','RUSTFLAGS','RUSTDOCFLAGS','RUSTC_BOOTSTRAP','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER',
            'CARGO_ENCODED_RUSTFLAGS','CARGO_TARGET_DIR','CARGO_INCREMENTAL','CC','CXX','AR','CFLAGS','CXXFLAGS','LDFLAGS','CL','_CL_','LINK'
        ) -or $_.Name -match '^CARGO_PROFILE_' -or
        $_.Name -match '^CARGO_BUILD_(RUSTFLAGS|RUSTC|RUSTC_WRAPPER|TARGET|JOBS|INCREMENTAL)$' -or
        $_.Name -match '^CARGO_TARGET_.*_(LINKER|RUNNER|RUSTFLAGS)$' -or
        $_.Name -match '^(CC|CXX|AR|CFLAGS|CXXFLAGS|LDFLAGS)_'
    } | Where-Object {-not [string]::IsNullOrWhiteSpace($_.Value)})
    if($forbidden.Count){throw 'Ambient compiler/profile override variables are not allowed for a source-bound Core build.'}
}

function Assert-F2sDeterministicNodeEnvironment([string]$Root){
    $rootFull=[IO.Path]::GetFullPath($Root).TrimEnd('\')
    $forbiddenEnv=@(Get-ChildItem Env: | Where-Object {
        $_.Name -in @(
            'NODE_OPTIONS','NODE_PATH','ESBUILD_BINARY_PATH','NPM_CONFIG_NODE_OPTIONS','NPM_CONFIG_SCRIPT_SHELL','NPM_CONFIG_SHELL'
        )
    } | Where-Object {-not [string]::IsNullOrWhiteSpace($_.Value)})
    if($forbiddenEnv.Count){throw "Ambient Node/npm override variables are not allowed for a source-bound Core build: $($forbiddenEnv.Name -join ', ')"}
    $npmPrefix=[Environment]::GetEnvironmentVariable('NPM_CONFIG_PREFIX')
    $defaultNpmPrefix=Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'npm'
    if(-not [string]::IsNullOrWhiteSpace($npmPrefix) -and [IO.Path]::GetFullPath($npmPrefix) -ne [IO.Path]::GetFullPath($defaultNpmPrefix)){
        throw 'Non-default NPM_CONFIG_PREFIX is not allowed for a source-bound Core build.'
    }
    $configs=New-Object 'System.Collections.Generic.List[string]'
    $cursor=[IO.DirectoryInfo]::new($rootFull)
    while($null -ne $cursor){
        $candidate=Join-Path $cursor.FullName '.npmrc'
        if(Test-Path -LiteralPath $candidate -PathType Leaf){$configs.Add($candidate)}
        $cursor=$cursor.Parent
    }
    foreach($candidate in @(
        (Join-Path $HOME '.npmrc'),
        (Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'npm\etc\npmrc'),
        [Environment]::GetEnvironmentVariable('NPM_CONFIG_USERCONFIG'),
        [Environment]::GetEnvironmentVariable('NPM_CONFIG_GLOBALCONFIG')
    )){
        if(-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path -LiteralPath $candidate -PathType Leaf)){$configs.Add($candidate)}
    }
    if(@($configs|Sort-Object -Unique).Count){throw 'External or project npmrc configuration is not allowed for an offline source-bound Core build.'}
}

function Get-F2sUiBundleBindingAtDirectory([string]$Directory){
    $directory=[IO.Path]::GetFullPath($Directory).TrimEnd('\')
    if(-not(Test-Path -LiteralPath $directory -PathType Container)){
        throw 'Built desktop UI directory is missing.'
    }
    $directoryItem=Get-Item -LiteralPath $directory -Force
    if(($directoryItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Built desktop UI directory must not be a reparse point.'}
    if(@(Get-ChildItem -LiteralPath $directory -Recurse -Directory -Force | Where-Object {($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0}).Count){throw 'Built desktop UI tree contains a reparse directory.'}
    $files=@(Get-ChildItem -LiteralPath $directory -Recurse -File -Force)
    if($files.Count -lt 3){throw 'Built desktop UI file set is incomplete.'}
    $canonical=($files|Sort-Object FullName|ForEach-Object {
        if(($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Built desktop UI contains a reparse file.'}
        $relative=$_.FullName.Substring($directory.Length+1).Replace('\','/')
        "$relative|$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant())|$($_.Length)"
    }) -join "`n"
    [ordered]@{uiBundleSha256=(Get-F2sSha256Text $canonical);uiBundleFileCount=$files.Count}
}

function Get-F2sUiBundleBinding([string]$Root){
    $rootFull=[IO.Path]::GetFullPath($Root).TrimEnd('\')
    Get-F2sUiBundleBindingAtDirectory (Join-Path $rootFull 'apps\desktop-ui\dist')
}

function Get-F2sToolchainFingerprintSha256(){
    $rustc=(& rustc -Vv 2>&1|Out-String).Trim();if($LASTEXITCODE -ne 0){throw 'rustc fingerprint probe failed.'}
    $cargo=(& cargo -V 2>&1|Out-String).Trim();if($LASTEXITCODE -ne 0){throw 'cargo fingerprint probe failed.'}
    $node=(& node --version 2>&1|Out-String).Trim();if($LASTEXITCODE -ne 0){throw 'Node fingerprint probe failed.'}
    $npm=(& npm.cmd --version 2>&1|Out-String).Trim();if($LASTEXITCODE -ne 0){throw 'npm fingerprint probe failed.'}
    Get-F2sSha256Text ("rustc=$rustc`ncargo=$cargo`nnode=$node`nnpm=$npm")
}

function Get-F2sCombinedBuildInputSha256($SourceBinding,$UiBinding,[string]$ToolchainFingerprintSha256){
    if($ToolchainFingerprintSha256 -notmatch '^[0-9a-f]{64}$'){throw 'toolchain fingerprint sha256 is invalid.'}
    Get-F2sSha256Text ($SourceBinding.sourceTreeSha256+'|'+$SourceBinding.sourceFileCount+'|'+$SourceBinding.cargoLockSha256+'|'+$SourceBinding.nodeLockSha256+'|'+$UiBinding.uiBundleSha256+'|'+$UiBinding.uiBundleFileCount+'|'+$ToolchainFingerprintSha256)
}

function Assert-F2sPropertiesEqual($Expected,$Actual,[string[]]$Properties,[string]$Context){
    foreach($property in $Properties){
        if($Expected.$property -ne $Actual.$property){
            throw "${Context}: $property differs"
        }
    }
}

function Assert-F2sSourceInputBindingEqual($Expected,$Actual,[string]$Context){
    Assert-F2sPropertiesEqual $Expected $Actual @('sourceTreeSha256','sourceFileCount','cargoLockSha256','nodeLockSha256') $Context
}

function Assert-F2sUiBundleBindingEqual($Expected,$Actual,[string]$Context){
    Assert-F2sPropertiesEqual $Expected $Actual @('uiBundleSha256','uiBundleFileCount') $Context
}
