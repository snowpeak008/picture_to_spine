[CmdletBinding()]param([switch]$Wait)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$candidates=@(
    (Join-Path $root 'FlashToSpineLauncher.exe'),
    (Join-Path $root 'dist\FlashToSpine-Core\FlashToSpine.exe'),
    (Join-Path $root 'target\x86_64-pc-windows-msvc\release\FlashToSpine.exe'),
    (Join-Path $root 'target\release\FlashToSpine.exe'),
    (Join-Path $root 'target\debug\FlashToSpine.exe')
)
$exe=$candidates|Where-Object{Test-Path -LiteralPath $_ -PathType Leaf}|Select-Object -First 1
$resultPath=Join-Path $root 'evidence\M01\F2S-DEV-M01-004\F2S-WU-M01-004-01\launcher-result.json'
$parent=Split-Path -Parent $resultPath
if(-not(Test-Path -LiteralPath $parent)){New-Item -ItemType Directory -Path $parent|Out-Null}
if(-not $exe){
    $r=[ordered]@{schemaVersion='1.0.0';status='NOT_BUILT';diagnosticCode='F2S-LAUNCH-001';executablePath=$null;message='FlashToSpine Core is not built. Run npm run build:core, then npm run release:verify. These commands use locked local dependencies and do not install or elevate.'}
    [IO.File]::WriteAllText($resultPath,(($r|ConvertTo-Json)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
    Write-Host $r.message -ForegroundColor Yellow
    exit 3
}
try{
    $hash=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    $process=Start-Process -FilePath $exe -WorkingDirectory (Split-Path -Parent $exe) -PassThru -WindowStyle Normal
    $exitCode=$null
    if($Wait){
        $process.WaitForExit()
        $exitCode=$process.ExitCode
        if($exitCode -ne 0){throw "FlashToSpine exited with code $exitCode"}
    }
    $r=[ordered]@{schemaVersion='1.0.0';status='STARTED';diagnosticCode='F2S-LAUNCH-OK';executablePath=$exe;executableSha256=$hash;processId=$process.Id;exitCode=$exitCode;message='FlashToSpine started.'}
    [IO.File]::WriteAllText($resultPath,(($r|ConvertTo-Json)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
    Write-Host $r.message -ForegroundColor Green
}catch{
    $r=[ordered]@{schemaVersion='1.0.0';status='FAILED';diagnosticCode='F2S-LAUNCH-002';executablePath=$exe;message=$_.Exception.Message}
    [IO.File]::WriteAllText($resultPath,(($r|ConvertTo-Json)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
    Write-Error $r.message
    exit 2
}
