[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('D0', 'FINAL')][string]$Phase,
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Z0-9-]+$')][string]$SnapshotId,
    [Parameter(Mandatory = $true)][ValidatePattern('^F2S-REV-DEVPLAN-(D0|FINAL)-[0-9]{3}$')][string]$EvidenceId,
    [string]$WorkspaceRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (($Phase -ceq 'D0' -and $EvidenceId -notmatch '-D0-') -or ($Phase -ceq 'FINAL' -and $EvidenceId -notmatch '-FINAL-')) {
    throw "EvidenceId phase does not match Phase: $EvidenceId / $Phase"
}

function Get-LowerSha256([string]$LiteralPath) {
    return (Get-FileHash -LiteralPath $LiteralPath -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-WorkspaceRelativePath([string]$Root, [string]$Path) {
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    $pathFull = [IO.Path]::GetFullPath($Path)
    if (-not $pathFull.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes workspace: $pathFull"
    }
    return $pathFull.Substring($rootFull.Length).Replace('\', '/')
}

if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
}
$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$sourceRoot = Join-Path $workspace 'plan\devplan'
$reviewRoot = Join-Path $sourceRoot 'reviews'
$snapshotRoot = Join-Path (Join-Path $reviewRoot 'snapshots') $SnapshotId
$snapshotDocsRoot = Join-Path $snapshotRoot 'devplan'
$archivePath = Join-Path (Join-Path $reviewRoot 'snapshots') ($SnapshotId + '.zip')
$manifestPath = Join-Path $snapshotRoot 'manifest.json'

if ((Test-Path -LiteralPath $snapshotRoot) -or (Test-Path -LiteralPath $archivePath)) {
    throw "Snapshot already exists and is immutable: $SnapshotId"
}

$sourceFiles = @(Get-ChildItem -LiteralPath $sourceRoot -File -Filter '*.md' | Sort-Object Name)
if ($sourceFiles.Count -ne 16) {
    throw "Expected 16 direct devplan markdown files, found $($sourceFiles.Count)."
}

New-Item -ItemType Directory -Path $snapshotDocsRoot | Out-Null
$documents = @()
foreach ($source in $sourceFiles) {
    $text = Get-Content -LiteralPath $source.FullName -Raw -Encoding UTF8
    $docIdMatch = [regex]::Match($text, '(?m)^doc_id:\s*(\S+)\s*$')
    $revisionMatch = [regex]::Match($text, '(?m)^revision:\s*(\S+)\s*$')
    if (-not $docIdMatch.Success -or -not $revisionMatch.Success) {
        throw "Missing doc_id/revision in $($source.Name)."
    }
    $destination = Join-Path $snapshotDocsRoot $source.Name
    Copy-Item -LiteralPath $source.FullName -Destination $destination
    (Get-Item -LiteralPath $destination).IsReadOnly = $true
    $copy = Get-Item -LiteralPath $destination
    $documents += [ordered]@{
        docId = $docIdMatch.Groups[1].Value
        path = 'devplan/' + $source.Name
        revision = $revisionMatch.Groups[1].Value
        size = [string]$copy.Length
        sha256 = Get-LowerSha256 $destination
    }
}

Compress-Archive -LiteralPath $snapshotDocsRoot -DestinationPath $archivePath -CompressionLevel Optimal
(Get-Item -LiteralPath $archivePath).IsReadOnly = $true

$runtimePath = (Get-Process -Id $PID).Path
$scriptPath = $MyInvocation.MyCommand.Path
$createdAt = [TimeZoneInfo]::ConvertTimeBySystemTimeZoneId(
    [DateTimeOffset]::UtcNow,
    'China Standard Time'
).ToString('yyyy-MM-ddTHH:mm:ss.fffffffzzz')
$exactCommand = "powershell -NoProfile -ExecutionPolicy Bypass -File plan/devplan/reviews/tools/New-DevPlanSnapshot.ps1 -Phase $Phase -SnapshotId $SnapshotId -EvidenceId $EvidenceId"

$manifest = [ordered]@{
    schemaVersion = '1.0.0'
    evidenceId = $EvidenceId
    phase = $Phase
    snapshotId = $SnapshotId
    createdAtAsiaShanghai = $createdAt
    archivePath = Get-WorkspaceRelativePath $workspace $archivePath
    archiveSha256 = Get-LowerSha256 $archivePath
    sourceRoot = 'plan/devplan'
    documentCount = [string]$documents.Count
    generator = [ordered]@{
        name = 'New-DevPlanSnapshot.ps1'
        version = '1.0.0'
        scriptPath = Get-WorkspaceRelativePath $workspace $scriptPath
        scriptSha256 = Get-LowerSha256 $scriptPath
        runtimePath = $runtimePath
        runtimeVersion = $PSVersionTable.PSVersion.ToString()
        runtimeSha256 = Get-LowerSha256 $runtimePath
        exactCommand = $exactCommand
    }
    documents = $documents
}

[IO.File]::WriteAllText(
    $manifestPath,
    ($manifest | ConvertTo-Json -Depth 8) + [Environment]::NewLine,
    [Text.UTF8Encoding]::new($false)
)
(Get-Item -LiteralPath $manifestPath).IsReadOnly = $true

[pscustomobject]@{
    SnapshotId = $SnapshotId
    Manifest = Get-WorkspaceRelativePath $workspace $manifestPath
    ManifestSha256 = Get-LowerSha256 $manifestPath
    Archive = Get-WorkspaceRelativePath $workspace $archivePath
    ArchiveSha256 = Get-LowerSha256 $archivePath
    Documents = $documents.Count
}
