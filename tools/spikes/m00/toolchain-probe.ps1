[CmdletBinding()]
param(
    [ValidateSet('local-observation', 'clean-vm-a', 'clean-vm-b')]
    [string]$RunnerKind = 'local-observation',
    [string]$OutputPath = 'evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-01/probe.json',
    [string]$LogPath = 'evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-01/probe.log'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Sha([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Resolve-Executable([string]$Name, [string[]]$Candidates = @()) {
    foreach ($candidate in $Candidates) {
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    $command = Get-Command $Name -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $command) { return $null }
    if ($command.Source) { return $command.Source }
    return $command.Path
}

function Invoke-Version([string]$Id, [string]$Executable, [string[]]$Arguments, [string]$Source) {
    if ([string]::IsNullOrWhiteSpace($Executable)) {
        return [ordered]@{ id=$Id; state='MISSING'; version=$null; executablePath=$null; sha256=$null; source=$Source; exitCode=$null }
    }
    $exitCode = 0
    $version = $null
    try {
        $lines = @(& $Executable @Arguments 2>&1)
        $exitCode = $LASTEXITCODE
        $version = ($lines -join ' ').Trim()
    } catch {
        $exitCode = 1
        $version = $_.Exception.Message
    }
    $state = if ($exitCode -eq 0) { if ($RunnerKind -eq 'local-observation') { 'OBSERVED_LOCAL' } else { 'UNVERIFIED' } } else { 'FAILED' }
    return [ordered]@{ id=$Id; state=$state; version=$version; executablePath=$Executable; sha256=Get-Sha $Executable; source=$Source; exitCode=$exitCode }
}

$nodePath = Resolve-Executable 'node.exe'
$npmPath = Resolve-Executable 'npm.cmd'
$rustcPath = Resolve-Executable 'rustc.exe'
$cargoPath = Resolve-Executable 'cargo.exe'
$pythonPath = Resolve-Executable 'python.exe'
$uvPath = Resolve-Executable 'uv.exe'
$vswhere = Resolve-Executable 'vswhere.exe' @("${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe")
$clPath = $null
if ($vswhere) {
    $install = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null | Select-Object -First 1)
    if ($install) {
        $clPath = Get-ChildItem -LiteralPath (Join-Path $install 'VC\Tools\MSVC') -Recurse -Filter cl.exe -File -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\Hostx64\\x64\\cl\.exe$' } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
    }
}

$sdkVersion = $null
$sdkSource = 'Windows Kits registry/filesystem'
$kitsRoot = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots' -Name KitsRoot10 -ErrorAction SilentlyContinue).KitsRoot10
if ($kitsRoot -and (Test-Path -LiteralPath (Join-Path $kitsRoot 'bin'))) {
    $sdkVersion = Get-ChildItem -LiteralPath (Join-Path $kitsRoot 'bin') -Directory -ErrorAction SilentlyContinue |
        Where-Object Name -Match '^10\.\d+\.\d+\.\d+$' | Sort-Object {[version]$_.Name} -Descending | Select-Object -First 1 -ExpandProperty Name
}

$webViewVersion = $null
$webViewSource = 'EdgeUpdate registry'
foreach ($root in @('HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients','HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients','HKCU:\Software\Microsoft\EdgeUpdate\Clients')) {
    if (-not (Test-Path -LiteralPath $root)) { continue }
    foreach ($key in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
        $props = Get-ItemProperty -LiteralPath $key.PSPath -ErrorAction SilentlyContinue
        if ([string]$props.name -match 'WebView2') { $webViewVersion = [string]$props.pv; break }
    }
    if ($webViewVersion) { break }
}

$tools = @(
    Invoke-Version 'node' $nodePath @('--version') 'explicit PATH resolution + binary hash'
    Invoke-Version 'npm' $npmPath @('--version') 'explicit PATH resolution + binary hash'
    Invoke-Version 'rustc' $rustcPath @('--version') 'rustup-managed executable + binary hash'
    Invoke-Version 'cargo' $cargoPath @('--version') 'rustup-managed executable + binary hash'
    Invoke-Version 'python' $pythonPath @('--version') 'explicit PATH resolution + binary hash'
    Invoke-Version 'uv' $uvPath @('--version') 'explicit PATH resolution + binary hash'
)
$clVersion = if ($clPath) { (Get-Item -LiteralPath $clPath).VersionInfo.ProductVersion } else { $null }
$tools += [ordered]@{ id='msvc-cl'; state=if($clPath){if($RunnerKind -eq 'local-observation'){'OBSERVED_LOCAL'}else{'UNVERIFIED'}}else{'MISSING'}; version=$clVersion; executablePath=$clPath; sha256=Get-Sha $clPath; source='Visual Studio Installer via vswhere + binary hash'; exitCode=if($clPath){0}else{$null} }
$tools += [ordered]@{ id='windows-sdk'; state=if($sdkVersion){if($RunnerKind -eq 'local-observation'){'OBSERVED_LOCAL'}else{'UNVERIFIED'}}else{'MISSING'}; version=$sdkVersion; executablePath=$kitsRoot; sha256=$null; source=$sdkSource; exitCode=if($sdkVersion){0}else{$null} }
$tools += [ordered]@{ id='webview2-runtime'; state=if($webViewVersion){if($RunnerKind -eq 'local-observation'){'OBSERVED_LOCAL'}else{'UNVERIFIED'}}else{'MISSING'}; version=$webViewVersion; executablePath=$null; sha256=$null; source=$webViewSource; exitCode=if($webViewVersion){0}else{$null} }

$overall = if (@($tools | Where-Object state -eq 'FAILED').Count -gt 0) { 'FAILED' } elseif ($RunnerKind -eq 'local-observation') { 'OBSERVED_LOCAL' } else { 'UNVERIFIED' }
$result = [ordered]@{
    schemaVersion = '1.0.0'
    probeId = 'F2S-TOOLCHAIN-PROBE-001'
    observedAtUtc = [DateTimeOffset]::UtcNow.ToString('o')
    machine = [ordered]@{ os=[Environment]::OSVersion.VersionString; architecture=[Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString(); runnerKind=$RunnerKind }
    tools = @($tools | Sort-Object id)
    overallState = $overall
}

foreach ($path in @($OutputPath, $LogPath)) {
    $parent = Split-Path -Parent $path
    if ($parent -and -not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent | Out-Null }
}
[IO.File]::WriteAllText((Join-Path (Get-Location) $OutputPath), (($result | ConvertTo-Json -Depth 12) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
$log = @("probeId=F2S-TOOLCHAIN-PROBE-001", "runnerKind=$RunnerKind", "overallState=$overall") + @($tools | ForEach-Object { "$($_.id)=$($_.state);$($_.version)" })
[IO.File]::WriteAllLines((Join-Path (Get-Location) $LogPath), $log, [Text.UTF8Encoding]::new($false))
$result
