[CmdletBinding()]
param([string]$WorkspaceRoot)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
}
$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$devRoot = Join-Path $workspace 'plan\devplan'
$planRoot = Join-Path $workspace 'plan'
$files = @(Get-ChildItem -LiteralPath $devRoot -File -Filter '*.md' | Where-Object Name -match '^(0[1-9]|1[0-2])-' | Sort-Object Name)
$baselineFiles = @(Get-ChildItem -LiteralPath $planRoot -File -Filter '21-*.md')
if ($files.Count -ne 12) { throw "Expected 12 milestone documents, found $($files.Count)." }
if ($baselineFiles.Count -ne 1) { throw "Expected one plan/21 file, found $($baselineFiles.Count)." }

$baselineText = Get-Content -LiteralPath $baselineFiles[0].FullName -Raw -Encoding UTF8
$taskPattern = '(?m)^#{2,4}\s+(?:[0-9]+\.[0-9]+\s+)?(F2S-DEV-M[0-9]{2}-[0-9]{3})\b[^\r\n]*$'
$wuPattern = '(?m)^#{4,6}\s+(F2S-WU-M[0-9]{2}-[0-9]{3}-[0-9]{2})\b[^\r\n]*$'
$testPattern = 'F2S-TST-[A-Z0-9]+(?:-[A-Z0-9]+)*-[0-9]{3}|F2S-TST-[0-9]{3}'

function Get-UniqueMatches([string]$Text, [string]$Pattern) {
    return @([regex]::Matches($Text, $Pattern) | ForEach-Object Value | Sort-Object -Unique)
}

function Get-FieldValue([string]$Text, [string]$Name) {
    $match = [regex]::Match($Text, '(?mi)^\s*-\s*' + [regex]::Escape($Name) + '\s*[:\uFF1A]\s*([^\r\n]+)$')
    if ($match.Success) { return $match.Groups[1].Value.Trim() }
    return ''
}

$taskRecords = @()
$taskTexts = @{}
foreach ($file in $files) {
    $text = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
    $matches = [regex]::Matches($text, $taskPattern)
    for ($i = 0; $i -lt $matches.Count; $i++) {
        $start = $matches[$i].Index
        $end = if ($i + 1 -lt $matches.Count) { $matches[$i + 1].Index } else { $text.Length }
        $afterHeading = $matches[$i].Index + $matches[$i].Length
        $nextLevelTwo = [regex]::Match($text.Substring($afterHeading), '(?m)^##\s+')
        if ($nextLevelTwo.Success) {
            $candidate = $afterHeading + $nextLevelTwo.Index
            if ($candidate -lt $end) { $end = $candidate }
        }
        $section = $text.Substring($start, $end - $start)
        $id = $matches[$i].Groups[1].Value
        $taskTexts[$id] = $section
        $rawTitle = $matches[$i].Value -replace '^#{2,4}\s+(?:[0-9]+\.[0-9]+\s+)?F2S-DEV-M[0-9]{2}-[0-9]{3}\s*', ''
        $wuMatches = [regex]::Matches($section, $wuPattern)
        $wuRecords = @()
        $allWrites = @()
        $allLocks = @()
        $allDeps = @()
        for ($j = 0; $j -lt $wuMatches.Count; $j++) {
            $wuStart = $wuMatches[$j].Index
            $wuEnd = if ($j + 1 -lt $wuMatches.Count) { $wuMatches[$j + 1].Index } else { $section.Length }
            $wuText = $section.Substring($wuStart, $wuEnd - $wuStart)
            $writes = Get-FieldValue $wuText 'writes'
            $lock = Get-FieldValue $wuText 'parallelSafety'
            $deps = @(Get-UniqueMatches (Get-FieldValue $wuText 'dependsOn') 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
            $allWrites += $writes
            $allLocks += $lock
            $allDeps += $deps
            $wuRecords += [ordered]@{
                id = $wuMatches[$j].Groups[1].Value
                writes = $writes
                dependsOn = @($deps)
                parallelSafety = $lock
                estimate = Get-FieldValue $wuText 'estimate'
            }
        }
        $suffix = $id.Substring('F2S-DEV-'.Length)
        $declaredDependencyMatch = [regex]::Match($section, '(?m)^.*?\u4E0A\u6E38(?:\s+DEV)?\s*[:\uFF1A]\s*([^;\uFF1B\r\n]+)')
        $declaredDependencies = @()
        if ($declaredDependencyMatch.Success) {
            $declaredDependencies = @(Get-UniqueMatches $declaredDependencyMatch.Groups[1].Value 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
        }
        if ($declaredDependencies.Count -eq 0) { $declaredDependencies = @($allDeps | Sort-Object -Unique) }
        $taskRecords += [ordered]@{
            id = $id
            milestone = $suffix.Substring(0, 3)
            title = $rawTitle
            file = $file.Name
            requirements = @(Get-UniqueMatches $section 'F2S-(?:FR|NFR)-[A-Z0-9-]+')
            tests = @(Get-UniqueMatches $section $testPattern)
            evd = 'F2S-EVD-' + $suffix
            dependencies = @($declaredDependencies)
            workUnitDependencies = @($allDeps | Sort-Object -Unique)
            writes = @($allWrites | Where-Object { $_ -ne '' } | Sort-Object -Unique)
            locks = @($allLocks | Where-Object { $_ -ne '' } | Sort-Object -Unique)
            workUnits = @($wuRecords)
        }
    }
}

$requirementRows = @()
$seenRequirements = @{}
foreach ($line in ($baselineText -split '[\r\n]+')) {
    if ($line -match '^\|\s*(F2S-(?:FR|NFR)-[A-Z0-9-]+)\s*\|\s*(P[012])\s*\|') {
        $id = $matches[1]
        if (-not $seenRequirements.ContainsKey($id)) {
            $seenRequirements[$id] = $true
            $responsibleDevs = @($taskRecords | Where-Object { $_.requirements -ccontains $id } | ForEach-Object { $_.id } | Sort-Object -Unique)
            $requirementRows += [ordered]@{
                id = $id
                priority = $matches[2]
                devs = @($responsibleDevs)
                tests = @(Get-UniqueMatches $line $testPattern)
            }
        }
    }
}

$baselineTests = @(Get-UniqueMatches $baselineText $testPattern)
$baselineLines = @($baselineText -split '[\r\n]+')
$testRows = @()
foreach ($testId in $baselineTests) {
    $devs = @()
    $evds = @()
    $exactPattern = [regex]::Escape($testId) + '(?![A-Z0-9-])'
    foreach ($task in $taskRecords) {
        if ($taskTexts[$task.id] -match $exactPattern) { $devs += $task.id }
    }
    $devs = @($devs | Sort-Object -Unique)
    foreach ($dev in $devs) { $evds += $dev.Replace('F2S-DEV-', 'F2S-EVD-') }
    $testRows += [ordered]@{
        id = $testId
        devs = @($devs)
        evds = @($evds | Sort-Object -Unique)
    }
}

$sourceHashes = @()
foreach ($file in $files) {
    $sourceHashes += [ordered]@{
        file = $file.Name
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}
$wuCount = 0
foreach ($task in $taskRecords) { $wuCount += @($task.workUnits).Count }

[ordered]@{
    tasks = @($taskRecords)
    requirements = @($requirementRows)
    tests = @($testRows)
    sourceHashes = @($sourceHashes)
    counts = [ordered]@{
        files = $files.Count
        tasks = $taskRecords.Count
        workUnits = $wuCount
        requirements = $requirementRows.Count
        tests = $testRows.Count
    }
} | ConvertTo-Json -Depth 20 -Compress
