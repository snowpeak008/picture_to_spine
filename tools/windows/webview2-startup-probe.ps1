[CmdletBinding()]
param(
    [Parameter()]
    [string]$ExecutablePath = '',

    [Parameter()]
    [ValidateSet('Text', 'JSON')]
    [string]$Output = 'Text'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$expectedClass = 'FlashToSpineWebView'
$expectedTitle = 'FlashToSpine Production Assist'
$waitSeconds = 5
$zeroSha256 = '0' * 64
$process = $null
$topWindow = $null
$failureCode = 'NONE'
$executableName = 'FlashToSpineLauncher.exe'
$executableSha256 = $zeroSha256
$isReparsePoint = $false
$topLevelMatched = $false
$chromeWidgetChildFound = $false
$renderWidgetChildFound = $false
$processResponding = $false
$wmCloseSent = $false
$gracefulExit = $false
$forcedCleanupRequired = $false
$cleanupSucceeded = $false
$phase = 'INITIALIZATION'

function Set-ProbeFailure {
    param([Parameter(Mandatory = $true)][string]$Code)
    if ($script:failureCode -eq 'NONE') {
        $script:failureCode = $Code
    }
}

function Test-ProcessHasExited {
    param([System.Diagnostics.Process]$Candidate)
    if ($null -eq $Candidate) { return $true }
    try {
        $Candidate.Refresh()
        return $Candidate.HasExited
    }
    catch {
        return $true
    }
}

try {
    $phase = 'PLATFORM'
    if ($env:OS -ne 'Windows_NT') {
        Set-ProbeFailure 'PLATFORM_UNSUPPORTED'
        throw 'controlled probe stop'
    }

    $phase = 'NATIVE_API'
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public sealed class F2sProbeWindow
{
    public IntPtr Handle { get; set; }
    public string ClassName { get; set; }
    public string Title { get; set; }
}

public static class F2sWebView2ProbeNative
{
    private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);

    [DllImport("user32.dll")]
    private static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc callback, IntPtr lParam);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr hWnd, StringBuilder className, int maxCount);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);

    [DllImport("user32.dll")]
    private static extern int GetWindowTextLength(IntPtr hWnd);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(
        IntPtr hWnd,
        uint message,
        UIntPtr wParam,
        IntPtr lParam,
        uint flags,
        uint timeout,
        out UIntPtr result);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);

    private static string ReadClassName(IntPtr hWnd)
    {
        StringBuilder value = new StringBuilder(512);
        GetClassName(hWnd, value, value.Capacity);
        return value.ToString();
    }

    private static string ReadTitle(IntPtr hWnd)
    {
        int length = GetWindowTextLength(hWnd);
        StringBuilder value = new StringBuilder(Math.Max(length + 1, 2));
        GetWindowText(hWnd, value, value.Capacity);
        return value.ToString();
    }

    public static F2sProbeWindow FindExpectedTopLevel(uint processId, string expectedClass, string expectedTitle)
    {
        F2sProbeWindow match = null;
        EnumWindows(delegate(IntPtr hWnd, IntPtr ignored)
        {
            uint ownerProcessId;
            GetWindowThreadProcessId(hWnd, out ownerProcessId);
            if (ownerProcessId == processId)
            {
                string className = ReadClassName(hWnd);
                string title = ReadTitle(hWnd);
                if (className == expectedClass && title == expectedTitle)
                {
                    match = new F2sProbeWindow { Handle = hWnd, ClassName = className, Title = title };
                    return false;
                }
            }
            return true;
        }, IntPtr.Zero);
        return match;
    }

    public static string[] EnumerateDescendantClasses(IntPtr parent)
    {
        List<string> classes = new List<string>();
        EnumChildWindows(parent, delegate(IntPtr hWnd, IntPtr ignored)
        {
            classes.Add(ReadClassName(hWnd));
            return true;
        }, IntPtr.Zero);
        return classes.ToArray();
    }

    public static bool IsResponding(IntPtr hWnd)
    {
        const uint WM_NULL = 0x0000;
        const uint SMTO_ABORTIFHUNG = 0x0002;
        UIntPtr result;
        return SendMessageTimeout(hWnd, WM_NULL, UIntPtr.Zero, IntPtr.Zero,
            SMTO_ABORTIFHUNG, 1000, out result) != IntPtr.Zero;
    }

    public static bool RequestClose(IntPtr hWnd)
    {
        const uint WM_CLOSE = 0x0010;
        return PostMessage(hWnd, WM_CLOSE, IntPtr.Zero, IntPtr.Zero);
    }
}
'@ | Out-Null

    $phase = 'EXECUTABLE_VALIDATION'
    if ([string]::IsNullOrWhiteSpace($ExecutablePath)) {
        $projectRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
        $ExecutablePath = Join-Path $projectRoot 'FlashToSpineLauncher.exe'
    }
    $executableName = [System.IO.Path]::GetFileName($ExecutablePath)
    if ($executableName -notmatch '^[A-Za-z0-9._-]+\.exe$') {
        Set-ProbeFailure 'EXECUTABLE_NAME_INVALID'
        throw 'controlled probe stop'
    }
    $executable = Get-Item -LiteralPath $ExecutablePath -Force
    if ($executable.PSIsContainer) {
        Set-ProbeFailure 'EXECUTABLE_NOT_A_FILE'
        throw 'controlled probe stop'
    }
    $isReparsePoint = [bool]($executable.Attributes -band [System.IO.FileAttributes]::ReparsePoint)
    if ($isReparsePoint) {
        Set-ProbeFailure 'EXECUTABLE_REPARSE_POINT_REJECTED'
        throw 'controlled probe stop'
    }
    $executableSha256 = (Get-FileHash -LiteralPath $executable.FullName -Algorithm SHA256).Hash.ToLowerInvariant()

    $phase = 'STARTUP'
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $executable.FullName
    $startInfo.WorkingDirectory = $executable.DirectoryName
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) {
        Set-ProbeFailure 'PROCESS_START_FAILED'
        throw 'controlled probe stop'
    }

    Start-Sleep -Seconds $waitSeconds
    if (Test-ProcessHasExited $process) {
        Set-ProbeFailure 'PROCESS_EXITED_DURING_STARTUP'
        throw 'controlled probe stop'
    }

    $phase = 'WINDOW_ENUMERATION'
    $topWindow = [F2sWebView2ProbeNative]::FindExpectedTopLevel(
        [uint32]$process.Id,
        $expectedClass,
        $expectedTitle)
    $topLevelMatched = $null -ne $topWindow
    if (-not $topLevelMatched) {
        Set-ProbeFailure 'EXPECTED_TOP_LEVEL_WINDOW_NOT_FOUND'
        throw 'controlled probe stop'
    }

    $descendantClasses = [F2sWebView2ProbeNative]::EnumerateDescendantClasses($topWindow.Handle)
    foreach ($className in $descendantClasses) {
        if ($className -like 'Chrome_WidgetWin_*') { $chromeWidgetChildFound = $true }
        if ($className -eq 'Chrome_RenderWidgetHostHWND') { $renderWidgetChildFound = $true }
    }
    if (-not $chromeWidgetChildFound) {
        Set-ProbeFailure 'CHROME_WIDGET_CHILD_NOT_FOUND'
    }
    elseif (-not $renderWidgetChildFound) {
        Set-ProbeFailure 'RENDER_WIDGET_CHILD_NOT_FOUND'
    }

    $phase = 'RESPONSIVENESS'
    $process.Refresh()
    $processResponding = [bool]$process.Responding -and
        [F2sWebView2ProbeNative]::IsResponding($topWindow.Handle)
    if (-not $processResponding) {
        Set-ProbeFailure 'PROCESS_NOT_RESPONDING'
    }

    $phase = 'GRACEFUL_SHUTDOWN'
    $wmCloseSent = [F2sWebView2ProbeNative]::RequestClose($topWindow.Handle)
    if (-not $wmCloseSent) {
        Set-ProbeFailure 'WM_CLOSE_SEND_FAILED'
    }
    elseif ($process.WaitForExit(5000)) {
        $gracefulExit = $true
        $cleanupSucceeded = $true
    }
    else {
        Set-ProbeFailure 'WM_CLOSE_TIMEOUT'
    }
}
catch {
    if ($failureCode -eq 'NONE') {
        Set-ProbeFailure ($phase + '_FAILED')
    }
}
finally {
    if ($null -ne $process -and -not (Test-ProcessHasExited $process)) {
        if (-not $wmCloseSent -and $null -ne $topWindow) {
            try {
                $wmCloseSent = [F2sWebView2ProbeNative]::RequestClose($topWindow.Handle)
                if ($wmCloseSent -and $process.WaitForExit(5000)) {
                    $gracefulExit = $true
                    $cleanupSucceeded = $true
                }
            }
            catch {
                Set-ProbeFailure 'FINAL_WM_CLOSE_FAILED'
            }
        }

        if (-not (Test-ProcessHasExited $process)) {
            $forcedCleanupRequired = $true
            try {
                $taskkill = Join-Path $env:SystemRoot 'System32\taskkill.exe'
                & $taskkill /PID $process.Id /T /F 2>$null | Out-Null
                $cleanupSucceeded = $process.WaitForExit(5000)
            }
            catch {
                $cleanupSucceeded = Test-ProcessHasExited $process
                Set-ProbeFailure 'FORCED_CLEANUP_FAILED'
            }
        }
    }
    elseif ($null -ne $process) {
        $cleanupSucceeded = $true
    }

    if (-not $cleanupSucceeded -and $null -ne $process) {
        Set-ProbeFailure 'PROCESS_CLEANUP_UNCONFIRMED'
    }
}

