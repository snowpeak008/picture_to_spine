[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Z0-9][A-Z0-9-]{0,127}$')]
    [string]$SnapshotId
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:AuditVersion = '1.0.3'
$script:Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$script:Utf8Strict = [System.Text.UTF8Encoding]::new($false, $true)
$script:Manifest = $null
$script:ArchivePath = $null
$script:Documents = @()
$script:DocumentTextByPrefix = @{}
$script:DocumentRecordByPrefix = @{}
$script:FrontMatterByPrefix = @{}
$script:CheckResults = @()

function Get-AsiaShanghaiTimestamp {
    return [TimeZoneInfo]::ConvertTimeBySystemTimeZoneId(
        [DateTimeOffset]::UtcNow,
        'China Standard Time'
    ).ToString('yyyy-MM-ddTHH:mm:ss.fffffffzzz')
}

function Get-Sha256HexFromBytes([byte[]]$Bytes) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $digest = $sha.ComputeHash($Bytes)
        return [BitConverter]::ToString($digest).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Get-Sha256HexFromFile([string]$LiteralPath) {
    $stream = [IO.File]::Open(
        $LiteralPath,
        [IO.FileMode]::Open,
        [IO.FileAccess]::Read,
        [IO.FileShare]::Read
    )
    try {
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            $digest = $sha.ComputeHash($stream)
            return [BitConverter]::ToString($digest).Replace('-', '').ToLowerInvariant()
        }
        finally {
            $sha.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Get-Sha256HexFromStream([IO.Stream]$Stream) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $digest = $sha.ComputeHash($Stream)
        return [BitConverter]::ToString($digest).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Join-ByteArrays([byte[]]$Left, [byte[]]$Right) {
    $combined = New-Object byte[] ($Left.Length + $Right.Length)
    [Array]::Copy($Left, 0, $combined, 0, $Left.Length)
    [Array]::Copy($Right, 0, $combined, $Left.Length, $Right.Length)
    return $combined
}

function Read-Utf8Text([string]$LiteralPath) {
    return [IO.File]::ReadAllText($LiteralPath, $script:Utf8Strict)
}

function Write-NewUtf8File([string]$LiteralPath, [string]$Content) {
    $parent = Split-Path -Parent $LiteralPath
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        $null = New-Item -ItemType Directory -Path $parent
    }

    $stream = [IO.File]::Open(
        $LiteralPath,
        [IO.FileMode]::CreateNew,
        [IO.FileAccess]::Write,
        [IO.FileShare]::None
    )
    try {
        $bytes = $script:Utf8NoBom.GetBytes($Content)
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush()
    }
    finally {
        $stream.Dispose()
    }
}

function Get-WorkspaceRelativePath([string]$Root, [string]$Path) {
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    $pathFull = [IO.Path]::GetFullPath($Path)
    if (-not $pathFull.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes workspace: $pathFull"
    }
    return $pathFull.Substring($rootFull.Length).Replace('\', '/')
}

function Resolve-WorkspaceRelativePath([string]$Root, [string]$RelativePath) {
    if ([IO.Path]::IsPathRooted($RelativePath)) {
        throw "Absolute path is forbidden in a manifest: $RelativePath"
    }
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    $candidate = [IO.Path]::GetFullPath((Join-Path $Root $RelativePath.Replace('/', '\')))
    if (-not $candidate.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Manifest path escapes workspace: $RelativePath"
    }
    return $candidate
}

function Get-ObjectPropertyValue($Object, [string]$Name) {
    if ($null -eq $Object) {
        return $null
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function New-Outcome([bool]$Pass, $Metric, $Details) {
    return [pscustomobject][ordered]@{
        Pass = $Pass
        Metric = $Metric
        Details = @($Details)
    }
}

function Get-FrontMatterScalar([string]$Body, [string]$Name) {
    $pattern = '(?m)^' + [regex]::Escape($Name) + ':[ \t]*(.*?)[ \t]*$'
    $matches = [regex]::Matches($Body, $pattern)
    if ($matches.Count -ne 1) {
        return $null
    }
    return $matches[0].Groups[1].Value.Trim().Trim('"').Trim("'")
}

function Get-FrontMatterList([string]$Body, [string]$Name) {
    $linePattern = '(?m)^' + [regex]::Escape($Name) + ':[ \t]*(.*?)[ \t]*$'
    $matches = [regex]::Matches($Body, $linePattern)
    if ($matches.Count -ne 1) {
        return [pscustomobject]@{ Valid = $false; Items = @(); Error = "Expected one $Name field." }
    }

    $raw = $matches[0].Groups[1].Value.Trim()
    if ($raw.StartsWith('[') -and $raw.EndsWith(']')) {
        $inside = $raw.Substring(1, $raw.Length - 2).Trim()
        if ($inside.Length -eq 0) {
            return [pscustomobject]@{ Valid = $true; Items = @(); Error = $null }
        }
        $items = @($inside.Split(',') | ForEach-Object { $_.Trim().Trim('"').Trim("'") })
        return [pscustomobject]@{ Valid = $true; Items = $items; Error = $null }
    }

    if ($raw.Length -ne 0) {
        return [pscustomobject]@{ Valid = $false; Items = @(); Error = "$Name is not a YAML list." }
    }

    $tailStart = $matches[0].Index + $matches[0].Length
    $tail = $Body.Substring($tailStart)
    $items = @()
    foreach ($line in ($tail -split "`n")) {
        $clean = $line.TrimEnd("`r")
        if ($clean -match '^\s*-\s*(\S.*?)\s*$') {
            $items += $matches[1].Trim().Trim('"').Trim("'")
            continue
        }
        if ($clean -match '^\s*$') {
            continue
        }
        if ($clean -match '^[A-Za-z_][A-Za-z0-9_]*\s*:') {
            break
        }
        return [pscustomobject]@{ Valid = $false; Items = @(); Error = "Invalid list item in $Name." }
    }
    return [pscustomobject]@{ Valid = $true; Items = $items; Error = $null }
}

function Parse-FrontMatter([string]$Text) {
    $normalized = $Text.Replace("`r`n", "`n").Replace("`r", "`n")
    $match = [regex]::Match(
        $normalized,
        '\A---\n(?<body>.*?)\n---(?:\n|\z)',
        [Text.RegularExpressions.RegexOptions]::Singleline
    )
    if (-not $match.Success) {
        return [pscustomobject]@{
            Valid = $false
            Error = 'Missing or malformed leading YAML frontmatter.'
            Body = $null
            DocId = $null
            Revision = $null
            Status = $null
            CanonicalFor = @()
            DependsOn = @()
            ReviewScoreRef = $null
            LastVerified = $null
        }
    }

    $body = $match.Groups['body'].Value
    $canonical = Get-FrontMatterList $body 'canonical_for'
    $depends = Get-FrontMatterList $body 'depends_on'
    $requiredScalars = @('doc_id', 'revision', 'status', 'review_score_ref', 'last_verified')
    $issues = @()
    foreach ($name in $requiredScalars) {
        $count = [regex]::Matches($body, ('(?m)^' + [regex]::Escape($name) + ':')).Count
        if ($count -ne 1) {
            $issues += "Expected one $name field; found $count."
        }
    }
    if (-not $canonical.Valid) {
        $issues += $canonical.Error
    }
    if (-not $depends.Valid) {
        $issues += $depends.Error
    }

    return [pscustomobject]@{
        Valid = ($issues.Count -eq 0)
        Error = ($issues -join ' ')
        Body = $body
        DocId = Get-FrontMatterScalar $body 'doc_id'
        Revision = Get-FrontMatterScalar $body 'revision'
        Status = Get-FrontMatterScalar $body 'status'
        CanonicalFor = @($canonical.Items)
        DependsOn = @($depends.Items)
        ReviewScoreRef = Get-FrontMatterScalar $body 'review_score_ref'
        LastVerified = Get-FrontMatterScalar $body 'last_verified'
    }
}

function Get-RequirementMap([string]$Text) {
    $map = @{}
    $duplicates = @()
    foreach ($line in ($Text -split "`r?`n")) {
        if ($line -match '^\|\s*`?(F2S-(FR|NFR)-[A-Z0-9-]+)`?\s*\|\s*(P[012])\s*\|') {
            $id = $matches[1]
            if ($map.ContainsKey($id)) {
                $duplicates += $id
            }
            else {
                $map[$id] = $matches[3]
            }
        }
    }
    return [pscustomobject]@{ Map = $map; Duplicates = @($duplicates) }
}

function Get-UniqueRegexMatches([string]$Text, [string]$Pattern) {
    return @([regex]::Matches($Text, $Pattern) | ForEach-Object { $_.Value } | Sort-Object -Unique)
}

function Invoke-AuditCheck {
    param(
        [Parameter(Mandatory = $true)][string]$CheckId,
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string]$Tool,
        [Parameter(Mandatory = $true)][scriptblock]$Action
    )

    $startedAt = Get-AsiaShanghaiTimestamp
    $status = 'ERROR'
    $exitCode = 2
    $metric = $null
    $details = @()
    try {
        $outputs = @(& $Action)
        if ($outputs.Count -ne 1) {
            throw "Check action returned $($outputs.Count) values; expected one."
        }
        $outcome = $outputs[0]
        $metric = $outcome.Metric
        $details = @($outcome.Details)
        if ([bool]$outcome.Pass) {
            $status = 'PASS'
            $exitCode = 0
        }
        else {
            $status = 'FAIL'
            $exitCode = 1
        }
    }
    catch {
        $status = 'ERROR'
        $exitCode = 2
        $metric = $null
        $details = @($_.Exception.ToString())
    }
    $endedAt = Get-AsiaShanghaiTimestamp

    $rawName = $CheckId.ToLowerInvariant() + '.json'
    $stagingRawPath = Join-Path $script:StagingRawRoot $rawName
    $finalRawPath = Join-Path $script:FinalRawRoot $rawName
    $rawRelativePath = Get-WorkspaceRelativePath $script:Workspace $finalRawPath
    $rawObject = [ordered]@{
        schemaVersion = '1.0.0'
        snapshotId = $SnapshotId
        checkId = $CheckId
        status = $status
        metric = $metric
        command = $Command
        tool = $Tool
        exitCode = $exitCode
        startedAtAsiaShanghai = $startedAt
        endedAtAsiaShanghai = $endedAt
        details = $details
    }
    $rawJson = $rawObject | ConvertTo-Json -Depth 40
    Write-NewUtf8File $stagingRawPath ($rawJson + [Environment]::NewLine)
    $rawSha256 = Get-Sha256HexFromFile $stagingRawPath

    $script:CheckResults += [pscustomobject][ordered]@{
        checkId = $CheckId
        status = $status
        metric = $metric
        command = $Command
        tool = $Tool
        exitCode = $exitCode
        startedAtAsiaShanghai = $startedAt
        endedAtAsiaShanghai = $endedAt
        rawPath = $rawRelativePath
        rawSha256 = $rawSha256
    }
}

$script:Workspace = [IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path)
$reviewRoot = Join-Path (Join-Path $script:Workspace 'plan') 'reviews'
$snapshotRoot = Join-Path (Join-Path $reviewRoot 'snapshots') $SnapshotId
$manifestPath = Join-Path $snapshotRoot 'manifest.json'
$auditsRoot = Join-Path $reviewRoot 'audits'
$script:FinalAuditRoot = Join-Path $auditsRoot $SnapshotId
$script:FinalRawRoot = Join-Path $script:FinalAuditRoot 'raw'

if (Test-Path -LiteralPath $script:FinalAuditRoot) {
    throw "Audit is immutable and already exists: $($script:FinalAuditRoot)"
}
if (-not (Test-Path -LiteralPath $auditsRoot -PathType Container)) {
    $null = New-Item -ItemType Directory -Path $auditsRoot
}

$stagingName = '.' + $SnapshotId + '.' + [Guid]::NewGuid().ToString('N') + '.staging'
$script:StagingAuditRoot = Join-Path $auditsRoot $stagingName
$script:StagingRawRoot = Join-Path $script:StagingAuditRoot 'raw'
$null = New-Item -ItemType Directory -Path $script:StagingRawRoot

$runtimePath = (Get-Process -Id $PID).Path
$runtimeVersion = $PSVersionTable.PSVersion.ToString()
$scriptPath = $MyInvocation.MyCommand.Path
$exactCommand = "powershell -NoProfile -ExecutionPolicy Bypass -File plan/reviews/tools/Invoke-PlanAudit.ps1 -SnapshotId $SnapshotId"
$baseTool = "PowerShell $runtimeVersion; Invoke-PlanAudit.ps1/$($script:AuditVersion)"

Invoke-AuditCheck 'SNAPSHOT-MANIFEST-ARCHIVE' $exactCommand $baseTool {
    $issues = @()
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        return New-Outcome $false ([ordered]@{ manifestExists = $false }) @('manifest.json is missing.')
    }

    $manifestText = Read-Utf8Text $manifestPath
    try {
        $manifest = $manifestText | ConvertFrom-Json
        $script:Manifest = $manifest
    }
    catch {
        return New-Outcome $false ([ordered]@{ manifestExists = $true; manifestParsed = $false }) @($_.Exception.Message)
    }

    if ((Get-ObjectPropertyValue $manifest 'schemaVersion') -cne '1.0.0') { $issues += 'schemaVersion must be 1.0.0.' }
    if ((Get-ObjectPropertyValue $manifest 'snapshotId') -cne $SnapshotId) { $issues += 'snapshotId does not match the parameter.' }
    if (@('R3A', 'R3B') -cnotcontains [string](Get-ObjectPropertyValue $manifest 'phase')) { $issues += 'phase must be R3A or R3B.' }
    if ((Get-ObjectPropertyValue $manifest 'sourceRoot') -cne 'plan') { $issues += 'sourceRoot must be plan.' }
    if ([string](Get-ObjectPropertyValue $manifest 'documentCount') -cne '25') { $issues += 'documentCount must be the string 25.' }
    if (-not (Get-Item -LiteralPath $manifestPath).IsReadOnly) { $issues += 'manifest.json is not read-only.' }

    $expectedArchiveRelative = ('plan/reviews/snapshots/' + $SnapshotId + '.zip')
    $archiveRelative = [string](Get-ObjectPropertyValue $manifest 'archivePath')
    if ($archiveRelative.Replace('\', '/') -cne $expectedArchiveRelative) {
        $issues += 'archivePath is not the canonical snapshot archive path.'
    }

    try {
        $archivePath = Resolve-WorkspaceRelativePath $script:Workspace $archiveRelative
        $script:ArchivePath = $archivePath
    }
    catch {
        $archivePath = $null
        $issues += $_.Exception.Message
    }

    $actualArchiveHash = $null
    $archiveReadOnly = $false
    if ($null -eq $archivePath -or -not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
        $issues += 'Snapshot archive is missing.'
    }
    else {
        $archiveItem = Get-Item -LiteralPath $archivePath
        $archiveReadOnly = $archiveItem.IsReadOnly
        if (-not $archiveReadOnly) { $issues += 'Snapshot archive is not read-only.' }
        $actualArchiveHash = Get-Sha256HexFromFile $archivePath
        if ($actualArchiveHash -cne [string](Get-ObjectPropertyValue $manifest 'archiveSha256')) {
            $issues += 'Snapshot archive SHA-256 does not match manifest.json.'
        }
    }

    $metric = [ordered]@{
        phase = Get-ObjectPropertyValue $manifest 'phase'
        manifestSha256 = Get-Sha256HexFromFile $manifestPath
        manifestReadOnly = (Get-Item -LiteralPath $manifestPath).IsReadOnly
        archivePath = $archiveRelative
        expectedArchiveSha256 = Get-ObjectPropertyValue $manifest 'archiveSha256'
        actualArchiveSha256 = $actualArchiveHash
        archiveReadOnly = $archiveReadOnly
        issueCount = $issues.Count
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'SNAPSHOT-DOCUMENT-HASHES' $exactCommand "$baseTool; .NET SHA256" {
    if ($null -eq $script:Manifest) {
        return New-Outcome $false ([ordered]@{ verified = 0; expected = 25 }) @('Manifest prerequisite failed.')
    }

    $issues = @()
    $records = @()
    $documents = @(Get-ObjectPropertyValue $script:Manifest 'documents')
    if ($documents.Count -ne 25) { $issues += "Manifest documents count is $($documents.Count), expected 25." }
    $paths = @()
    $docIds = @()
    $readOnlyCount = 0
    $hashMatchCount = 0
    $sizeMatchCount = 0

    foreach ($document in $documents) {
        $relativePath = [string](Get-ObjectPropertyValue $document 'path')
        $docId = [string](Get-ObjectPropertyValue $document 'docId')
        $revision = [string](Get-ObjectPropertyValue $document 'revision')
        $expectedSize = [string](Get-ObjectPropertyValue $document 'size')
        $expectedHash = [string](Get-ObjectPropertyValue $document 'sha256')
        $paths += $relativePath.Replace('\', '/')
        $docIds += $docId

        if ($relativePath.Replace('\', '/') -notmatch '^plan/[^/]+\.md$') {
            $issues += "Non-direct plan document path: $relativePath"
            continue
        }
        try {
            $literalPath = Resolve-WorkspaceRelativePath $snapshotRoot $relativePath
        }
        catch {
            $issues += $_.Exception.Message
            continue
        }
        if (-not (Test-Path -LiteralPath $literalPath -PathType Leaf)) {
            $issues += "Snapshot document is missing: $relativePath"
            continue
        }

        $item = Get-Item -LiteralPath $literalPath
        if ($item.IsReadOnly) { $readOnlyCount++ } else { $issues += "Document is not read-only: $relativePath" }
        if ([string]$item.Length -ceq $expectedSize) { $sizeMatchCount++ } else { $issues += "Size mismatch: $relativePath" }
        $actualHash = Get-Sha256HexFromFile $literalPath
        if ($actualHash -ceq $expectedHash) { $hashMatchCount++ } else { $issues += "SHA-256 mismatch: $relativePath" }
        if ($expectedHash -notmatch '^[0-9a-f]{64}$') { $issues += "Non-canonical SHA-256: $relativePath" }

        $records += [pscustomobject][ordered]@{
            DocId = $docId
            RelativePath = $relativePath.Replace('\', '/')
            LiteralPath = $literalPath
            Revision = $revision
            Size = $item.Length
            Sha256 = $actualHash
        }
    }

    $duplicatePaths = @($paths | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
    $duplicateDocIds = @($docIds | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
    if ($duplicatePaths.Count -gt 0) { $issues += 'Duplicate manifest paths: ' + ($duplicatePaths -join ', ') }
    if ($duplicateDocIds.Count -gt 0) { $issues += 'Duplicate manifest docIds: ' + ($duplicateDocIds -join ', ') }

    $actualDirectFiles = @(Get-ChildItem -LiteralPath (Join-Path $snapshotRoot 'plan') -File -Filter '*.md')
    $actualRelative = @($actualDirectFiles | ForEach-Object { 'plan/' + $_.Name } | Sort-Object)
    $manifestRelative = @($paths | Sort-Object)
    $extra = @($actualRelative | Where-Object { $_ -cnotin $manifestRelative })
    $missing = @($manifestRelative | Where-Object { $_ -cnotin $actualRelative })
    if ($extra.Count -gt 0) { $issues += 'Unmanifested snapshot documents: ' + ($extra -join ', ') }
    if ($missing.Count -gt 0) { $issues += 'Manifest documents absent from snapshot: ' + ($missing -join ', ') }
    if ($actualDirectFiles.Count -ne 25) { $issues += "Direct snapshot markdown count is $($actualDirectFiles.Count), expected 25." }

    $script:Documents = @($records)
    $metric = [ordered]@{
        manifestDocumentCount = $documents.Count
        directSnapshotDocumentCount = $actualDirectFiles.Count
        verifiedRecordCount = $records.Count
        readOnlyCount = $readOnlyCount
        sizeMatchCount = $sizeMatchCount
        sha256MatchCount = $hashMatchCount
        duplicatePathCount = $duplicatePaths.Count
        duplicateDocIdCount = $duplicateDocIds.Count
        missingCount = $missing.Count
        extraCount = $extra.Count
        issueCount = $issues.Count
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'SNAPSHOT-ARCHIVE-ENTRIES' $exactCommand "$baseTool; ZipArchive; .NET SHA256" {
    if ($null -eq $script:ArchivePath -or $script:Documents.Count -eq 0) {
        return New-Outcome $false ([ordered]@{ verified = 0; expected = 25 }) @('Archive or document prerequisite failed.')
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $issues = @()
    $entryRows = @()
    $archive = [IO.Compression.ZipFile]::OpenRead($script:ArchivePath)
    try {
        $entries = @($archive.Entries | Where-Object { -not $_.FullName.EndsWith('/') -and -not $_.FullName.EndsWith('\') })
        $entryNames = @($entries | ForEach-Object { $_.FullName.Replace('\', '/') })
        $expectedNames = @($script:Documents | ForEach-Object { $_.RelativePath })
        foreach ($entry in $entries) {
            $name = $entry.FullName.Replace('\', '/')
            $record = @($script:Documents | Where-Object { $_.RelativePath -ceq $name })
            $stream = $entry.Open()
            try { $hash = Get-Sha256HexFromStream $stream } finally { $stream.Dispose() }
            $entryRows += [pscustomobject]@{ path = $name; length = [string]$entry.Length; sha256 = $hash }
            if ($record.Count -ne 1) {
                $issues += "Unexpected or duplicate archive entry: $name"
            }
            else {
                if ($hash -cne $record[0].Sha256) { $issues += "Archive entry SHA-256 mismatch: $name" }
                if ([Int64]$entry.Length -ne [Int64]$record[0].Size) { $issues += "Archive entry size mismatch: $name" }
            }
        }
        $missing = @($expectedNames | Where-Object { $_ -cnotin $entryNames })
        $duplicates = @($entryNames | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
        if ($missing.Count -gt 0) { $issues += 'Archive entries missing: ' + ($missing -join ', ') }
        if ($duplicates.Count -gt 0) { $issues += 'Duplicate archive entries: ' + ($duplicates -join ', ') }
        if ($entries.Count -ne 25) { $issues += "Archive file entry count is $($entries.Count), expected 25." }
    }
    finally {
        $archive.Dispose()
    }

    $metric = [ordered]@{
        archiveEntryCount = $entryRows.Count
        expectedEntryCount = 25
        entries = $entryRows
        issueCount = $issues.Count
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'DOCUMENT-SET-00-24' $exactCommand "$baseTool; UTF-8 strict decoder" {
    $issues = @()
    $prefixes = @()
    $script:DocumentTextByPrefix = @{}
    $script:DocumentRecordByPrefix = @{}
    foreach ($record in $script:Documents) {
        $name = [IO.Path]::GetFileName($record.RelativePath)
        if ($name -notmatch '^(0[0-9]|1[0-9]|2[0-4])-.*\.md$') {
            $issues += "Invalid document prefix or extension: $name"
            continue
        }
        $prefix = $matches[1]
        $prefixes += $prefix
        if ($script:DocumentTextByPrefix.ContainsKey($prefix)) {
            $issues += "Duplicate document prefix: $prefix"
            continue
        }
        try {
            $script:DocumentTextByPrefix[$prefix] = Read-Utf8Text $record.LiteralPath
            $script:DocumentRecordByPrefix[$prefix] = $record
        }
        catch {
            $issues += "UTF-8 decode failed for ${name}: $($_.Exception.Message)"
        }
    }
    $expected = @(0..24 | ForEach-Object { $_.ToString('00') })
    $missing = @($expected | Where-Object { $_ -cnotin $prefixes })
    $extra = @($prefixes | Where-Object { $_ -cnotin $expected })
    $duplicates = @($prefixes | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
    if ($missing.Count -gt 0) { $issues += 'Missing prefixes: ' + ($missing -join ', ') }
    if ($extra.Count -gt 0) { $issues += 'Unexpected prefixes: ' + ($extra -join ', ') }
    if ($duplicates.Count -gt 0) { $issues += 'Duplicate prefixes: ' + ($duplicates -join ', ') }

    $metric = [ordered]@{
        documentCount = $script:Documents.Count
        decodedDocumentCount = $script:DocumentTextByPrefix.Count
        uniquePrefixCount = @($prefixes | Sort-Object -Unique).Count
        missingPrefixes = $missing
        extraPrefixes = $extra
        duplicatePrefixes = $duplicates
        issueCount = $issues.Count
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'FRONTMATTER-PHASE-CONTRACT' $exactCommand "$baseTool; deterministic frontmatter parser" {
    if ($null -eq $script:Manifest) {
        return New-Outcome $false ([ordered]@{ parsed = 0; expected = 25 }) @('Manifest prerequisite failed.')
    }
    $phase = [string](Get-ObjectPropertyValue $script:Manifest 'phase')
    $expectedStatus = if ($phase -ceq 'R3B') { 'reviewed' } else { 'draft' }
    $scoreSuffix = if ($phase -ceq 'R3B') { 'R3B' } else { 'R2' }
    $issues = @()
    $script:FrontMatterByPrefix = @{}
    $docIds = @()

    foreach ($prefix in ($script:DocumentTextByPrefix.Keys | Sort-Object)) {
        $frontMatter = Parse-FrontMatter $script:DocumentTextByPrefix[$prefix]
        $script:FrontMatterByPrefix[$prefix] = $frontMatter
        if (-not $frontMatter.Valid) { $issues += "$prefix frontmatter: $($frontMatter.Error)" }
        if ($frontMatter.DocId -notmatch '^F2S-DOC-[A-Z0-9-]+-001$') { $issues += "$prefix has invalid doc_id." }
        if ($frontMatter.Revision -notmatch '^[0-9]+\.[0-9]+$') { $issues += "$prefix has invalid revision." }
        if ($frontMatter.Status -cne $expectedStatus) { $issues += "$prefix status must be $expectedStatus for $phase." }
        if ($frontMatter.LastVerified -notmatch '^20[0-9]{2}-[0-9]{2}-[0-9]{2}$') { $issues += "$prefix has invalid last_verified." }
        if ($frontMatter.CanonicalFor.Count -eq 0) { $issues += "$prefix canonical_for is empty." }

        $expectedScore = $null
        if ($frontMatter.DocId -match '^F2S-(.+)$') {
            $expectedScore = 'F2S-SCORE-' + $matches[1] + '-' + $scoreSuffix
        }
        if ($frontMatter.ReviewScoreRef -cne $expectedScore) {
            $issues += "$prefix review_score_ref must be $expectedScore."
        }

        $record = $script:DocumentRecordByPrefix[$prefix]
        if ($null -ne $record) {
            if ($frontMatter.DocId -cne $record.DocId) { $issues += "$prefix doc_id differs from manifest." }
            if ($frontMatter.Revision -cne $record.Revision) { $issues += "$prefix revision differs from manifest." }
        }
        $docIds += $frontMatter.DocId
    }
    $duplicateDocIds = @($docIds | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
    if ($duplicateDocIds.Count -gt 0) { $issues += 'Duplicate frontmatter doc_ids: ' + ($duplicateDocIds -join ', ') }
    if ($script:FrontMatterByPrefix.Count -ne 25) { $issues += "Parsed frontmatter count is $($script:FrontMatterByPrefix.Count), expected 25." }

    $metric = [ordered]@{
        phase = $phase
        expectedStatus = $expectedStatus
        expectedScoreSuffix = $scoreSuffix
        parsedDocumentCount = $script:FrontMatterByPrefix.Count
        uniqueDocIdCount = @($docIds | Sort-Object -Unique).Count
        duplicateDocIds = $duplicateDocIds
        issueCount = $issues.Count
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'MARKDOWN-FENCE-BALANCE' $exactCommand "$baseTool; regex" {
    $issues = @()
    $counts = @()
    foreach ($prefix in ($script:DocumentTextByPrefix.Keys | Sort-Object)) {
        $count = [regex]::Matches($script:DocumentTextByPrefix[$prefix], '(?m)^\s*```').Count
        $counts += [pscustomobject]@{ prefix = $prefix; fenceLineCount = $count }
        if (($count % 2) -ne 0) { $issues += "$prefix has an odd fenced-code delimiter count: $count." }
    }
    return New-Outcome ($issues.Count -eq 0) ([ordered]@{ documents = $counts; issueCount = $issues.Count }) $issues
}

Invoke-AuditCheck 'MERGE-CONFLICT-MARKERS' $exactCommand "$baseTool; regex" {
    $hits = @()
    foreach ($prefix in ($script:DocumentTextByPrefix.Keys | Sort-Object)) {
        $matchesFound = [regex]::Matches(
            $script:DocumentTextByPrefix[$prefix],
            '(?m)^(?:<<<<<<<(?: .*)?|=======|>>>>>>>(?: .*)?)\s*$'
        )
        foreach ($hit in $matchesFound) {
            $hits += [pscustomobject]@{ prefix = $prefix; marker = $hit.Value.Trim() }
        }
    }
    $details = @($hits | ForEach-Object { "$($_.prefix): $($_.marker)" })
    return New-Outcome ($hits.Count -eq 0) ([ordered]@{ conflictMarkerCount = $hits.Count; hits = $hits }) $details
}

Invoke-AuditCheck 'CANONICAL-FOR-REGISTRY' $exactCommand "$baseTool; frontmatter registry reducer" {
    $tokens = @()
    foreach ($prefix in ($script:FrontMatterByPrefix.Keys | Sort-Object)) {
        $tokens += @($script:FrontMatterByPrefix[$prefix].CanonicalFor)
    }
    $duplicates = @($tokens | Group-Object -CaseSensitive | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
    $invalid = @($tokens | Where-Object { $_ -notmatch '^F2S-[A-Z0-9-]+$' })
    $issues = @()
    if ($tokens.Count -ne 384) { $issues += "canonical_for token count is $($tokens.Count), expected 384." }
    if ($duplicates.Count -gt 0) { $issues += 'Duplicate canonical_for tokens: ' + ($duplicates -join ', ') }
    if ($invalid.Count -gt 0) { $issues += 'Invalid canonical_for tokens: ' + ($invalid -join ', ') }
    $metric = [ordered]@{
        tokenCount = $tokens.Count
        expectedTokenCount = 384
        uniqueTokenCount = @($tokens | Sort-Object -Unique).Count
        duplicateCount = $duplicates.Count
        duplicates = $duplicates
        invalidCount = $invalid.Count
        invalid = $invalid
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'DEPENDS-ON-GRAPH' $exactCommand "$baseTool; deterministic graph reduction" {
    $issues = @()
    $known = @{}
    foreach ($prefix in $script:FrontMatterByPrefix.Keys) {
        $fm = $script:FrontMatterByPrefix[$prefix]
        if ($null -ne $fm.DocId) { $known[$fm.DocId] = $fm }
    }
    $dangling = @()
    $duplicateEdges = @()
    foreach ($docId in $known.Keys) {
        $deps = @($known[$docId].DependsOn)
        $duplicateEdges += @($deps | Group-Object -CaseSensitive | Where-Object { $_.Count -gt 1 } | ForEach-Object { "$docId->$($_.Name)" })
        foreach ($dep in $deps) {
            if (-not $known.ContainsKey($dep)) { $dangling += "$docId->$dep" }
        }
    }

    $remaining = @{}
    foreach ($docId in $known.Keys) { $remaining[$docId] = $true }
    do {
        $removable = @()
        foreach ($docId in @($remaining.Keys)) {
            $remainingDeps = @($known[$docId].DependsOn | Where-Object { $remaining.ContainsKey($_) })
            if ($remainingDeps.Count -eq 0) { $removable += $docId }
        }
        foreach ($docId in $removable) { $remaining.Remove($docId) }
    } while ($removable.Count -gt 0)
    $cycleNodes = @($remaining.Keys | Sort-Object)

    if ($dangling.Count -gt 0) { $issues += 'Dangling depends_on edges: ' + ($dangling -join ', ') }
    if ($duplicateEdges.Count -gt 0) { $issues += 'Duplicate depends_on edges: ' + ($duplicateEdges -join ', ') }
    if ($cycleNodes.Count -gt 0) { $issues += 'Dependency cycle nodes: ' + ($cycleNodes -join ', ') }
    $metric = [ordered]@{
        nodeCount = $known.Count
        danglingEdgeCount = $dangling.Count
        danglingEdges = $dangling
        duplicateEdgeCount = $duplicateEdges.Count
        duplicateEdges = $duplicateEdges
        cycleNodeCount = $cycleNodes.Count
        cycleNodes = $cycleNodes
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'REQUIREMENTS-02-21-PARITY' $exactCommand "$baseTool; markdown table reducer" {
    if (-not $script:DocumentTextByPrefix.ContainsKey('02') -or -not $script:DocumentTextByPrefix.ContainsKey('21')) {
        return New-Outcome $false ([ordered]@{ requirements = 0 }) @('Documents 02 or 21 are unavailable.')
    }
    $source = Get-RequirementMap $script:DocumentTextByPrefix['02']
    $trace = Get-RequirementMap $script:DocumentTextByPrefix['21']
    $sourceIds = @($source.Map.Keys)
    $traceIds = @($trace.Map.Keys)
    $missing = @($sourceIds | Where-Object { -not $trace.Map.ContainsKey($_) } | Sort-Object)
    $extra = @($traceIds | Where-Object { -not $source.Map.ContainsKey($_) } | Sort-Object)
    $priorityDiff = @($sourceIds | Where-Object { $trace.Map.ContainsKey($_) -and $source.Map[$_] -cne $trace.Map[$_] } | Sort-Object)
    $fr = @($sourceIds | Where-Object { $_ -like 'F2S-FR-*' }).Count
    $nfr = @($sourceIds | Where-Object { $_ -like 'F2S-NFR-*' }).Count
    $p0 = @($source.Map.Values | Where-Object { $_ -ceq 'P0' }).Count
    $p1 = @($source.Map.Values | Where-Object { $_ -ceq 'P1' }).Count
    $p2 = @($source.Map.Values | Where-Object { $_ -ceq 'P2' }).Count
    $issues = @()
    if ($fr -ne 69) { $issues += "FR count is $fr, expected 69." }
    if ($nfr -ne 33) { $issues += "NFR count is $nfr, expected 33." }
    if ($p0 -ne 96) { $issues += "P0 count is $p0, expected 96." }
    if ($p1 -ne 6) { $issues += "P1 count is $p1, expected 6." }
    if ($p2 -ne 0) { $issues += "P2 baseline count is $p2, expected 0." }
    if ($source.Duplicates.Count -gt 0) { $issues += 'Duplicate 02 requirements: ' + ($source.Duplicates -join ', ') }
    if ($trace.Duplicates.Count -gt 0) { $issues += 'Duplicate 21 requirements: ' + ($trace.Duplicates -join ', ') }
    if ($missing.Count -gt 0) { $issues += 'Missing from 21: ' + ($missing -join ', ') }
    if ($extra.Count -gt 0) { $issues += 'Extra in 21: ' + ($extra -join ', ') }
    if ($priorityDiff.Count -gt 0) { $issues += 'Priority differences: ' + ($priorityDiff -join ', ') }
    $metric = [ordered]@{
        sourceTotal = $source.Map.Count
        traceTotal = $trace.Map.Count
        fr = $fr
        nfr = $nfr
        p0 = $p0
        p1 = $p1
        p2 = $p2
        sourceDuplicateCount = $source.Duplicates.Count
        traceDuplicateCount = $trace.Duplicates.Count
        missingCount = $missing.Count
        extraCount = $extra.Count
        priorityDifferenceCount = $priorityDiff.Count
        missing = $missing
        extra = $extra
        priorityDifferences = $priorityDiff
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'EXACT-TESTS-OWNER-TRACE-PARITY' $exactCommand "$baseTool; exact-ID registry reducer" {
    $ownerPrefixes = @('06', '07', '08', '09', '10', '11', '12', '15')
    $pattern = 'F2S-TST-[A-Z0-9]+(?:-[A-Z0-9]+)*-[0-9]{3}|F2S-TST-[0-9]{3}'
    $ownerIds = @()
    foreach ($prefix in $ownerPrefixes) {
        if ($script:DocumentTextByPrefix.ContainsKey($prefix)) {
            $ownerIds += Get-UniqueRegexMatches $script:DocumentTextByPrefix[$prefix] $pattern
        }
    }
    $ownerIds = @($ownerIds | Sort-Object -Unique)
    $traceIds = if ($script:DocumentTextByPrefix.ContainsKey('21')) {
        @(Get-UniqueRegexMatches $script:DocumentTextByPrefix['21'] $pattern)
    } else { @() }
    $missing = @($ownerIds | Where-Object { $_ -cnotin $traceIds } | Sort-Object)
    $extra = @($traceIds | Where-Object { $_ -cnotin $ownerIds } | Sort-Object)
    $issues = @()
    if ($ownerIds.Count -ne 133) { $issues += "Owner exact-test count is $($ownerIds.Count), expected 133." }
    if ($traceIds.Count -ne 133) { $issues += "Trace exact-test count is $($traceIds.Count), expected 133." }
    if ($missing.Count -gt 0) { $issues += 'Tests missing from 21: ' + ($missing -join ', ') }
    if ($extra.Count -gt 0) { $issues += 'Tests extra in 21: ' + ($extra -join ', ') }
    $metric = [ordered]@{
        ownerPrefixes = $ownerPrefixes
        ownerExactTestCount = $ownerIds.Count
        traceExactTestCount = $traceIds.Count
        missingCount = $missing.Count
        extraCount = $extra.Count
        missing = $missing
        extra = $extra
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'DEV-EVD-21-SUFFIX-PARITY' $exactCommand "$baseTool; exact-ID registry reducer" {
    $text = if ($script:DocumentTextByPrefix.ContainsKey('21')) { $script:DocumentTextByPrefix['21'] } else { '' }
    $devIds = @(Get-UniqueRegexMatches $text 'F2S-DEV-M[0-9]{2}-[0-9]{3}')
    $evdIds = @(Get-UniqueRegexMatches $text 'F2S-EVD-M[0-9]{2}-[0-9]{3}')
    $devSuffix = @($devIds | ForEach-Object { $_.Substring('F2S-DEV-'.Length) } | Sort-Object -Unique)
    $evdSuffix = @($evdIds | ForEach-Object { $_.Substring('F2S-EVD-'.Length) } | Sort-Object -Unique)
    $missingEvd = @($devSuffix | Where-Object { $_ -cnotin $evdSuffix })
    $extraEvd = @($evdSuffix | Where-Object { $_ -cnotin $devSuffix })
    $issues = @()
    if ($devIds.Count -ne 80) { $issues += "DEV count is $($devIds.Count), expected 80." }
    if ($evdIds.Count -ne 80) { $issues += "EVD count is $($evdIds.Count), expected 80." }
    if ($missingEvd.Count -gt 0) { $issues += 'DEV suffixes without EVD: ' + ($missingEvd -join ', ') }
    if ($extraEvd.Count -gt 0) { $issues += 'EVD suffixes without DEV: ' + ($extraEvd -join ', ') }
    $metric = [ordered]@{
        devCount = $devIds.Count
        evdCount = $evdIds.Count
        missingEvdSuffixCount = $missingEvd.Count
        extraEvdSuffixCount = $extraEvd.Count
        missingEvdSuffixes = $missingEvd
        extraEvdSuffixes = $extraEvd
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

$fixedHashes = @(
    [pscustomobject]@{ CheckId = 'HASH-01-WIRE-ARTIFACT'; Prefix = '09'; Marker = '{"approvalRecordKinds":'; Mode = 'artifact'; Domain = $null; Expected = '57bab593f62917d35b7eba88148f0c95b037a40b2a6be1962f8068f4e0bf8d55' },
    [pscustomobject]@{ CheckId = 'HASH-02-DECISION-ARTIFACT'; Prefix = '09'; Marker = '"hashAlgorithmId":"f2s-decision-policy-sha256-jcs-v1"'; Mode = 'artifact'; Domain = $null; Expected = '56ae4052b605dae307b091c8133fb896835bba573c0555377f67d615060d6dad' },
    [pscustomobject]@{ CheckId = 'HASH-03-DECISION-DOMAIN'; Prefix = '09'; Marker = '"hashAlgorithmId":"f2s-decision-policy-sha256-jcs-v1"'; Mode = 'domain'; Domain = 'f2s-decision-policy-v1'; Expected = '48b5f0f91046ec69494794fad06154aa6aeada1c82e638607b06370e576ba4e9' },
    [pscustomobject]@{ CheckId = 'HASH-04-RELEASE-WAIVER-ARTIFACT'; Prefix = '18'; Marker = '"hashAlgorithmId":"f2s-release-waiver-policy-sha256-jcs-v1"'; Mode = 'artifact'; Domain = $null; Expected = 'bb224e3c46e9b73e34ffc562dc62a7167fc4764e592e4fde75152b178d7863c7' },
    [pscustomobject]@{ CheckId = 'HASH-05-RELEASE-WAIVER-DOMAIN'; Prefix = '18'; Marker = '"hashAlgorithmId":"f2s-release-waiver-policy-sha256-jcs-v1"'; Mode = 'domain'; Domain = 'f2s-release-waiver-policy-v1'; Expected = '830c2a9ff364db19686ac34ec0800fda89599a74e567fd8cfdc5568e1332c60d' },
    [pscustomobject]@{ CheckId = 'HASH-06-ACTOR-POLICY-ARTIFACT'; Prefix = '18'; Marker = '"hashAlgorithmId":"f2s-actor-attestation-policy-sha256-jcs-v1"'; Mode = 'artifact'; Domain = $null; Expected = '561d25223c507fe89f680ac5a1327c2c9500a85524e6929c90ae8d79eeef9cad' },
    [pscustomobject]@{ CheckId = 'HASH-07-ACTOR-POLICY-DOMAIN'; Prefix = '18'; Marker = '"hashAlgorithmId":"f2s-actor-attestation-policy-sha256-jcs-v1"'; Mode = 'domain'; Domain = 'f2s-actor-attestation-policy-v1'; Expected = 'b370ab9a28a68c010297e706e71d62c063bc97167494a051439fc1b3328c0d90' },
    [pscustomobject]@{ CheckId = 'HASH-08-SIGNATURE-REGISTRY-ARTIFACT'; Prefix = '18'; Marker = '"hashAlgorithmId":"f2s-signature-algorithm-registry-sha256-jcs-v1"'; Mode = 'artifact'; Domain = $null; Expected = '43185e9a1a5bdf5a5ee4b6e3ce13a7c27852d1c8487ea9ec4125094cd54b5ac8' },
    [pscustomobject]@{ CheckId = 'HASH-09-SIGNATURE-REGISTRY-DOMAIN'; Prefix = '18'; Marker = '"hashAlgorithmId":"f2s-signature-algorithm-registry-sha256-jcs-v1"'; Mode = 'domain'; Domain = 'f2s-signature-algorithm-registry-v1'; Expected = '1419106463bf0193c2885c90ef68520914ea426d3c3ffe1848fce6b7acd77c40' }
)

foreach ($fixedHash in $fixedHashes) {
    $spec = $fixedHash
    Invoke-AuditCheck $spec.CheckId $exactCommand "$baseTool; UTF-8 bytes; .NET SHA256" {
        if (-not $script:DocumentTextByPrefix.ContainsKey($spec.Prefix)) {
            return New-Outcome $false ([ordered]@{ expected = $spec.Expected; actual = $null }) @("Document $($spec.Prefix) is unavailable.")
        }
        $lines = @($script:DocumentTextByPrefix[$spec.Prefix] -split "`r?`n")
        $candidates = @($lines | Where-Object { $_.StartsWith('{') -and $_.EndsWith('}') -and $_.Contains($spec.Marker) })
        $issues = @()
        $actual = $null
        if ($candidates.Count -ne 1) {
            $issues += "Expected one authoritative JSON line containing marker; found $($candidates.Count)."
        }
        else {
            $jsonBytes = $script:Utf8NoBom.GetBytes($candidates[0])
            if ($spec.Mode -ceq 'domain') {
                $domainBytes = $script:Utf8NoBom.GetBytes($spec.Domain + [char]0)
                $actual = Get-Sha256HexFromBytes (Join-ByteArrays $domainBytes $jsonBytes)
            }
            else {
                $actual = Get-Sha256HexFromBytes $jsonBytes
            }
            if ($actual -cne $spec.Expected) { $issues += 'Recomputed SHA-256 differs from the frozen value.' }
        }
        $metric = [ordered]@{
            sourceDocumentPrefix = $spec.Prefix
            mode = $spec.Mode
            domainSeparator = $spec.Domain
            candidateLineCount = $candidates.Count
            byteLength = if ($candidates.Count -eq 1) { $script:Utf8NoBom.GetByteCount($candidates[0]) } else { $null }
            expectedSha256 = $spec.Expected
            actualSha256 = $actual
        }
        return New-Outcome ($issues.Count -eq 0) $metric $issues
    }
}

Invoke-AuditCheck 'POLICY-LICENSE-ALLOWLIST' $exactCommand "$baseTool; canonical fenced-block parser" {
    $text = if ($script:DocumentTextByPrefix.ContainsKey('14')) { $script:DocumentTextByPrefix['14'] } else { '' }
    $match = [regex]::Match(
        $text,
        '(?s)F2S-LIC-POLICY-001.*?```text\r?\n(?<body>.*?)\r?\n```'
    )
    $expected = @('MIT', 'Apache-2.0', 'BSD-2-Clause', 'BSD-3-Clause', 'ISC', 'Zlib', '0BSD', 'PSF-2.0', 'CC0-1.0', 'LicenseRef-Public-Domain')
    $actual = @()
    $issues = @()
    if (-not $match.Success) {
        $issues += 'Canonical F2S-LIC-POLICY-001 fenced allowlist was not found.'
    }
    else {
        $actual = @($match.Groups['body'].Value -split "`r?`n" | ForEach-Object { $_.Trim() } | Where-Object { $_.Length -gt 0 })
        if (($actual -join "`n") -cne ($expected -join "`n")) { $issues += 'License allowlist differs from the frozen ordered set.' }
    }
    $forbiddenNames = @('GPL', 'AGPL', 'SSPL', 'LGPL', 'MPL', 'EPL', 'CDDL')
    foreach ($name in $forbiddenNames) {
        if (-not $text.Contains($name)) { $issues += "Missing forbidden-license declaration for $name." }
    }
    $metric = [ordered]@{
        expected = $expected
        actual = $actual
        exactOrderedMatch = (($actual -join "`n") -ceq ($expected -join "`n"))
        forbiddenPolicyTokenCount = @($forbiddenNames | Where-Object { $text.Contains($_) }).Count
        issueCount = $issues.Count
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'POLICY-TLS-FAIL-CLOSED' $exactCommand "$baseTool; requirement-row assertion" {
    $reqText = if ($script:DocumentTextByPrefix.ContainsKey('02')) { $script:DocumentTextByPrefix['02'] } else { '' }
    $secText = if ($script:DocumentTextByPrefix.ContainsKey('14')) { $script:DocumentTextByPrefix['14'] } else { '' }
    $row = [regex]::Match($reqText, '(?m)^\|\s*`?F2S-NFR-SEC-004`?\s*\|\s*P0\s*\|.*$')
    $issues = @()
    if (-not $row.Success) { $issues += 'P0 F2S-NFR-SEC-004 row is missing.' }
    elseif (-not $row.Value.Contains('TLS')) { $issues += 'F2S-NFR-SEC-004 does not bind TLS.' }
    if (-not $secText.Contains('TLS 1.2+')) { $issues += 'The security plan lacks the TLS 1.2+ floor.' }
    if (-not $secText.Contains('mTLS')) { $issues += 'The security plan lacks the mTLS option.' }
    $metric = [ordered]@{
        requirementRowFound = $row.Success
        requirementPriorityP0 = ($row.Success -and $row.Value -match '\|\s*P0\s*\|')
        requirementBindsTls = ($row.Success -and $row.Value.Contains('TLS'))
        securityTlsFloorFound = $secText.Contains('TLS 1.2+')
        mTlsOptionFound = $secText.Contains('mTLS')
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'POLICY-WINDOWS-APPCONTAINER' $exactCommand "$baseTool; requirement/profile assertion" {
    $reqText = if ($script:DocumentTextByPrefix.ContainsKey('02')) { $script:DocumentTextByPrefix['02'] } else { '' }
    $secText = if ($script:DocumentTextByPrefix.ContainsKey('14')) { $script:DocumentTextByPrefix['14'] } else { '' }
    $row = [regex]::Match($reqText, '(?m)^\|\s*`?F2S-NFR-SEC-005`?\s*\|\s*P0\s*\|.*$')
    $requiredTokens = @('windows-appcontainer-v1', 'AppContainer', 'Job Object', 'network capability')
    $issues = @()
    if (-not $row.Success) { $issues += 'P0 F2S-NFR-SEC-005 row is missing.' }
    foreach ($token in $requiredTokens) {
        if (-not $reqText.Contains($token) -and -not $secText.Contains($token)) { $issues += "Missing sandbox token: $token" }
    }
    if ([regex]::Matches($secText, 'windows-appcontainer-v1').Count -lt 2) { $issues += 'Canonical windows-appcontainer-v1 profile is not sufficiently anchored in document 14.' }
    $metric = [ordered]@{
        requirementRowFound = $row.Success
        requirementPriorityP0 = ($row.Success -and $row.Value -match '\|\s*P0\s*\|')
        requiredTokens = $requiredTokens
        foundTokens = @($requiredTokens | Where-Object { $reqText.Contains($_) -or $secText.Contains($_) })
        canonicalProfileReferenceCount = [regex]::Matches($secText, 'windows-appcontainer-v1').Count
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'POLICY-P0-P1-NO-WAIVER' $exactCommand "$baseTool; policy-byte and prose assertion" {
    $text = if ($script:DocumentTextByPrefix.ContainsKey('18')) { $script:DocumentTextByPrefix['18'] } else { '' }
    $issues = @()
    $p0NearWaiver = [regex]::IsMatch($text, '(?im)P0.{0,120}waiver')
    $p1NearWaiver = [regex]::IsMatch($text, '(?im)P1.{0,120}waiver')
    $p2OnlyBytes = $text.Contains('{"allowedPriorities":["p2"]')
    $forbiddenCategories = $text.Contains('"forbiddenCategories"')
    if (-not $p0NearWaiver) { $issues += 'No explicit P0/waiver prohibition statement was found.' }
    if (-not $p1NearWaiver) { $issues += 'No explicit P1/waiver prohibition statement was found.' }
    if (-not $p2OnlyBytes) { $issues += 'ReleaseWaiverPolicyV1 is not visibly restricted to p2.' }
    if (-not $forbiddenCategories) { $issues += 'ReleaseWaiverPolicyV1 forbiddenCategories are missing.' }
    $metric = [ordered]@{
        p0WaiverRuleFound = $p0NearWaiver
        p1WaiverRuleFound = $p1NearWaiver
        allowedPrioritiesExactlyP2 = $p2OnlyBytes
        forbiddenCategoriesFound = $forbiddenCategories
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'POLICY-SPINE-4-2-43' $exactCommand "$baseTool; exact-version assertion" {
    $reqText = if ($script:DocumentTextByPrefix.ContainsKey('02')) { $script:DocumentTextByPrefix['02'] } else { '' }
    $exportText = if ($script:DocumentTextByPrefix.ContainsKey('13')) { $script:DocumentTextByPrefix['13'] } else { '' }
    $issues = @()
    $reqCount = [regex]::Matches($reqText, '4\.2\.43').Count
    $exportCount = [regex]::Matches($exportText, '4\.2\.43').Count
    if ($reqCount -lt 2) { $issues += 'Document 02 does not freeze Spine 4.2.43 in both output and CLI requirements.' }
    if ($exportCount -lt 3) { $issues += 'Document 13 does not sufficiently anchor Spine 4.2.43 export behavior.' }
    if (-not $reqText.Contains('F2S-FR-EXP-003') -or -not $reqText.Contains('F2S-FR-EXP-005')) { $issues += 'Required export requirement IDs are missing.' }
    $metric = [ordered]@{
        requirementsReferenceCount = $reqCount
        exportPlanReferenceCount = $exportCount
        exp003Found = $reqText.Contains('F2S-FR-EXP-003')
        exp005Found = $reqText.Contains('F2S-FR-EXP-005')
        fixedPatch = '4.2.43'
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'POLICY-BUILTIN-WRITER-FORBIDDEN' $exactCommand "$baseTool; export-boundary assertion" {
    $reqText = if ($script:DocumentTextByPrefix.ContainsKey('02')) { $script:DocumentTextByPrefix['02'] } else { '' }
    $exportText = if ($script:DocumentTextByPrefix.ContainsKey('13')) { $script:DocumentTextByPrefix['13'] } else { '' }
    $combined = $reqText + "`n" + $exportText
    $issues = @()
    $writerArtifactRule = [regex]::IsMatch($combined, '(?im)writer.{0,120}\.atlas/\.spine/\.skel')
    $fallbackRule = [regex]::IsMatch($combined, '(?im)fallback writer')
    $cliRule = $combined.Contains('Professional CLI')
    if (-not $writerArtifactRule) { $issues += 'Built-in writer prohibition for .atlas/.spine/.skel was not found.' }
    if (-not $fallbackRule) { $issues += 'Fallback writer prohibition was not found.' }
    if (-not $cliRule) { $issues += 'Professional CLI boundary was not found.' }
    $metric = [ordered]@{
        writerArtifactProhibitionFound = $writerArtifactRule
        fallbackWriterProhibitionFound = $fallbackRule
        professionalCliBoundaryFound = $cliRule
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'POLICY-STRICT-ULID' $exactCommand "$baseTool; literal grammar assertion" {
    $text = if ($script:DocumentTextByPrefix.ContainsKey('18')) { $script:DocumentTextByPrefix['18'] } else { '' }
    $grammar = '^[0-7][0-9A-HJKMNP-TV-Z]{25}$'
    $issues = @()
    $grammarFound = $text.Contains($grammar)
    $forbiddenAlphabet = $text.Contains('I/L/O/U')
    $monotonic = [regex]::IsMatch($text, '(?i)monotonic ULID')
    $roundTrip = [regex]::IsMatch($text, '(?i)ULID.{0,500}roundtrip|roundtrip.{0,500}ULID')
    if (-not $grammarFound) { $issues += 'Strict ULID grammar is missing.' }
    if (-not $forbiddenAlphabet) { $issues += 'Forbidden Crockford aliases are not declared.' }
    if (-not $monotonic) { $issues += 'Monotonic ULID generation rule is missing.' }
    if (-not $roundTrip) { $issues += 'ULID exact round-trip validation is missing.' }
    $metric = [ordered]@{
        grammar = $grammar
        grammarFound = $grammarFound
        forbiddenAliasRuleFound = $forbiddenAlphabet
        monotonicRuleFound = $monotonic
        roundTripRuleFound = $roundTrip
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

Invoke-AuditCheck 'POLICY-COMMANDKIND-HASH-DOMAIN' $exactCommand "$baseTool; canonical hash-domain assertion" {
    $domainText = if ($script:DocumentTextByPrefix.ContainsKey('09')) { $script:DocumentTextByPrefix['09'] } else { '' }
    $releaseText = if ($script:DocumentTextByPrefix.ContainsKey('18')) { $script:DocumentTextByPrefix['18'] } else { '' }
    $qualityText = if ($script:DocumentTextByPrefix.ContainsKey('15')) { $script:DocumentTextByPrefix['15'] } else { '' }
    $combined = $domainText + "`n" + $releaseText + "`n" + $qualityText
    $issues = @()
    $payloadFormula = $combined.Contains('JCS({commandKind')
    $domainSeparator = $combined.Contains('f2s-actor-registry-command-payload-v1\0')
    $commandKindReferences = [regex]::Matches($combined, 'commandKind').Count
    $canonicalPayloadReferences = [regex]::Matches($combined, 'canonicalPayloadHash').Count
    $crossKindRule = [regex]::IsMatch($combined, '(?im)cross.kind|same key across command kinds|commandKind.{0,300}different-payload|different-payload.{0,300}commandKind')
    if (-not $payloadFormula) { $issues += 'commandKind is absent from the canonical JCS payload formula.' }
    if (-not $domainSeparator) { $issues += 'The command payload domain separator is missing.' }
    if ($commandKindReferences -lt 5) { $issues += 'commandKind closed-enum/hash contract is insufficiently anchored.' }
    if ($canonicalPayloadReferences -lt 3) { $issues += 'canonicalPayloadHash contract is insufficiently anchored.' }
    if (-not $crossKindRule) { $issues += 'Cross-command-kind idempotency conflict rule is missing.' }
    $metric = [ordered]@{
        payloadFormulaIncludesCommandKind = $payloadFormula
        domainSeparatorFound = $domainSeparator
        commandKindReferenceCount = $commandKindReferences
        canonicalPayloadHashReferenceCount = $canonicalPayloadReferences
        crossKindConflictRuleFound = $crossKindRule
    }
    return New-Outcome ($issues.Count -eq 0) $metric $issues
}

$completedAt = Get-AsiaShanghaiTimestamp
$failedChecks = @($script:CheckResults | Where-Object { $_.status -cne 'PASS' })
$overallStatus = if ($failedChecks.Count -eq 0) { 'PASS' } else { 'FAIL' }
$phaseValue = if ($null -ne $script:Manifest) { Get-ObjectPropertyValue $script:Manifest 'phase' } else { $null }
$manifestSha256 = if (Test-Path -LiteralPath $manifestPath -PathType Leaf) { Get-Sha256HexFromFile $manifestPath } else { $null }

$report = [ordered]@{
    schemaVersion = '1.0.0'
    auditType = 'mechanical-plan-audit'
    auditVersion = $script:AuditVersion
    snapshotId = $SnapshotId
    phase = $phaseValue
    status = $overallStatus
    generatedAtAsiaShanghai = $completedAt
    input = [ordered]@{
        manifestPath = Get-WorkspaceRelativePath $script:Workspace $manifestPath
        manifestSha256 = $manifestSha256
        snapshotPath = Get-WorkspaceRelativePath $script:Workspace $snapshotRoot
    }
    generator = [ordered]@{
        scriptPath = Get-WorkspaceRelativePath $script:Workspace $scriptPath
        scriptSha256 = Get-Sha256HexFromFile $scriptPath
        runtimePath = $runtimePath
        runtimeVersion = $runtimeVersion
        runtimeSha256 = Get-Sha256HexFromFile $runtimePath
        exactCommand = $exactCommand
    }
    summary = [ordered]@{
        checkCount = $script:CheckResults.Count
        passCount = @($script:CheckResults | Where-Object { $_.status -ceq 'PASS' }).Count
        failCount = @($script:CheckResults | Where-Object { $_.status -ceq 'FAIL' }).Count
        errorCount = @($script:CheckResults | Where-Object { $_.status -ceq 'ERROR' }).Count
    }
    checks = $script:CheckResults
}

$jsonPath = Join-Path $script:StagingAuditRoot 'mechanical-audit.json'
$textPath = Join-Path $script:StagingAuditRoot 'mechanical-audit.txt'
Write-NewUtf8File $jsonPath (($report | ConvertTo-Json -Depth 50) + [Environment]::NewLine)

$summaryText = New-Object Text.StringBuilder
$null = $summaryText.AppendLine('Flash to Spine plan mechanical audit')
$null = $summaryText.AppendLine("SnapshotId: $SnapshotId")
$null = $summaryText.AppendLine("Phase: $phaseValue")
$null = $summaryText.AppendLine("Status: $overallStatus")
$null = $summaryText.AppendLine("GeneratedAtAsiaShanghai: $completedAt")
$null = $summaryText.AppendLine("ManifestSha256: $manifestSha256")
$null = $summaryText.AppendLine("Checks: $($report.summary.checkCount); PASS=$($report.summary.passCount); FAIL=$($report.summary.failCount); ERROR=$($report.summary.errorCount)")
$null = $summaryText.AppendLine('')
foreach ($check in $script:CheckResults) {
    $null = $summaryText.AppendLine("[$($check.status)] $($check.checkId)")
    $null = $summaryText.AppendLine("  exitCode: $($check.exitCode)")
    $null = $summaryText.AppendLine("  startedAtAsiaShanghai: $($check.startedAtAsiaShanghai)")
    $null = $summaryText.AppendLine("  endedAtAsiaShanghai: $($check.endedAtAsiaShanghai)")
    $null = $summaryText.AppendLine("  command: $($check.command)")
    $null = $summaryText.AppendLine("  tool: $($check.tool)")
    $null = $summaryText.AppendLine("  rawPath: $($check.rawPath)")
    $null = $summaryText.AppendLine("  rawSha256: $($check.rawSha256)")
}
Write-NewUtf8File $textPath $summaryText.ToString()

if (Test-Path -LiteralPath $script:FinalAuditRoot) {
    throw "Audit destination appeared during execution and will not be overwritten: $($script:FinalAuditRoot)"
}
Move-Item -LiteralPath $script:StagingAuditRoot -Destination $script:FinalAuditRoot
Get-ChildItem -LiteralPath $script:FinalAuditRoot -File -Recurse | ForEach-Object { $_.IsReadOnly = $true }

[pscustomobject][ordered]@{
    SnapshotId = $SnapshotId
    Phase = $phaseValue
    Status = $overallStatus
    Checks = $script:CheckResults.Count
    Passed = $report.summary.passCount
    Failed = $report.summary.failCount
    Errors = $report.summary.errorCount
    Json = Get-WorkspaceRelativePath $script:Workspace (Join-Path $script:FinalAuditRoot 'mechanical-audit.json')
    Text = Get-WorkspaceRelativePath $script:Workspace (Join-Path $script:FinalAuditRoot 'mechanical-audit.txt')
}
if ($overallStatus -cne 'PASS') {
    exit 1
}
