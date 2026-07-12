[CmdletBinding()]
param()

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'core-package-validation.ps1')
$base=[IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) 'FlashToSpine-package-tree-selftest'))
$package=[IO.Path]::GetFullPath((Join-Path $base ([Guid]::NewGuid().ToString('N'))))
try{
    New-Item -ItemType Directory -Path $package -Force|Out-Null
    foreach($name in @('FlashToSpine.exe','README.txt','build-binding.json','checksums.sha256','package-manifest.json')){
        [IO.File]::WriteAllText((Join-Path $package $name),'test',[Text.UTF8Encoding]::new($false))
    }
    Assert-F2sPackagePhysicalTree $package
    $worker=Join-Path $package 'worker'
    New-Item -ItemType Directory -Path $worker|Out-Null
    [IO.File]::WriteAllText((Join-Path $worker 'AppContainerWorker.exe'),'hidden payload',[Text.UTF8Encoding]::new($false))
    (Get-Item -LiteralPath $worker).Attributes=[IO.FileAttributes]::Hidden
    (Get-Item -LiteralPath (Join-Path $worker 'AppContainerWorker.exe') -Force).Attributes=[IO.FileAttributes]::Hidden
    $rejected=$false
    try{Assert-F2sPackagePhysicalTree $package}catch{$rejected=$true}
    if(-not $rejected){throw 'Hidden nested capability payload was not rejected.'}
    [ordered]@{status='PASS';hiddenNestedPayloadRejected=$true}|ConvertTo-Json -Compress
}finally{
    if(Test-Path -LiteralPath $package){
        if(-not $package.StartsWith($base.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe package-tree selftest cleanup path.'}
        Remove-Item -LiteralPath $package -Recurse -Force
    }
}