$passed = $failureCode -eq 'NONE' -and
    $topLevelMatched -and
    $chromeWidgetChildFound -and
    $renderWidgetChildFound -and
    $processResponding -and
    $wmCloseSent -and
    $gracefulExit -and
    $cleanupSucceeded -and
    -not $isReparsePoint

$report = [ordered]@{
    schemaVersion = 'f2s-webview2-local-startup-probe/1.0.0'
    status = $(if ($passed) { 'PASS' } else { 'FAIL' })
    runtimeScope = 'LOCAL_RUNTIME_ONLY'
    cleanVm = $false
    failureCode = $(if ($passed) { 'NONE' } else { $failureCode })
    executable = [ordered]@{
        name = $executableName
        sha256 = $executableSha256
        reparsePoint = $isReparsePoint
    }
    startup = [ordered]@{
        hidden = $true
        waitSeconds = $waitSeconds
    }
    windowContract = [ordered]@{
        expectedTopLevelClass = $expectedClass
        expectedTopLevelTitle = $expectedTitle
        topLevelMatched = $topLevelMatched
        chromeWidgetChildFound = $chromeWidgetChildFound
        renderWidgetChildFound = $renderWidgetChildFound
        processResponding = $processResponding
    }
    shutdown = [ordered]@{
        wmCloseSent = $wmCloseSent
        gracefulExit = $gracefulExit
        forcedCleanupRequired = $forcedCleanupRequired
        cleanupSucceeded = $cleanupSucceeded
    }
}

if ($Output -eq 'JSON') {
    $report | ConvertTo-Json -Depth 6 -Compress
}
else {
    Write-Output ('WebView2 local startup probe: ' + $report.status)
    Write-Output ('Scope: LOCAL_RUNTIME_ONLY; clean VM: false')
    Write-Output ('Executable: ' + $report.executable.name + '; SHA-256: ' + $report.executable.sha256)
    Write-Output ('Failure code: ' + $report.failureCode)
}

if (-not $passed) { exit 1 }

