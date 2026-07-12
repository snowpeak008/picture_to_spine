[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('D0', 'FINAL')][string]$Phase,
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Z0-9-]+$')][string]$SnapshotId,
    [Parameter(Mandatory = $true)][ValidatePattern('^F2S-AUD-DEVPLAN-(D0|FINAL)-[0-9]{3}$')][string]$EvidenceId,
    [string]$WorkspaceRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (($Phase -ceq 'D0' -and $EvidenceId -notmatch '-D0-') -or ($Phase -ceq 'FINAL' -and $EvidenceId -notmatch '-FINAL-')) {
    throw "EvidenceId phase does not match Phase: $EvidenceId / $Phase"
}

function Get-Sha256([string]$LiteralPath) {
    return (Get-FileHash -LiteralPath $LiteralPath -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-RelativePath([string]$Root, [string]$Path) {
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    $pathFull = [IO.Path]::GetFullPath($Path)
    if (-not $pathFull.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes workspace: $pathFull"
    }
    return $pathFull.Substring($rootFull.Length).Replace('\', '/')
}

function Write-Utf8New([string]$LiteralPath, [string]$Text) {
    if (Test-Path -LiteralPath $LiteralPath) {
        throw "Immutable output already exists: $LiteralPath"
    }
    $parent = Split-Path -Parent $LiteralPath
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent | Out-Null
    }
    [IO.File]::WriteAllText($LiteralPath, $Text, [Text.UTF8Encoding]::new($false))
}

function Get-FrontMatterScalar([string]$Text, [string]$Name) {
    $match = [regex]::Match($Text, '(?m)^' + [regex]::Escape($Name) + ':\s*([^\r\n]+)\s*$')
    if (-not $match.Success) { return $null }
    return $match.Groups[1].Value.Trim()
}

function Get-FrontMatterList([string]$Text, [string]$Name) {
    $raw = Get-FrontMatterScalar $Text $Name
    if ($null -eq $raw -or $raw -notmatch '^\[(.*)\]$') { return @() }
    if ([string]::IsNullOrWhiteSpace($matches[1])) { return @() }
    return @($matches[1].Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
}

function Get-UniqueMatches([string]$Text, [string]$Pattern) {
    return @([regex]::Matches($Text, $Pattern) | ForEach-Object { $_.Value } | Sort-Object -Unique)
}

function Get-CycleNodes([string[]]$Nodes, [hashtable]$Edges) {
    $remaining = @{}
    foreach ($node in $Nodes) { $remaining[$node] = $true }
    do {
        $removable = @()
        foreach ($node in @($remaining.Keys)) {
            $incoming = @($Edges[$node] | Where-Object { $remaining.ContainsKey($_) })
            if ($incoming.Count -eq 0) { $removable += $node }
        }
        foreach ($node in $removable) { $remaining.Remove($node) }
    } while ($removable.Count -gt 0)
    return @($remaining.Keys | Sort-Object)
}

function Add-Check([string]$CheckId, [bool]$Pass, [object]$Metric, [string[]]$Details) {
    $status = if ($Pass) { 'PASS' } else { 'FAIL' }
    $rawName = $CheckId.ToLowerInvariant() + '.json'
    $rawStaging = Join-Path $script:RawStagingRoot $rawName
    $rawFinal = Join-Path $script:RawFinalRoot $rawName
    $raw = [ordered]@{
        schemaVersion = '1.0.0'
        evidenceId = $EvidenceId
        snapshotId = $SnapshotId
        phase = $Phase
        checkId = $CheckId
        status = $status
        metric = $Metric
        details = @($Details)
        command = $script:ExactCommand
        toolPath = Get-RelativePath $script:Workspace $MyInvocation.ScriptName
        toolSha256 = Get-Sha256 $MyInvocation.ScriptName
    }
    Write-Utf8New $rawStaging (($raw | ConvertTo-Json -Depth 40) + [Environment]::NewLine)
    $script:Results += [pscustomobject][ordered]@{
        checkId = $CheckId
        status = $status
        metric = $Metric
        details = @($Details)
        rawPath = Get-RelativePath $script:Workspace $rawFinal
        rawSha256 = Get-Sha256 $rawStaging
    }
}

if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
}
$script:Workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$devPlanRoot = Join-Path $script:Workspace 'plan\devplan'
$reviewRoot = Join-Path $devPlanRoot 'reviews'
$snapshotRoot = Join-Path (Join-Path $reviewRoot 'snapshots') $SnapshotId
$snapshotDocsRoot = Join-Path $snapshotRoot 'devplan'
$manifestPath = Join-Path $snapshotRoot 'manifest.json'
$archivePath = Join-Path (Join-Path $reviewRoot 'snapshots') ($SnapshotId + '.zip')
$auditRoot = Join-Path (Join-Path $reviewRoot 'audits') $SnapshotId
$stagingRoot = Join-Path (Join-Path $reviewRoot 'audits') ('.staging-' + $SnapshotId + '-' + [guid]::NewGuid().ToString('N'))
$script:RawStagingRoot = Join-Path $stagingRoot 'raw'
$script:RawFinalRoot = Join-Path $auditRoot 'raw'
$script:ExactCommand = "powershell -NoProfile -ExecutionPolicy Bypass -File plan/devplan/reviews/tools/Invoke-DevPlanAudit.ps1 -Phase $Phase -SnapshotId $SnapshotId -EvidenceId $EvidenceId"
$script:Results = @()

if (Test-Path -LiteralPath $auditRoot) { throw "Audit already exists and is immutable: $auditRoot" }
New-Item -ItemType Directory -Path $script:RawStagingRoot | Out-Null

$expectedFiles = [ordered]@{
    '00' = 'F2S-DOC-DEVPLAN-INDEX-001'
    '01' = 'F2S-DOC-DEVPLAN-M00-001'
    '02' = 'F2S-DOC-DEVPLAN-M01-001'
    '03' = 'F2S-DOC-DEVPLAN-M02-001'
    '04' = 'F2S-DOC-DEVPLAN-M03-001'
    '05' = 'F2S-DOC-DEVPLAN-M04-001'
    '06' = 'F2S-DOC-DEVPLAN-M05-001'
    '07' = 'F2S-DOC-DEVPLAN-M06-001'
    '08' = 'F2S-DOC-DEVPLAN-M07-001'
    '09' = 'F2S-DOC-DEVPLAN-M08-001'
    '10' = 'F2S-DOC-DEVPLAN-M09-001'
    '11' = 'F2S-DOC-DEVPLAN-M10-001'
    '12' = 'F2S-DOC-DEVPLAN-M11-001'
    '13' = 'F2S-DOC-DEVPLAN-TRACE-001'
    '14' = 'F2S-DOC-DEVPLAN-SCORE-001'
    '15' = 'F2S-DOC-DEVPLAN-COMPLIANCE-001'
}
$expectedCanonicalFor = [ordered]@{
    '00' = @('F2S-DEVPLAN-GOV-001', 'F2S-DEVPLAN-TEMPLATE-001', 'F2S-DEVPLAN-GATE-001')
    '01' = @('F2S-DEVPLAN-M00-CARDS-001', 'F2S-WU-M00')
    '02' = @('F2S-DEVPLAN-M01-CARDS-001', 'F2S-WU-M01')
    '03' = @('F2S-DEVPLAN-M02-CARDS-001', 'F2S-WU-M02')
    '04' = @('F2S-DEVPLAN-M03-CARDS-001', 'F2S-WU-M03')
    '05' = @('F2S-DEVPLAN-M04-CARDS-001', 'F2S-WU-M04')
    '06' = @('F2S-DEVPLAN-M05-CARDS-001', 'F2S-WU-M05')
    '07' = @('F2S-DEVPLAN-M06-CARDS-001', 'F2S-WU-M06')
    '08' = @('F2S-DEVPLAN-M07-CARDS-001', 'F2S-WU-M07')
    '09' = @('F2S-DEVPLAN-M08-CARDS-001', 'F2S-WU-M08')
    '10' = @('F2S-DEVPLAN-M09-CARDS-001', 'F2S-WU-M09')
    '11' = @('F2S-DEVPLAN-M10-CARDS-001', 'F2S-WU-M10')
    '12' = @('F2S-DEVPLAN-M11-CARDS-001', 'F2S-WU-M11')
    '13' = @('F2S-DEVPLAN-TRACE-001', 'F2S-DEVPLAN-DAG-001', 'F2S-DEVPLAN-REQ-TEST-001', 'F2S-DEVPLAN-PATH-OWNER-001')
    '14' = @('F2S-DEVPLAN-SCORE-001', 'F2S-DEVPLAN-REV-001')
    '15' = @('F2S-DEVPLAN-COMP-001', 'F2S-DEVPLAN-AUTH-001')
}

$manifest = $null
$manifestIssues = @()
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) { $manifestIssues += 'manifest.json is missing.' }
if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf)) { $manifestIssues += 'snapshot archive is missing.' }
if ($manifestIssues.Count -eq 0) {
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($manifest.snapshotId -cne $SnapshotId) { $manifestIssues += 'Manifest snapshotId mismatch.' }
    if ($manifest.phase -cne $Phase) { $manifestIssues += 'Manifest phase mismatch.' }
    if ([int]$manifest.documentCount -ne 16) { $manifestIssues += 'Manifest documentCount is not 16.' }
    if ($manifest.archiveSha256 -cne (Get-Sha256 $archivePath)) { $manifestIssues += 'Archive SHA-256 mismatch.' }
    foreach ($document in @($manifest.documents)) {
        $path = Join-Path $snapshotRoot ($document.path.Replace('/', '\'))
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { $manifestIssues += "Missing snapshot document: $($document.path)"; continue }
        if ($document.sha256 -cne (Get-Sha256 $path)) { $manifestIssues += "Document SHA-256 mismatch: $($document.path)" }
        if ([string]$document.size -cne [string](Get-Item -LiteralPath $path).Length) { $manifestIssues += "Document size mismatch: $($document.path)" }
    }
}
Add-Check 'SNAPSHOT-INTEGRITY' ($manifestIssues.Count -eq 0) ([ordered]@{
    manifestSha256 = if (Test-Path -LiteralPath $manifestPath) { Get-Sha256 $manifestPath } else { $null }
    archiveSha256 = if (Test-Path -LiteralPath $archivePath) { Get-Sha256 $archivePath } else { $null }
    documentCount = if ($null -ne $manifest) { [int]$manifest.documentCount } else { 0 }
}) $manifestIssues

$docTexts = @{}
$docIds = @{}
$registryIssues = @()
$actualFiles = @(Get-ChildItem -LiteralPath $snapshotDocsRoot -File -Filter '*.md' | Sort-Object Name)
$actualPrefixes = @($actualFiles | ForEach-Object { if ($_.Name -match '^([0-9]{2})-') { $matches[1] } else { '__' } })
$missingFiles = @($expectedFiles.Keys | Where-Object { $_ -cnotin $actualPrefixes })
$extraFiles = @($actualFiles | Where-Object { $_.Name -notmatch '^([0-9]{2})-' -or -not $expectedFiles.Contains($matches[1]) } | ForEach-Object Name)
if ($missingFiles.Count -gt 0) { $registryIssues += 'Missing files: ' + ($missingFiles -join ', ') }
if ($extraFiles.Count -gt 0) { $registryIssues += 'Extra files: ' + ($extraFiles -join ', ') }
$expectedStatus = if ($Phase -ceq 'FINAL') { 'reviewed' } else { 'draft' }
foreach ($file in $actualFiles) {
    $text = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
    $docTexts[$file.Name] = $text
    $docId = Get-FrontMatterScalar $text 'doc_id'
    $docIds[$file.Name] = $docId
    $prefix = if ($file.Name -match '^([0-9]{2})-') { $matches[1] } else { '__' }
    if ($expectedFiles.Contains($prefix) -and $docId -cne $expectedFiles[$prefix]) { $registryIssues += "doc_id mismatch in $($file.Name)." }
    if ((Get-FrontMatterScalar $text 'status') -cne $expectedStatus) { $registryIssues += "status mismatch in $($file.Name); expected $expectedStatus." }
    if ((Get-FrontMatterScalar $text 'revision') -notmatch '^\d+\.\d+$') { $registryIssues += "Invalid revision in $($file.Name)." }
    if ((Get-FrontMatterScalar $text 'last_verified') -cne '2026-07-11') { $registryIssues += "last_verified mismatch in $($file.Name)." }
    $score = Get-FrontMatterScalar $text 'review_score_ref'
    if ($Phase -ceq 'FINAL' -and $score -notmatch '-R1$') { $registryIssues += "Final score ref is not R1 in $($file.Name)." }
    if ($Phase -ceq 'D0' -and $score -notmatch '-D0$') { $registryIssues += "D0 score ref is not D0 in $($file.Name)." }
    $canonicalFor = @(Get-FrontMatterList $text 'canonical_for')
    if ($canonicalFor.Count -eq 0) { $registryIssues += "canonical_for is empty in $($file.Name)." }
    if ($expectedCanonicalFor.Contains($prefix)) {
        $expectedCanonical = @($expectedCanonicalFor[$prefix])
        $missingCanonical = @($expectedCanonical | Where-Object { $_ -cnotin $canonicalFor })
        $extraCanonical = @($canonicalFor | Where-Object { $_ -cnotin $expectedCanonical })
        if ($missingCanonical.Count -gt 0 -or $extraCanonical.Count -gt 0) {
            $registryIssues += "canonical_for mismatch in $($file.Name); missing=[$($missingCanonical -join ',')], extra=[$($extraCanonical -join ',')]."
        }
    }
}
$duplicateDocIds = @($docIds.Values | Group-Object | Where-Object Count -gt 1 | ForEach-Object Name)
if ($duplicateDocIds.Count -gt 0) { $registryIssues += 'Duplicate doc_id values: ' + ($duplicateDocIds -join ', ') }
Add-Check 'DOCUMENT-REGISTRY-FRONTMATTER' ($registryIssues.Count -eq 0) ([ordered]@{
    actual = $actualFiles.Count
    expected = 16
    missing = $missingFiles
    extra = $extraFiles
    duplicateDocIds = $duplicateDocIds
    expectedStatus = $expectedStatus
}) $registryIssues

$docDagIssues = @()
$knownDocIds = @($docIds.Values)
$docEdges = @{}
foreach ($file in $actualFiles) {
    $id = $docIds[$file.Name]
    $deps = @(Get-FrontMatterList $docTexts[$file.Name] 'depends_on')
    $docEdges[$id] = @($deps)
    foreach ($dep in $deps) { if ($dep -cnotin $knownDocIds) { $docDagIssues += "Dangling document edge: $id -> $dep" } }
}
$docCycles = @(Get-CycleNodes $knownDocIds $docEdges)
if ($docCycles.Count -gt 0) { $docDagIssues += 'Document dependency cycle: ' + ($docCycles -join ', ') }
Add-Check 'DOCUMENT-DEPENDENCY-DAG' ($docDagIssues.Count -eq 0) ([ordered]@{ nodes = $knownDocIds.Count; cycleNodes = $docCycles }) $docDagIssues

$upstreamIssues = @()
$upstreamManifestPath = Join-Path $script:Workspace 'plan\reviews\snapshots\R3B-20260711-085312-FINAL\manifest.json'
$upstreamReviewPath = Join-Path $script:Workspace 'plan\reviews\2026-07-11-R3b-final-review.md'
$baselineCandidates = @(Get-ChildItem -LiteralPath (Join-Path $script:Workspace 'plan') -File -Filter '21-*.md')
$baselinePath = if ($baselineCandidates.Count -eq 1) { $baselineCandidates[0].FullName } else { Join-Path $script:Workspace 'plan\__missing_plan21.md' }
$expectedUpstreamManifestSha = '7763d20d4f46c5d62b249bc8a080a761cfbe8390cab7d0a55748743af0cc46ae'
$expectedUpstreamReviewSha = '4d6d7b7699be3b02577b464f8030ae5ad32aafa0b9dcbcb48c56a6a8c9fd53fd'
if ($baselineCandidates.Count -ne 1) { $upstreamIssues += "Expected one live plan/21 file, found $($baselineCandidates.Count)." }
foreach ($pair in @(@($upstreamManifestPath, $expectedUpstreamManifestSha), @($upstreamReviewPath, $expectedUpstreamReviewSha))) {
    if (-not (Test-Path -LiteralPath $pair[0] -PathType Leaf)) { $upstreamIssues += "Missing upstream artifact: $($pair[0])" }
    elseif ((Get-Sha256 $pair[0]) -cne $pair[1]) { $upstreamIssues += "Upstream SHA-256 mismatch: $($pair[0])" }
}
if (-not (Test-Path -LiteralPath $baselinePath -PathType Leaf)) { $upstreamIssues += 'Live frozen plan 21 is missing.' }
if ($upstreamIssues.Count -eq 0) {
    $upstreamManifest = Get-Content -LiteralPath $upstreamManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $plan21Entry = @($upstreamManifest.documents | Where-Object { $_.path -like 'plan/21-*' })
    if ($plan21Entry.Count -ne 1) { $upstreamIssues += 'Upstream manifest does not contain exactly one plan/21 entry.' }
    elseif ($plan21Entry[0].sha256 -cne (Get-Sha256 $baselinePath)) { $upstreamIssues += 'Live plan 21 drifted from R3b snapshot.' }
}
Add-Check 'UPSTREAM-R3B-BINDING' ($upstreamIssues.Count -eq 0) ([ordered]@{
    manifestSha256 = if (Test-Path $upstreamManifestPath) { Get-Sha256 $upstreamManifestPath } else { $null }
    reviewSha256 = if (Test-Path $upstreamReviewPath) { Get-Sha256 $upstreamReviewPath } else { $null }
    baselineSha256 = if (Test-Path $baselinePath) { Get-Sha256 $baselinePath } else { $null }
}) $upstreamIssues

$baselineText = if (Test-Path -LiteralPath $baselinePath) { Get-Content -LiteralPath $baselinePath -Raw -Encoding UTF8 } else { '' }
$baselineDev = @(Get-UniqueMatches $baselineText 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
$baselineEvd = @(Get-UniqueMatches $baselineText 'F2S-EVD-M[0-9]{2}-[0-9]{3}')
$baselineReq = @(Get-UniqueMatches $baselineText 'F2S-(?:FR|NFR)-[A-Z0-9-]+')
$testPattern = 'F2S-TST-[A-Z0-9]+(?:-[A-Z0-9]+)*-[0-9]{3}|F2S-TST-[0-9]{3}'
$baselineTests = @(Get-UniqueMatches $baselineText $testPattern)

$taskPattern = '(?m)^#{2,4}\s+(?:[0-9]+\.[0-9]+\s+)?(F2S-DEV-M[0-9]{2}-[0-9]{3})\b[^\r\n]*$'
$wuPattern = '(?m)^#{4,6}\s+(F2S-WU-M[0-9]{2}-[0-9]{3}-[0-9]{2})\b[^\r\n]*$'
$tasks = @()
foreach ($file in $actualFiles | Where-Object { $_.Name -match '^(0[1-9]|1[0-2])-' }) {
    $text = $docTexts[$file.Name]
    $matches = [regex]::Matches($text, $taskPattern)
    for ($i = 0; $i -lt $matches.Count; $i++) {
        $start = $matches[$i].Index
        $end = if ($i + 1 -lt $matches.Count) { $matches[$i + 1].Index } else { $text.Length }
        $afterHeading = $matches[$i].Index + $matches[$i].Length
        $nextLevelTwo = [regex]::Match($text.Substring($afterHeading), '(?m)^##\s+')
        if ($nextLevelTwo.Success) {
            $levelTwoEnd = $afterHeading + $nextLevelTwo.Index
            if ($levelTwoEnd -lt $end) { $end = $levelTwoEnd }
        }
        $tasks += [pscustomobject]@{
            Id = $matches[$i].Groups[1].Value
            File = $file.Name
            Text = $text.Substring($start, $end - $start)
        }
    }
}
$taskIds = @($tasks.Id)
$taskUnique = @($taskIds | Sort-Object -Unique)
$taskDuplicates = @($taskIds | Group-Object | Where-Object Count -gt 1 | ForEach-Object Name)
$missingTasks = @($baselineDev | Where-Object { $_ -cnotin $taskUnique })
$extraTasks = @($taskUnique | Where-Object { $_ -cnotin $baselineDev })
$registryTaskIssues = @()
if ($baselineDev.Count -ne 80) { $registryTaskIssues += "Baseline DEV count is $($baselineDev.Count), expected 80." }
if ($baselineEvd.Count -ne 80) { $registryTaskIssues += "Baseline EVD count is $($baselineEvd.Count), expected 80." }
if ($taskUnique.Count -ne 80) { $registryTaskIssues += "Task heading count is $($taskUnique.Count), expected 80." }
if ($taskDuplicates.Count -gt 0) { $registryTaskIssues += 'Duplicate task headings: ' + ($taskDuplicates -join ', ') }
if ($missingTasks.Count -gt 0) { $registryTaskIssues += 'Missing task headings: ' + ($missingTasks -join ', ') }
if ($extraTasks.Count -gt 0) { $registryTaskIssues += 'Extra task headings: ' + ($extraTasks -join ', ') }
Add-Check 'DEV-REGISTRY-PARITY' ($registryTaskIssues.Count -eq 0) ([ordered]@{
    baselineDev = $baselineDev.Count; baselineEvd = $baselineEvd.Count; taskHeadings = $taskUnique.Count
    missing = $missingTasks; extra = $extraTasks; duplicates = $taskDuplicates
}) $registryTaskIssues

$taskIssues = @()
$requiredTaskFields = @(1..12)
$taskMetrics = @()
$allWus = @()
foreach ($task in $tasks) {
    $fieldMatches = [regex]::Matches($task.Text, '(?m)^\s*(?:#{3,6}\s*)?(1[0-2]|[1-9])\.\s+')
    $fieldNumbers = @($fieldMatches | ForEach-Object { [int]$_.Groups[1].Value })
    foreach ($field in $requiredTaskFields) {
        if (@($fieldNumbers | Where-Object { $_ -eq $field }).Count -ne 1) { $taskIssues += "$($task.Id) must contain task field $field exactly once." }
    }
    if (($fieldNumbers -join ',') -cne ($requiredTaskFields -join ',')) { $taskIssues += "$($task.Id) task fields are not in frozen 1..12 order." }
    if ($task.Text.IndexOf('work units', [StringComparison]::OrdinalIgnoreCase) -lt 0) { $taskIssues += "$($task.Id) field 6 does not use the frozen work units label." }
    $suffix = $task.Id.Substring('F2S-DEV-'.Length)
    $expectedEvd = 'F2S-EVD-' + $suffix
    $expectedEvidencePath = 'evidence/' + $suffix.Substring(0, 3) + '/' + $task.Id + '/'
    if ($task.Text -notmatch [regex]::Escape($expectedEvd)) { $taskIssues += "$($task.Id) missing same-suffix EVD $expectedEvd." }
    if ($task.Text.Replace('\', '/') -notmatch [regex]::Escape($expectedEvidencePath)) { $taskIssues += "$($task.Id) missing canonical evidence path $expectedEvidencePath." }
    if (@(Get-UniqueMatches $task.Text 'F2S-(?:FR|NFR)-[A-Z0-9-]+').Count -eq 0) { $taskIssues += "$($task.Id) has no FR/NFR edge." }
    if (@(Get-UniqueMatches $task.Text $testPattern).Count -eq 0) { $taskIssues += "$($task.Id) has no exact F2S-TST edge." }
    $wuMatches = [regex]::Matches($task.Text, $wuPattern)
    if ($wuMatches.Count -lt 2 -or $wuMatches.Count -gt 6) { $taskIssues += "$($task.Id) has $($wuMatches.Count) WU headings; expected 2..6." }
    $wuIdsForTask = @()
    for ($i = 0; $i -lt $wuMatches.Count; $i++) {
        $start = $wuMatches[$i].Index
        $end = if ($i + 1 -lt $wuMatches.Count) { $wuMatches[$i + 1].Index } else { $task.Text.Length }
        $wuText = $task.Text.Substring($start, $end - $start)
        $wuId = $wuMatches[$i].Groups[1].Value
        $wuIdsForTask += $wuId
        if (-not $wuId.StartsWith('F2S-WU-' + $suffix + '-', [StringComparison]::Ordinal)) { $taskIssues += "$wuId does not belong to $($task.Id)." }
        $allWus += [pscustomobject]@{ Id = $wuId; TaskId = $task.Id; File = $task.File; Text = $wuText }
    }
    $taskMetrics += [pscustomobject]@{ taskId = $task.Id; file = $task.File; wuCount = $wuMatches.Count; evdId = $expectedEvd }
}
Add-Check 'TASK-CARD-STRUCTURE' ($taskIssues.Count -eq 0) ([ordered]@{ tasks = $tasks.Count; requiredFields = $requiredTaskFields; taskMetrics = $taskMetrics }) $taskIssues

$wuIssues = @()
$wuIds = @($allWus.Id)
$wuUnique = @($wuIds | Sort-Object -Unique)
$wuDuplicates = @($wuIds | Group-Object | Where-Object Count -gt 1 | ForEach-Object Name)
if ($wuDuplicates.Count -gt 0) { $wuIssues += 'Duplicate WU headings: ' + ($wuDuplicates -join ', ') }
$requiredWuFields = @('output', 'reads', 'writes', 'steps', 'command', 'tests', 'evidence', 'dependsOn', 'parallelSafety', 'rollback', 'estimate')
$wuEdges = @{}
$devEdges = @{}
foreach ($taskId in $baselineDev) { $devEdges[$taskId] = @() }
$pathOwners = @{}
foreach ($wu in $allWus) {
    foreach ($field in $requiredWuFields) {
        if ($wu.Text -notmatch ('(?mi)^\s*-\s*' + [regex]::Escape($field) + '\s*[:\uFF1A]')) { $wuIssues += "$($wu.Id) missing field: $field" }
    }
    $commandMatches = [regex]::Matches($wu.Text, '(?mi)^\s*-\s*command\s*[:\uFF1A]\s*([^\r\n]+)$')
    $allowedCommands = @('npm ci', 'npm run bootstrap:check', 'npm run build:ai-pack', 'npm run build:core', 'npm run lint', 'npm run release:verify', 'npm run test:integration', 'npm run test:spine', 'npm run typecheck', 'npm test')
    if ($commandMatches.Count -ne 1) {
        $wuIssues += "$($wu.Id) must contain exactly one command field."
    } else {
        $commandValue = $commandMatches[0].Groups[1].Value.Trim().TrimEnd('.', [char]0x3002)
        if ($commandValue -cnotin $allowedCommands -or $commandValue -match '[;\uFF1B&|]') { $wuIssues += "$($wu.Id) command is not one frozen top-level command: $commandValue" }
    }
    $evidenceMatches = [regex]::Matches($wu.Text, '(?mi)^\s*-\s*evidence\s*[:\uFF1A]\s*([^\r\n]+)$')
    $expectedWuEvidencePath = 'evidence/' + $wu.Id.Substring('F2S-WU-'.Length, 3) + '/' + $wu.TaskId + '/' + $wu.Id + '/'
    if ($evidenceMatches.Count -ne 1 -or $evidenceMatches[0].Groups[1].Value.Replace('\', '/') -notmatch [regex]::Escape($expectedWuEvidencePath)) {
        $wuIssues += "$($wu.Id) evidence must contain canonical WU path $expectedWuEvidencePath"
    }
    $parallelMatches = [regex]::Matches($wu.Text, '(?mi)^\s*-\s*parallelSafety\s*[:\uFF1A]\s*([^\r\n]+)$')
    if ($parallelMatches.Count -ne 1 -or $parallelMatches[0].Groups[1].Value.Trim().TrimEnd('.', [char]0x3002) -cnotmatch '^(isolated|sequential|shared-lock:[a-z0-9._-]+)$') {
        $wuIssues += "$($wu.Id) parallelSafety must use the frozen enum."
    }
    $estimateMatches = [regex]::Matches($wu.Text, '(?mi)^\s*-\s*estimate\s*[:\uFF1A]\s*([^\r\n]+)$')
    if ($estimateMatches.Count -ne 1 -or $estimateMatches[0].Groups[1].Value.Trim().TrimEnd('.', [char]0x3002) -cnotmatch '^(0\.25d|0\.5d|1d|1\.5d)$') {
        $wuIssues += "$($wu.Id) estimate must be one of 0.25d, 0.5d, 1d, 1.5d."
    }
    $dependsText = [regex]::Match($wu.Text, '(?mi)^\s*-\s*dependsOn\s*[:\uFF1A]\s*([^\r\n]+)$').Groups[1].Value
    $depWus = @(Get-UniqueMatches $dependsText 'F2S-WU-M[0-9]{2}-[0-9]{3}-[0-9]{2}')
    $depDevs = @(Get-UniqueMatches $dependsText 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
    $wuEdges[$wu.Id] = @($depWus)
    $devEdges[$wu.TaskId] = @($devEdges[$wu.TaskId] + $depDevs | Sort-Object -Unique)
    foreach ($dep in $depWus) { if ($dep -cnotin $wuUnique) { $wuIssues += "Dangling WU edge: $($wu.Id) -> $dep" } }
    foreach ($dep in $depDevs) { if ($dep -cnotin $baselineDev) { $wuIssues += "Dangling DEV edge: $($wu.Id) -> $dep" } }
    $writesMatch = [regex]::Match($wu.Text, '(?mi)^\s*-\s*writes\s*[:\uFF1A]\s*([^\r\n]+)$')
    if ($writesMatch.Success) {
        $writeParts = @($writesMatch.Groups[1].Value -split '[;\uFF1B\u3001,\uFF0C]' | ForEach-Object { $_.Trim().Trim('`', '.') } | Where-Object { $_ -match '[/\\]' -and $_ -notmatch '^none$' })
        foreach ($path in $writeParts) {
            $normalized = $path.Replace('\', '/').ToLowerInvariant()
            if (-not $pathOwners.ContainsKey($normalized)) { $pathOwners[$normalized] = @() }
            $pathOwners[$normalized] = @($pathOwners[$normalized] + $wu.TaskId | Sort-Object -Unique)
        }
    }
}
$wuCycles = @(Get-CycleNodes $wuUnique $wuEdges)
if ($wuCycles.Count -gt 0) { $wuIssues += 'WU dependency cycle: ' + ($wuCycles -join ', ') }
$devCycles = @(Get-CycleNodes $baselineDev $devEdges)
if ($devCycles.Count -gt 0) { $wuIssues += 'DEV dependency cycle: ' + ($devCycles -join ', ') }
$pathConflicts = @()
foreach ($path in $pathOwners.Keys) {
    $owners = @($pathOwners[$path])
    if ($owners.Count -gt 1) { $pathConflicts += "$path <= $($owners -join ',')" }
}
if ($pathConflicts.Count -gt 0) { $wuIssues += 'Exact write-path multi-owner conflicts: ' + ($pathConflicts -join ' | ') }
$allMilestoneText = (($actualFiles | Where-Object { $_.Name -match '^(0[1-9]|1[0-2])-' } | ForEach-Object { $docTexts[$_.Name] }) -join "`n")
$mentionedDev = @(Get-UniqueMatches $allMilestoneText 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
$mentionedEvd = @(Get-UniqueMatches $allMilestoneText 'F2S-EVD-M[0-9]{2}-[0-9]{3}')
$danglingMentionDev = @($mentionedDev | Where-Object { $_ -cnotin $baselineDev })
$danglingMentionEvd = @($mentionedEvd | Where-Object { $_ -cnotin $baselineEvd })
if ($danglingMentionDev.Count -gt 0) { $wuIssues += 'Dangling DEV mentions: ' + ($danglingMentionDev -join ', ') }
if ($danglingMentionEvd.Count -gt 0) { $wuIssues += 'Dangling EVD mentions: ' + ($danglingMentionEvd -join ', ') }
Add-Check 'WU-DAG-FIELDS-PATH-OWNERS' ($wuIssues.Count -eq 0) ([ordered]@{
    wuCount = $wuUnique.Count; duplicateWus = $wuDuplicates; wuCycleNodes = $wuCycles; devCycleNodes = $devCycles
    exactWritePathCount = $pathOwners.Count; pathConflicts = $pathConflicts
    danglingDevMentions = $danglingMentionDev; danglingEvdMentions = $danglingMentionEvd
}) $wuIssues

$shorthandIssues = @()
$shorthandPatterns = [ordered]@{
    compactIdSlash = 'F2S-(?:FR|NFR|TST)-[A-Z0-9-]+/[0-9]'
    compactIdRange = 'F2S-(?:FR|NFR|TST|DEV|WU|EVD)-[A-Z0-9-]+\.\.[A-Z0-9]'
    bareWuAlias = '\bWU-(?:[A-Z]|[0-9]{1,2})\b'
    bareMilestoneRange = '\bM[0-9]{2}\.\.M[0-9]{2}\b'
    bareMilestoneSlash = '\bM[0-9]{2}/M[0-9]{2}\b'
}
foreach ($task in $tasks) {
    foreach ($kind in $shorthandPatterns.Keys) {
        $hits = @(Get-UniqueMatches $task.Text $shorthandPatterns[$kind])
        if ($hits.Count -gt 0) { $shorthandIssues += "$($task.Id) contains ${kind}: $($hits -join ', ')" }
    }
}
$bareDevAliasPattern = '(?<!F2S-DEV-)(?<!F2S-EVD-)(?<!F2S-WU-)\bM[0-9]{2}-[0-9]{3}\b'
foreach ($wu in $allWus) {
    foreach ($fieldName in @('reads', 'dependsOn')) {
        $fieldMatch = [regex]::Match($wu.Text, '(?mi)^\s*-\s*' + $fieldName + '\s*[:\uFF1A]\s*([^\r\n]+)$')
        if ($fieldMatch.Success) {
            $hits = @(Get-UniqueMatches $fieldMatch.Groups[1].Value $bareDevAliasPattern)
            if ($hits.Count -gt 0) { $shorthandIssues += "$($wu.Id) contains bare DEV aliases in ${fieldName}: $($hits -join ', ')" }
        }
    }
}
Add-Check 'NO-SHORTHAND-TRACE-IDS' ($shorthandIssues.Count -eq 0) ([ordered]@{ patterns = $shorthandPatterns; issueCount = $shorthandIssues.Count }) $shorthandIssues

$allSnapshotText = ($actualFiles | ForEach-Object { $docTexts[$_.Name] }) -join "`n"
$coverageIssues = @()
$snapshotReq = @(Get-UniqueMatches $allSnapshotText 'F2S-(?:FR|NFR)-[A-Z0-9-]+')
$snapshotTests = @(Get-UniqueMatches $allSnapshotText $testPattern)
$missingReq = @($baselineReq | Where-Object { $_ -cnotin $snapshotReq })
$extraReq = @($snapshotReq | Where-Object { $_ -cnotin $baselineReq })
$missingTests = @($baselineTests | Where-Object { $_ -cnotin $snapshotTests })
$extraTests = @($snapshotTests | Where-Object { $_ -cnotin $baselineTests })
if ($baselineReq.Count -ne 102) { $coverageIssues += "Baseline requirement count is $($baselineReq.Count), expected 102." }
if ($baselineTests.Count -ne 133) { $coverageIssues += "Baseline test count is $($baselineTests.Count), expected 133." }
if ($missingReq.Count -gt 0) { $coverageIssues += 'Missing requirement coverage: ' + ($missingReq -join ', ') }
if ($extraReq.Count -gt 0) { $coverageIssues += 'Unknown requirement IDs: ' + ($extraReq -join ', ') }
if ($missingTests.Count -gt 0) { $coverageIssues += 'Missing test coverage: ' + ($missingTests -join ', ') }
if ($extraTests.Count -gt 0) { $coverageIssues += 'Unknown test IDs: ' + ($extraTests -join ', ') }
Add-Check 'REQUIREMENT-TEST-COVERAGE' ($coverageIssues.Count -eq 0) ([ordered]@{
    baselineRequirements = $baselineReq.Count; snapshotRequirements = $snapshotReq.Count
    baselineTests = $baselineTests.Count; snapshotTests = $snapshotTests.Count
    missingRequirements = $missingReq; extraRequirements = $extraReq; missingTests = $missingTests; extraTests = $extraTests
}) $coverageIssues

$p2Map = [ordered]@{
    'F2S-R3A-B06-P2-001' = @('F2S-DEV-M01-001', 'F2S-DEV-M02-001')
    'F2S-R3A-B07-P2-001' = @('F2S-DEV-M00-001', 'F2S-DEV-M01-002')
    'F2S-R3A-B08-P2-001' = @('F2S-DEV-M07-003', 'F2S-DEV-M09-005')
    'F2S-R3A-B09-P2-001' = @('F2S-DEV-M02-001', 'F2S-DEV-M02-002')
    'F2S-R3A-B10-P2-001' = @('F2S-DEV-M03-002', 'F2S-DEV-M03-003')
    'F2S-R3A-B11-P2-001' = @('F2S-DEV-M02-006', 'F2S-DEV-M02-007', 'F2S-DEV-M09-006')
    'F2S-R3A-B12-P2-001' = @('F2S-DEV-M00-004', 'F2S-DEV-M09-001')
    'F2S-R3A-SPINE-13-P2-001' = @('F2S-DEV-M08-007')
}
$p2Issues = @()
$p2Rows = @()
foreach ($p2 in $p2Map.Keys) {
    foreach ($taskId in @($p2Map[$p2])) {
        $task = @($tasks | Where-Object Id -ceq $taskId)
        $present = $task.Count -eq 1 -and $task[0].Text.Contains($p2)
        if (-not $present) { $p2Issues += "$p2 missing from primary task $taskId." }
        $p2Rows += [pscustomobject]@{ p2 = $p2; primaryTask = $taskId; present = $present; evd = $taskId.Replace('F2S-DEV-', 'F2S-EVD-') }
    }
}
Add-Check 'R3B-P2-CARRYOVER' ($p2Issues.Count -eq 0) ([ordered]@{ p2Count = $p2Map.Count; mappings = $p2Rows }) $p2Issues

$boundaryIssues = @()
$markers = [ordered]@{
    spinePatch = @('Spine', '4.2.43')
    openOutputs = @('Rig IR', 'PSD', 'PNG', 'Spine JSON', 'atlas-input')
    proprietaryBoundary = @('.atlas', '.spine', '.skel', 'Professional CLI')
    humanGates = @('approval', 'Layer Gate', 'KeyPose', 'hit')
    actionSet = @('idle', 'run', 'jump', 'fall', 'dash', 'attack_01', 'attack_02', 'attack_03', 'hit', 'death')
    commercialBoundary = @('Windows', 'Production Assist')
    privacy = @('Credential Manager', 'TLS')
    license = @('MIT', 'Apache', 'BSD', 'fail closed')
    externalState = @('NOT_RUN', 'EXTERNAL', 'UNVERIFIED')
    launcher = @('.cmd', 'exe')
}
foreach ($group in $markers.Keys) {
    foreach ($marker in @($markers[$group])) {
        if ($allSnapshotText.IndexOf($marker, [StringComparison]::OrdinalIgnoreCase) -lt 0) { $boundaryIssues += "Boundary marker missing [$group]: $marker" }
    }
}
$forbiddenClaims = @('releaseAuthorized=true')
foreach ($claim in $forbiddenClaims) {
    if ($allSnapshotText.IndexOf($claim, [StringComparison]::OrdinalIgnoreCase) -ge 0) { $boundaryIssues += "Forbidden claim present: $claim" }
}
$forbiddenActionAliases = @(Get-UniqueMatches $allMilestoneText '(?<![0-9])attack_[123](?![0-9])|attack_1\.\.3')
if ($forbiddenActionAliases.Count -gt 0) { $boundaryIssues += 'Non-canonical ActionKey aliases present: ' + ($forbiddenActionAliases -join ', ') }
$unversionedSpineMentions = @(Get-UniqueMatches $allMilestoneText 'Spine\s+4\.2(?!\.43)')
if ($unversionedSpineMentions.Count -gt 0) { $boundaryIssues += 'Unversioned Spine 4.2 mentions present; exact patch 4.2.43 is required.' }
if ($allMilestoneText.Contains('F2S-DOC-TRACE-001')) { $boundaryIssues += 'Deprecated trace document alias F2S-DOC-TRACE-001 is present.' }
if ($allMilestoneText.Contains('capability manifest 外部')) { $boundaryIssues += 'Capability manifest is incorrectly modeled as an external dependency.' }
foreach ($evidenceMarker in @('EvidenceEnvelope', 'schemas/src/evidence.schema.json', 'startedAtUtc', 'externalBlockers', 'previousEvidenceRef', 'payload')) {
    if ($allMilestoneText.IndexOf($evidenceMarker, [StringComparison]::Ordinal) -lt 0) { $boundaryIssues += "Evidence contract marker missing: $evidenceMarker" }
}
foreach ($milestoneFile in @($actualFiles | Where-Object { $_.Name -match '^(0[1-9]|1[0-2])-' })) {
    $milestoneText = $docTexts[$milestoneFile.Name]
    if (-not $milestoneText.Contains('EvidenceEnvelope')) { $boundaryIssues += "EvidenceEnvelope not bound in $($milestoneFile.Name)." }
}
foreach ($capabilityPrefix in @('01', '06', '07', '09')) {
    $capabilityFile = @($actualFiles | Where-Object { $_.Name -like "$capabilityPrefix-*" })
    if ($capabilityFile.Count -ne 1) { continue }
    $capabilityText = $docTexts[$capabilityFile[0].Name]
    if (-not $capabilityText.Contains('F2S-SPINE-CAP-4.2.43-001')) { $boundaryIssues += "Exact Spine capability ID missing in $($capabilityFile[0].Name)." }
    if (-not $capabilityText.Contains('fixtures/m00/spine42-probe/capability-manifest.json')) { $boundaryIssues += "Exact Spine capability path missing in $($capabilityFile[0].Name)." }
}
$m09File = @($actualFiles | Where-Object { $_.Name -like '10-*' })
if ($m09File.Count -eq 1) {
    $m09Text = $docTexts[$m09File[0].Name]
    foreach ($performanceMarker in @('evidence/M07/F2S-DEV-M07-003/performance-fixture-contract.json', 'fixtureManifestSha256', 'rawSampleFiles', 'rendererBuildSha256', 'nearest-rank')) {
        if (-not $m09Text.Contains($performanceMarker)) { $boundaryIssues += "M09 exact performance-consumption marker missing: $performanceMarker" }
    }
}
Add-Check 'PRODUCT-SPINE-SECURITY-RELEASE-BOUNDARIES' ($boundaryIssues.Count -eq 0) ([ordered]@{ markerGroups = $markers.Keys; forbiddenClaims = $forbiddenClaims }) $boundaryIssues

$traceIssues = @()
$traceFiles = @($actualFiles | Where-Object { $_.Name -like '13-*' })
if ($traceFiles.Count -ne 1) { $traceIssues += 'Trace document is missing or duplicated.' }
else {
    $traceName = $traceFiles[0].Name
    $traceText = $docTexts[$traceName]
    $traceDev = @(Get-UniqueMatches $traceText 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
    $traceEvd = @(Get-UniqueMatches $traceText 'F2S-EVD-M[0-9]{2}-[0-9]{3}')
    $traceWus = @(Get-UniqueMatches $traceText 'F2S-WU-M[0-9]{2}-[0-9]{3}-[0-9]{2}')
    if (@($baselineDev | Where-Object { $_ -cnotin $traceDev }).Count -gt 0) { $traceIssues += 'Trace does not enumerate all 80 DEV IDs.' }
    if (@($baselineEvd | Where-Object { $_ -cnotin $traceEvd }).Count -gt 0) { $traceIssues += 'Trace does not enumerate all 80 EVD IDs.' }
    if (@($wuUnique | Where-Object { $_ -cnotin $traceWus }).Count -gt 0) { $traceIssues += 'Trace does not enumerate all WU IDs.' }
    foreach ($p2 in $p2Map.Keys) { if (-not $traceText.Contains($p2)) { $traceIssues += "Trace missing P2 row: $p2" } }
    foreach ($milestoneFile in @($actualFiles | Where-Object { $_.Name -match '^(0[1-9]|1[0-2])-' })) {
        $expectedSourceHash = Get-Sha256 $milestoneFile.FullName
        $sourceHashPattern = '(?m)^\|\s*' + [regex]::Escape($milestoneFile.Name) + '\s*\|\s*`([0-9a-f]{64})`\s*\|\s*$'
        $sourceHashMatch = [regex]::Match($traceText, $sourceHashPattern)
        if (-not $sourceHashMatch.Success) { $traceIssues += "Trace source hash row missing: $($milestoneFile.Name)" }
        elseif ($sourceHashMatch.Groups[1].Value -cne $expectedSourceHash) { $traceIssues += "Trace source hash mismatch: $($milestoneFile.Name)" }
    }

    $section3Match = [regex]::Match($traceText, '(?s)^## 3\..*?(?=^## 4\.)', [System.Text.RegularExpressions.RegexOptions]::Multiline)
    $section5Match = [regex]::Match($traceText, '(?s)^## 5\..*?(?=^## 6\.)', [System.Text.RegularExpressions.RegexOptions]::Multiline)
    $section6Match = [regex]::Match($traceText, '(?s)^## 6\..*?(?=^## 7\.)', [System.Text.RegularExpressions.RegexOptions]::Multiline)
    if (-not $section3Match.Success -or -not $section5Match.Success -or -not $section6Match.Success) {
        $traceIssues += 'Trace sections 3, 5, or 6 are not parseable.'
    } else {
        $traceTaskRows = @{}
        foreach ($line in ($section3Match.Value -split '[\r\n]+')) {
            if ($line -notmatch '^\|\s*(F2S-DEV-M[0-9]{2}-[0-9]{3})') { continue }
            $columns = @($line -split '\|')
            if ($columns.Count -lt 10) { $traceIssues += "Malformed section 3 row: $($matches[1])"; continue }
            $traceTaskRows[$matches[1]] = [ordered]@{
                requirements = @(Get-UniqueMatches $columns[4] 'F2S-(?:FR|NFR)-[A-Z0-9-]+')
                tests = @(Get-UniqueMatches $columns[5] $testPattern)
                dependencies = @(Get-UniqueMatches $columns[7] 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
            }
        }
        foreach ($task in $tasks) {
            if (-not $traceTaskRows.ContainsKey($task.Id)) { $traceIssues += "Section 3 missing task row $($task.Id)."; continue }
            $expectedReqEdges = @(Get-UniqueMatches $task.Text 'F2S-(?:FR|NFR)-[A-Z0-9-]+')
            $expectedTestEdges = @(Get-UniqueMatches $task.Text $testPattern)
            $declaredDependencyMatch = [regex]::Match($task.Text, '(?m)^.*?\u4E0A\u6E38(?:\s+DEV)?\s*[:\uFF1A]\s*([^;\uFF1B\r\n]+)')
            $expectedDependencyEdges = @()
            if ($declaredDependencyMatch.Success) { $expectedDependencyEdges = @(Get-UniqueMatches $declaredDependencyMatch.Groups[1].Value 'F2S-DEV-M[0-9]{2}-[0-9]{3}') }
            if ($expectedDependencyEdges.Count -eq 0) {
                $taskWus = @($allWus | Where-Object TaskId -ceq $task.Id)
                foreach ($taskWu in $taskWus) {
                    $dependsValue = [regex]::Match($taskWu.Text, '(?mi)^\s*-\s*dependsOn\s*[:\uFF1A]\s*([^\r\n]+)$').Groups[1].Value
                    $expectedDependencyEdges += Get-UniqueMatches $dependsValue 'F2S-DEV-M[0-9]{2}-[0-9]{3}'
                }
                $expectedDependencyEdges = @($expectedDependencyEdges | Sort-Object -Unique)
            }
            foreach ($edge in @($expectedReqEdges | Where-Object { $_ -cnotin $traceTaskRows[$task.Id].requirements })) { $traceIssues += "Section 3 missing requirement edge: $($task.Id) -> $edge" }
            foreach ($edge in @($traceTaskRows[$task.Id].requirements | Where-Object { $_ -cnotin $expectedReqEdges })) { $traceIssues += "Section 3 has extra requirement edge: $($task.Id) -> $edge" }
            foreach ($edge in @($expectedTestEdges | Where-Object { $_ -cnotin $traceTaskRows[$task.Id].tests })) { $traceIssues += "Section 3 missing test edge: $($task.Id) -> $edge" }
            foreach ($edge in @($traceTaskRows[$task.Id].tests | Where-Object { $_ -cnotin $expectedTestEdges })) { $traceIssues += "Section 3 has extra test edge: $($task.Id) -> $edge" }
            foreach ($edge in @($expectedDependencyEdges | Where-Object { $_ -cnotin $traceTaskRows[$task.Id].dependencies })) { $traceIssues += "Section 3 missing dependency edge: $($task.Id) -> $edge" }
            foreach ($edge in @($traceTaskRows[$task.Id].dependencies | Where-Object { $_ -cnotin $expectedDependencyEdges })) { $traceIssues += "Section 3 has extra dependency edge: $($task.Id) -> $edge" }
        }

        $traceReqRows = @{}
        foreach ($line in ($section5Match.Value -split '[\r\n]+')) {
            if ($line -notmatch '^\|\s*(F2S-(?:FR|NFR)-[A-Z0-9-]+)\s*\|') { continue }
            $columns = @($line -split '\|')
            if ($columns.Count -ge 6) { $traceReqRows[$matches[1]] = @(Get-UniqueMatches $columns[3] 'F2S-DEV-M[0-9]{2}-[0-9]{3}') }
        }
        foreach ($reqId in $baselineReq) {
            $expectedDevs = @($tasks | Where-Object { @(Get-UniqueMatches $_.Text 'F2S-(?:FR|NFR)-[A-Z0-9-]+') -ccontains $reqId } | ForEach-Object Id | Sort-Object -Unique)
            if (-not $traceReqRows.ContainsKey($reqId)) { $traceIssues += "Section 5 missing requirement row $reqId."; continue }
            foreach ($edge in @($expectedDevs | Where-Object { $_ -cnotin $traceReqRows[$reqId] })) { $traceIssues += "Section 5 missing reverse requirement edge: $reqId -> $edge" }
            foreach ($edge in @($traceReqRows[$reqId] | Where-Object { $_ -cnotin $expectedDevs })) { $traceIssues += "Section 5 has extra reverse requirement edge: $reqId -> $edge" }
        }

        $traceTestRows = @{}
        foreach ($line in ($section6Match.Value -split '[\r\n]+')) {
            if ($line -notmatch '^\|\s*(F2S-TST-[A-Z0-9-]+)\s*\|') { continue }
            $columns = @($line -split '\|')
            if ($columns.Count -ge 6) { $traceTestRows[$matches[1]] = [ordered]@{ devs = @(Get-UniqueMatches $columns[2] 'F2S-DEV-M[0-9]{2}-[0-9]{3}'); evds = @(Get-UniqueMatches $columns[3] 'F2S-EVD-M[0-9]{2}-[0-9]{3}') } }
        }
        foreach ($testId in $baselineTests) {
            $expectedDevs = @($tasks | Where-Object { @(Get-UniqueMatches $_.Text $testPattern) -ccontains $testId } | ForEach-Object Id | Sort-Object -Unique)
            $expectedEvds = @($expectedDevs | ForEach-Object { $_.Replace('F2S-DEV-', 'F2S-EVD-') } | Sort-Object -Unique)
            if (-not $traceTestRows.ContainsKey($testId)) { $traceIssues += "Section 6 missing test row $testId."; continue }
            foreach ($edge in @($expectedDevs | Where-Object { $_ -cnotin $traceTestRows[$testId].devs })) { $traceIssues += "Section 6 missing reverse test edge: $testId -> $edge" }
            foreach ($edge in @($traceTestRows[$testId].devs | Where-Object { $_ -cnotin $expectedDevs })) { $traceIssues += "Section 6 has extra reverse test edge: $testId -> $edge" }
            foreach ($edge in @($expectedEvds | Where-Object { $_ -cnotin $traceTestRows[$testId].evds })) { $traceIssues += "Section 6 missing evidence edge: $testId -> $edge" }
            foreach ($edge in @($traceTestRows[$testId].evds | Where-Object { $_ -cnotin $expectedEvds })) { $traceIssues += "Section 6 has extra evidence edge: $testId -> $edge" }
        }
    }
}
Add-Check 'TRACE-MATRIX-REVERSE-COVERAGE' ($traceIssues.Count -eq 0) ([ordered]@{
    dev = $baselineDev.Count; evd = $baselineEvd.Count; wu = $wuUnique.Count; requirements = $baselineReq.Count; tests = $baselineTests.Count
}) $traceIssues

$passCount = @($script:Results | Where-Object status -ceq 'PASS').Count
$failCount = @($script:Results | Where-Object status -ceq 'FAIL').Count
$overall = if ($failCount -eq 0) { 'PASS' } else { 'FAIL' }
$summary = [ordered]@{
    schemaVersion = '1.0.0'
    evidenceId = $EvidenceId
    phase = $Phase
    snapshotId = $SnapshotId
    overallVerdict = $overall
    designGatePassed = ($Phase -ceq 'FINAL' -and $overall -ceq 'PASS')
    executionState = if ($Phase -ceq 'FINAL' -and $overall -ceq 'PASS') { 'WAITING_FOR_USER_START' } else { 'REVIEW_PENDING' }
    userStartRequired = $true
    implementationAuthorized = $false
    releaseAuthorized = $false
    checkCount = $script:Results.Count
    passCount = $passCount
    failCount = $failCount
    manifestPath = Get-RelativePath $script:Workspace $manifestPath
    manifestSha256 = if (Test-Path -LiteralPath $manifestPath) { Get-Sha256 $manifestPath } else { $null }
    command = $script:ExactCommand
    toolPath = Get-RelativePath $script:Workspace $MyInvocation.MyCommand.Path
    toolSha256 = Get-Sha256 $MyInvocation.MyCommand.Path
    checks = $script:Results
}
$summaryJsonPath = Join-Path $stagingRoot 'audit.json'
Write-Utf8New $summaryJsonPath (($summary | ConvertTo-Json -Depth 50) + [Environment]::NewLine)
$textLines = @(
    "# Devplan mechanical audit - $SnapshotId",
    '',
    "- evidenceId: $EvidenceId",
    "- phase: $Phase",
    "- overallVerdict: $overall",
    "- checks: $passCount PASS / $failCount FAIL / $($script:Results.Count) total",
    "- designGatePassed: $($summary.designGatePassed.ToString().ToLowerInvariant())",
    "- executionState: $($summary.executionState)",
    '- userStartRequired: true',
    '- implementationAuthorized: false',
    '- releaseAuthorized: false',
    '',
    '| Check | Status | Raw SHA-256 |',
    '| --- | --- | --- |'
)
foreach ($result in $script:Results) { $textLines += "| $($result.checkId) | $($result.status) | $($result.rawSha256) |" }
if ($failCount -gt 0) {
    $textLines += ''
    $textLines += '## Failures'
    $textLines += ''
    foreach ($result in $script:Results | Where-Object status -ceq 'FAIL') {
        foreach ($detail in @($result.details)) { $textLines += "- $($result.checkId): $detail" }
    }
}
$summaryTextPath = Join-Path $stagingRoot 'audit.md'
Write-Utf8New $summaryTextPath (($textLines -join [Environment]::NewLine) + [Environment]::NewLine)

Move-Item -LiteralPath $stagingRoot -Destination $auditRoot
Get-ChildItem -LiteralPath $auditRoot -Recurse -File | ForEach-Object { $_.IsReadOnly = $true }

[pscustomobject]@{
    SnapshotId = $SnapshotId
    Verdict = $overall
    Checks = $script:Results.Count
    Pass = $passCount
    Fail = $failCount
    AuditJson = Get-RelativePath $script:Workspace (Join-Path $auditRoot 'audit.json')
    AuditJsonSha256 = Get-Sha256 (Join-Path $auditRoot 'audit.json')
    AuditMarkdown = Get-RelativePath $script:Workspace (Join-Path $auditRoot 'audit.md')
    AuditMarkdownSha256 = Get-Sha256 (Join-Path $auditRoot 'audit.md')
}

if ($overall -cne 'PASS') { exit 1 }
