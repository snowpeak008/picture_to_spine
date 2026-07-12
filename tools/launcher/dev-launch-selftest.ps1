$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$entry=Get-ChildItem -LiteralPath $root -File -Filter 'FlashToSpine-*.cmd'|Select-Object -First 1 -ExpandProperty FullName
if(-not $entry){throw 'Development launcher CMD is missing.'}
$files=@(
    $entry,
    (Join-Path $root 'tools\launcher\dev-launch.ps1'),
    (Join-Path $root 'tools\windows\build-core.ps1'),
    (Join-Path $root 'tools\windows\package-core.ps1'),
    (Join-Path $root 'tools\windows\verify-core-package.ps1')
)
$forbidden=@('Invoke-WebRequest','curl ','npm install','npm ci','cargo install','Start-Process powershell','-Verb RunAs','Set-ExecutionPolicy','signtool sign')
$issues=@()
foreach($file in $files){
    if(-not(Test-Path -LiteralPath $file -PathType Leaf)){$issues+="$file is missing";continue}
    $text=Get-Content -LiteralPath $file -Raw
    foreach($token in $forbidden){
        if($text.IndexOf($token,[StringComparison]::OrdinalIgnoreCase)-ge 0){$issues+="$file contains $token"}
    }
}
$result=[ordered]@{schemaVersion='1.0.0';status=if($issues.Count){'FAIL'}else{'PASS'};checkedFiles=$files;issues=$issues}
$result|ConvertTo-Json -Depth 5
if($issues.Count){exit 2}
