[CmdletBinding()]
param()

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'build-input-binding.ps1')
$base=[IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) 'FlashToSpine-binding-selftest'))
$copy=[IO.Path]::GetFullPath((Join-Path $base ([Guid]::NewGuid().ToString('N'))))

function Copy-TestItem([string]$Relative){
    $source=Join-Path $root $Relative;$destination=Join-Path $copy $Relative
    $parent=Split-Path -Parent $destination
    if(-not(Test-Path -LiteralPath $parent)){New-Item -ItemType Directory -Path $parent -Force|Out-Null}
    Copy-Item -LiteralPath $source -Destination $destination -Recurse -Force
}

try{
    New-Item -ItemType Directory -Path $copy -Force|Out-Null
    foreach($relative in @(
        'Cargo.toml','Cargo.lock','rust-toolchain.toml','.node-version','package.json','package-lock.json',
        'crates','apps\desktop\src-tauri','apps\desktop-ui\src','apps\desktop-ui\package.json',
        'apps\desktop-ui\tsconfig.json','apps\desktop-ui\dist','schemas','fixtures\m00\spine42-probe',
        'tools\frontend','tools\windows\build-core.ps1','tools\windows\build-input-binding.ps1'
    )){Copy-TestItem $relative}
    if(Test-Path -LiteralPath (Join-Path $root '.cargo')){Copy-TestItem '.cargo'}
    $before=Get-F2sSourceInputBinding $copy
    $hiddenBuild=Join-Path $copy 'apps\desktop\src-tauri\build.rs'
    [IO.File]::WriteAllText($hiddenBuild,'fn main(){println!("cargo:rustc-cfg=f2s_hidden_test");}',[Text.UTF8Encoding]::new($false))
    (Get-Item -LiteralPath $hiddenBuild).Attributes=[IO.FileAttributes]::Hidden
    $after=Get-F2sSourceInputBinding $copy
    if($before.sourceTreeSha256 -eq $after.sourceTreeSha256){throw 'Hidden Cargo build input did not change the source binding.'}
    Remove-Item -LiteralPath $hiddenBuild -Force

    $previous=[Environment]::GetEnvironmentVariable('CARGO_PROFILE_RELEASE_OPT_LEVEL')
    try{
        $env:CARGO_PROFILE_RELEASE_OPT_LEVEL='0'
        $rejected=$false
        try{Assert-F2sDeterministicCargoEnvironment $copy}catch{$rejected=$true}
        if(-not $rejected){throw 'Cargo profile override was not rejected.'}
    }finally{
        if($null -eq $previous){Remove-Item Env:CARGO_PROFILE_RELEASE_OPT_LEVEL -ErrorAction SilentlyContinue}else{$env:CARGO_PROFILE_RELEASE_OPT_LEVEL=$previous}
    }
    $previousNodeOptions=[Environment]::GetEnvironmentVariable('NODE_OPTIONS')
    try{
        $env:NODE_OPTIONS='--require=C:\f2s-should-not-load.cjs'
        $nodeRejected=$false
        try{Assert-F2sDeterministicNodeEnvironment $copy}catch{$nodeRejected=$true}
        if(-not $nodeRejected){throw 'NODE_OPTIONS injection was not rejected.'}
    }finally{
        if($null -eq $previousNodeOptions){Remove-Item Env:NODE_OPTIONS -ErrorAction SilentlyContinue}else{$env:NODE_OPTIONS=$previousNodeOptions}
    }
    $previousEsbuild=[Environment]::GetEnvironmentVariable('ESBUILD_BINARY_PATH')
    try{
        $env:ESBUILD_BINARY_PATH='C:\f2s-untrusted-esbuild.exe'
        $buildBinaryRejected=$false
        try{Assert-F2sDeterministicNodeEnvironment $copy}catch{$buildBinaryRejected=$true}
        if(-not $buildBinaryRejected){throw 'ESBUILD_BINARY_PATH injection was not rejected.'}
    }finally{
        if($null -eq $previousEsbuild){Remove-Item Env:ESBUILD_BINARY_PATH -ErrorAction SilentlyContinue}else{$env:ESBUILD_BINARY_PATH=$previousEsbuild}
    }
    $previousPrefix=[Environment]::GetEnvironmentVariable('NPM_CONFIG_PREFIX')
    try{
        $env:NPM_CONFIG_PREFIX='C:\f2s-untrusted-prefix'
        $prefixRejected=$false
        try{Assert-F2sDeterministicNodeEnvironment $copy}catch{$prefixRejected=$true}
        if(-not $prefixRejected){throw 'Non-default NPM_CONFIG_PREFIX was not rejected.'}
    }finally{
        if($null -eq $previousPrefix){Remove-Item Env:NPM_CONFIG_PREFIX -ErrorAction SilentlyContinue}else{$env:NPM_CONFIG_PREFIX=$previousPrefix}
    }
    [ordered]@{status='PASS';hiddenInputDetected=$true;profileOverrideRejected=$true;nodeOverrideRejected=$true;buildBinaryOverrideRejected=$true;prefixOverrideRejected=$true}|ConvertTo-Json -Compress
}finally{
    if(Test-Path -LiteralPath $copy){
        if(-not $copy.StartsWith($base.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe binding selftest cleanup path.'}
        Remove-Item -LiteralPath $copy -Recurse -Force
    }
}
