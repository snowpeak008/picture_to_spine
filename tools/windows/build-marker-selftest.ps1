[CmdletBinding()]
param()

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'core-package-validation.ps1')
$base=[IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) 'FlashToSpine-build-marker-selftest'))
$root=[IO.Path]::GetFullPath((Join-Path $base ([Guid]::NewGuid().ToString('N'))))
try{
    New-Item -ItemType Directory -Path $root -Force|Out-Null
    Assert-F2sNoInterruptedCoreBuild $root
    $release=Join-Path $root 'target\x86_64-pc-windows-msvc\release'
    New-Item -ItemType Directory -Path $release -Force|Out-Null
    [IO.File]::WriteAllText((Join-Path $release '.f2s-build-transaction.json'),'{}',[Text.UTF8Encoding]::new($false))
    $rejected=$false
    try{Assert-F2sNoInterruptedCoreBuild $root}catch{$rejected=$true}
    if(-not $rejected){throw 'Interrupted build marker was not rejected.'}
    [ordered]@{status='PASS';interruptedBuildRejected=$true}|ConvertTo-Json -Compress
}finally{
    if(Test-Path -LiteralPath $root){
        if(-not $root.StartsWith($base.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe build marker selftest cleanup path.'}
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
