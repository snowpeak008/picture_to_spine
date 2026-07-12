[CmdletBinding()]
param([string]$WorkspaceRoot)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
}
$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$tracePath = Join-Path $workspace 'plan\devplan\13-*.md'
$traceFile = @(Get-ChildItem -Path $tracePath -File)
if ($traceFile.Count -ne 1) { throw "Expected one trace document, found $($traceFile.Count)." }
$exporter = Join-Path $PSScriptRoot 'Export-DevPlanTraceData.ps1'
$data = (& powershell -NoProfile -ExecutionPolicy Bypass -File $exporter -WorkspaceRoot $workspace) | ConvertFrom-Json
$text = Get-Content -LiteralPath $traceFile[0].FullName -Raw -Encoding UTF8

function Join-Cell([object[]]$Values) {
    $clean = @($Values | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
    if ($clean.Count -eq 0) { return '-' }
    return $clean -join '<br>'
}

function Clean-Field([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return '-' }
    return $Value.Trim().TrimEnd('.', [char]0x3002)
}

$hashHeading = [regex]::Match($text, '(?m)^### 1\.1[^\r\n]*').Value
if ([string]::IsNullOrWhiteSpace($hashHeading)) { $hashHeading = '### 1.1 Input file hashes' }
$hashLines = @($hashHeading, '', '| File | SHA-256 |', '| --- | --- |')
foreach ($source in @($data.sourceHashes)) {
    $hashLines += '| ' + $source.file + ' | `' + $source.sha256 + '` |'
}
$hashSection = ($hashLines -join [Environment]::NewLine) + [Environment]::NewLine
if ($text -match '(?m)^### 1\.1') {
    $text = [regex]::Replace($text, '(?s)### 1\.1.*?(?=\r?\n## 2\.)', [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $hashSection })
} else {
    $text = [regex]::Replace($text, '(?s)\| File \| SHA-256 \|.*?(?=\r?\n## 2\.)', [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $hashSection })
}

$heading3 = [regex]::Match($text, '(?m)^## 3\.[^\r\n]*').Value
$heading4 = [regex]::Match($text, '(?m)^## 4\.[^\r\n]*').Value
$heading5 = [regex]::Match($text, '(?m)^## 5\.[^\r\n]*').Value
$heading6 = [regex]::Match($text, '(?m)^## 6\.[^\r\n]*').Value
if ([string]::IsNullOrWhiteSpace($heading3)) { $heading3 = '## 3. DEV-WU-REQ-TST-EVD matrix' }
if ([string]::IsNullOrWhiteSpace($heading4)) { $heading4 = '## 4. Write paths, locks, and WU reverse index' }
if ([string]::IsNullOrWhiteSpace($heading5)) { $heading5 = '## 5. 102 Requirement reverse trace' }
if ([string]::IsNullOrWhiteSpace($heading6)) { $heading6 = '## 6. 133 exact test reverse trace' }

$lines = @($heading3, '', 'Generated from the exact task-card edge sets in documents 01-12. Manual edge additions are forbidden.', '', '| DEV / short name | File | Complete WU | FR/NFR | Exact tests | EVD | dependsOn DEV | State |', '| --- | --- | --- | --- | --- | --- | --- | --- |')
foreach ($task in @($data.tasks)) {
    $title = ([string]$task.title) -replace '^[^\p{L}\p{N}]+', ''
    $lines += '| ' + $task.id + '<br>' + $title + ' | ' + $task.file + ' | ' + (Join-Cell @($task.workUnits.id)) + ' | ' + (Join-Cell @($task.requirements)) + ' | ' + (Join-Cell @($task.tests)) + ' | ' + $task.evd + ' | ' + (Join-Cell @($task.dependencies)) + ' | planned / UNVERIFIED |'
}

$lines += @('', $heading4, '', 'Generated from the same work-unit records used by section 3.', '', '| DEV | WU count | Exact writes | Locks |', '| --- | ---: | --- | --- |')
foreach ($task in @($data.tasks)) {
    $lines += '| ' + $task.id + ' | ' + @($task.workUnits).Count + ' | ' + (Join-Cell @($task.writes)) + ' | ' + (Join-Cell @($task.locks | ForEach-Object { Clean-Field $_ })) + ' |'
}
$lines += @('', '### WU execution index', '')
foreach ($task in @($data.tasks)) {
    $wuParts = @()
    foreach ($wu in @($task.workUnits)) {
        $wuParts += $wu.id + ' [' + (Clean-Field $wu.estimate) + '; ' + (Clean-Field $wu.parallelSafety) + ']'
    }
    $lines += '- ' + $task.id + ': ' + ($wuParts -join '; ')
}

$lines += @('', $heading5, '', 'Responsible DEV edges are the exact reverse of section 3.', '', '| Requirement | Priority | Responsible DEV | Exact tests |', '| --- | --- | --- | --- |')
foreach ($req in @($data.requirements)) {
    $lines += '| ' + $req.id + ' | ' + $req.priority + ' | ' + (Join-Cell @($req.devs)) + ' | ' + (Join-Cell @($req.tests)) + ' |'
}

$lines += @('', $heading6, '', 'Responsible DEV and EVD edges are the exact reverse of section 3.', '', '| Exact test | Responsible DEV | Evidence | Initial state |', '| --- | --- | --- | --- |')
foreach ($test in @($data.tests)) {
    $lines += '| ' + $test.id + ' | ' + (Join-Cell @($test.devs)) + ' | ' + (Join-Cell @($test.evds)) + ' | NOT_RUN / UNVERIFIED |'
}

$generated = ($lines -join [Environment]::NewLine) + [Environment]::NewLine
if ($text -match '(?m)^## 3\.') {
    $text = [regex]::Replace($text, '(?s)## 3\..*?(?=\r?\n## 7\.)', [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $generated })
} else {
    $text = [regex]::Replace($text, '(?s)(\|\s*M11\s*\|[^\r\n]*\r?\n).*?(?=\r?\n## 7\.)', [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $m.Groups[1].Value + [Environment]::NewLine + $generated })
}
$writer = New-Object IO.StreamWriter($traceFile[0].FullName, $false, (New-Object Text.UTF8Encoding($false)))
try { $writer.Write($text) } finally { $writer.Dispose() }

[pscustomobject][ordered]@{
    tracePath = $traceFile[0].FullName
    traceSha256 = (Get-FileHash -LiteralPath $traceFile[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    tasks = [int]$data.counts.tasks
    workUnits = [int]$data.counts.workUnits
    requirements = [int]$data.counts.requirements
    tests = [int]$data.counts.tests
}
