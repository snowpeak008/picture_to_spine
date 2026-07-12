[CmdletBinding()]
param(
    [switch]$ExecuteAuthorizedProbe,
    [string]$OutputPath = 'evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-01/sandbox-probe.json',
    [string]$LogPath = 'evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-01/os-events.log'
)
$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest

$controls = @(
    [ordered]@{ id='appcontainer-token'; required=$true; state='NOT_RUN'; detail='Requires native CreateAppContainerProfile/DeriveAppContainerSid harness.' },
    [ordered]@{ id='network-capability-empty'; required=$true; state='NOT_RUN'; detail='Must prove network probe rejection from the actual worker token.' },
    [ordered]@{ id='dedicated-job-root-acl'; required=$true; state='NOT_RUN'; detail='Must prove writes outside the per-job root are denied.' },
    [ordered]@{ id='job-object-kill-and-limits'; required=$true; state='NOT_RUN'; detail='Must prove child termination, memory/process limits and no inherited handles.' },
    [ordered]@{ id='breakaway-denied'; required=$true; state='NOT_RUN'; detail='Must prove child/breakaway and handle escape attempts fail.' }
)

$isWindows = [Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([Runtime.InteropServices.OSPlatform]::Windows)
$nativePrerequisites = [ordered]@{
    windows = $isWindows
    powershellArchitecture = [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
    appContainerApiAvailable = $isWindows -and (Test-Path -LiteralPath "$env:SystemRoot\System32\userenv.dll")
    jobObjectApiAvailable = $isWindows -and (Test-Path -LiteralPath "$env:SystemRoot\System32\kernel32.dll")
}

$status='NOT_RUN'; $capability='UNVERIFIED'; $reason='Read-only preflight completed; native isolated worker harness is not built yet.'
if ($ExecuteAuthorizedProbe) {
    $status='FAIL'; $capability='FAILED'; $reason='Fail closed: ExecuteAuthorizedProbe was requested before the native five-control harness exists.'
    foreach($control in $controls){$control.state='FAILED'}
}
$result=[ordered]@{schemaVersion='1.0.0';probeId='F2S-SANDBOX-PROBE-001';status=$status;capabilityState=$capability;observedAtUtc=[DateTimeOffset]::UtcNow.ToString('o');controls=$controls;prerequisites=$nativePrerequisites;notRunReason=$reason;externalBlockers=@('M09 native windows-appcontainer-v1 harness')}
foreach($path in @($OutputPath,$LogPath)){ $parent=Split-Path -Parent $path; if($parent -and -not(Test-Path -LiteralPath $parent)){New-Item -ItemType Directory -Path $parent|Out-Null} }
[IO.File]::WriteAllText((Join-Path (Get-Location) $OutputPath),(($result|ConvertTo-Json -Depth 12)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllLines((Join-Path (Get-Location) $LogPath),@("status=$status","capabilityState=$capability","reason=$reason"),[Text.UTF8Encoding]::new($false))
$result
if($status -eq 'FAIL'){exit 2}
