[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('R3A', 'R3B')][string]$Phase,
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Z0-9-]+$')][string]$SnapshotId,
    [Parameter(Mandatory = $true)][ValidatePattern('^F2S-REV-R3[AB]-[0-9]{3}$')][string]$EvidenceId,
    [string]$WorkspaceRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

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
    $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
}
$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$planRoot = Join-Path $workspace 'plan'
$reviewRoot = Join-Path $planRoot 'reviews'
$snapshotRoot = Join-Path (Join-Path $reviewRoot 'snapshots') $SnapshotId
$snapshotPlanRoot = Join-Path $snapshotRoot 'plan'
$archivePath = Join-Path (Join-Path $reviewRoot 'snapshots') ($SnapshotId + '.zip')
$manifestPath = Join-Path $snapshotRoot 'manifest.json'

if ((Test-Path -LiteralPath $snapshotRoot) -or (Test-Path -LiteralPath $archivePath)) {
    throw "Snapshot already exists and is immutable: $SnapshotId"
}

$sourceFiles = @(Get-ChildItem -LiteralPath $planRoot -File -Filter '*.md' | Sort-Object Name)
if ($sourceFiles.Count -ne 25) {
    throw "Expected 25 direct plan markdown files, found $($sourceFiles.Count)."
}

New-Item -ItemType Directory -Path $snapshotPlanRoot | Out-Null

$documents = @()
foreach ($source in $sourceFiles) {
    $text = Get-Content -LiteralPath $source.FullName -Raw -Encoding UTF8
    $docIdMatch = [regex]::Match($text, '(?m)^doc_id:\s*(\S+)\s*$')
    $revisionMatch = [regex]::Match($text, '(?m)^revision:\s*(\S+)\s*$')
    if (-not $docIdMatch.Success -or -not $revisionMatch.Success) {
        throw "Missing doc_id/revision in $($source.Name)."
    }

    $destination = Join-Path $snapshotPlanRoot $source.Name
    Copy-Item -LiteralPath $source.FullName -Destination $destination
    (Get-Item -LiteralPath $destination).IsReadOnly = $true
    $copied = Get-Item -LiteralPath $destination
    $documents += [ordered]@{
        docId = $docIdMatch.Groups[1].Value
        path = 'plan/' + $source.Name
        revision = $revisionMatch.Groups[1].Value
        size = [string]$copied.Length
        sha256 = Get-LowerSha256 $destination
    }
}

Compress-Archive -LiteralPath $snapshotPlanRoot -DestinationPath $archivePath -CompressionLevel Optimal
(Get-Item -LiteralPath $archivePath).IsReadOnly = $true

$powerShellPath = (Get-Process -Id $PID).Path
$powerShellVersion = $PSVersionTable.PSVersion.ToString()
$scriptPath = $MyInvocation.MyCommand.Path
$createdAt = [TimeZoneInfo]::ConvertTimeBySystemTimeZoneId([DateTimeOffset]::UtcNow, 'China Standard Time').ToString('yyyy-MM-ddTHH:mm:ss.fffffffzzz')
$exactCommand = "powershell -NoProfile -ExecutionPolicy Bypass -File plan/reviews/tools/New-PlanSnapshot.ps1 -Phase $Phase -SnapshotId $SnapshotId -EvidenceId $EvidenceId"

$manifest = [ordered]@{
    schemaVersion = '1.0.0'
    evidenceId = $EvidenceId
    phase = $Phase
    snapshotId = $SnapshotId
    createdAtAsiaShanghai = $createdAt
    archivePath = Get-WorkspaceRelativePath $workspace $archivePath
    archiveSha256 = Get-LowerSha256 $archivePath
    sourceRoot = 'plan'
    documentCount = [string]$documents.Count
    generator = [ordered]@{
        name = 'New-PlanSnapshot.ps1'
        version = '1.0.1'
        scriptPath = Get-WorkspaceRelativePath $workspace $scriptPath
        scriptSha256 = Get-LowerSha256 $scriptPath
        runtimePath = $powerShellPath
        runtimeVersion = $powerShellVersion
        runtimeSha256 = Get-LowerSha256 $powerShellPath
        exactCommand = $exactCommand
    }
    documents = $documents
}

$json = $manifest | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText($manifestPath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
(Get-Item -LiteralPath $manifestPath).IsReadOnly = $true

[pscustomobject]@{
    SnapshotId = $SnapshotId
    Manifest = Get-WorkspaceRelativePath $workspace $manifestPath
    ManifestSha256 = Get-LowerSha256 $manifestPath
    Archive = Get-WorkspaceRelativePath $workspace $archivePath
    ArchiveSha256 = Get-LowerSha256 $archivePath
    Documents = $documents.Count
}
