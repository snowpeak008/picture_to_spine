[CmdletBinding()]
param(
    [string]$OutputPath = 'evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-02/gpu-matrix.json',
    [string]$ResourcePath = 'evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-02/resource.csv'
)
$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest
function Resolve-Exe([string]$Name){$c=Get-Command $Name -ErrorAction SilentlyContinue|Select-Object -First 1;if($c){if($c.Source){$c.Source}else{$c.Path}}else{$null}}
$nvidia=Resolve-Exe 'nvidia-smi.exe'; $python=Resolve-Exe 'python.exe'; $gpu=@(); $driver=$null
if($nvidia){
  $rows=@(& $nvidia --query-gpu=name,memory.total,driver_version --format=csv,noheader,nounits 2>$null)
  foreach($row in $rows){$p=$row.Split(',')|ForEach-Object{$_.Trim()};if($p.Count-ge 3){$gpu+=[ordered]@{name=$p[0];memoryMiB=[int]$p[1];driverVersion=$p[2]};$driver=$p[2]}}
}
$pythonVersion=$null;if($python){$pythonVersion=(@(& $python --version 2>&1)-join ' ').Trim()}
$profiles=@(512,1024,2048|ForEach-Object{[ordered]@{resolution=$_;status='NOT_RUN';peakVramMiB=$null;peakRamMiB=$null;latencyMs=$null;reason='No model or CUDA workload is bundled by Core.'}})
$has8Gb=@($gpu|Where-Object memoryMiB -ge 8192).Count-gt 0
$result=[ordered]@{schemaVersion='1.0.0';probeId='F2S-GPU-PROFILE-PROBE-001';status='NOT_RUN';capabilityState='UNVERIFIED';observedAtUtc=[DateTimeOffset]::UtcNow.ToString('o');controls=@([ordered]@{id='gpu-identity';state=if($gpu.Count){'OBSERVED'}else{'MISSING'}},[ordered]@{id='8gb-budget';state=if($has8Gb){'OBSERVED'}else{'UNVERIFIED'}},[ordered]@{id='oom-cancel-cleanup';state='NOT_RUN'});gpu=$gpu;driverVersion=$driver;pythonVersion=$pythonVersion;cudaWorkloadBundled=$false;profiles=$profiles;fallback='Core manual workflow remains enabled; Worker disabled until M09 verification.';notRunReason='Read-only identity probe only; Python/CUDA/model installation and GPU load were not authorized or bundled.'}
foreach($path in @($OutputPath,$ResourcePath)){ $parent=Split-Path -Parent $path;if($parent -and -not (Test-Path -LiteralPath $parent)){New-Item -ItemType Directory -Path $parent|Out-Null} }
[IO.File]::WriteAllText((Join-Path (Get-Location) $OutputPath),(($result|ConvertTo-Json -Depth 12)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path (Get-Location) $ResourcePath),"resolution,status,peakVramMiB,peakRamMiB,latencyMs`n512,NOT_RUN,,,`n1024,NOT_RUN,,,`n2048,NOT_RUN,,,`n",[Text.UTF8Encoding]::new($false))
$result
