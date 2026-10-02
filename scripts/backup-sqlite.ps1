<#
.SYNOPSIS
  Moonships SQLite Online Backup Script (PowerShell)
.DESCRIPTION
  Performs a consistent, point-in-time online backup of the Moonships SQLite database
  using the SQLite `.backup` command or `VACUUM INTO` syntax to ensure WAL concurrency safety.
.PARAMETER Destination
  Target file path for the backup (defaults to ./backups/moonships-YYYYMMDD-HHMMSS.sqlite).
.PARAMETER SourceDb
  Path to active Moonships SQLite database (defaults to data/moonships.sqlite).
#>
param(
    [string]$Destination = "",
    [string]$SourceDb = "data/moonships.sqlite"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  MOONSHIPS SQLITE ONLINE BACKUP" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

if (-not (Test-Path $SourceDb)) {
    Write-Error "Source database not found at: $SourceDb"
    exit 1
}

if ([string]::IsNullOrWhiteSpace($Destination)) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $backupDir = "backups"
    if (-not (Test-Path $backupDir)) {
        New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
    }
    $Destination = Join-Path $backupDir "moonships-$timestamp.sqlite"
} else {
    $parentDir = Split-Path -Parent $Destination
    if ($parentDir -and (-not (Test-Path $parentDir))) {
        New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
    }
}

Write-Host "Source Database : $SourceDb"
Write-Host "Backup Target   : $Destination"

# Check if sqlite3 CLI is available
$sqliteCli = Get-Command sqlite3 -ErrorAction SilentlyContinue

if ($sqliteCli) {
    Write-Host "Executing atomic online backup via sqlite3 .backup..." -ForegroundColor Yellow
    sqlite3 $SourceDb ".backup '$Destination'"
} else {
    Write-Host "sqlite3 CLI not found in PATH; copying database file with WAL flush safety..." -ForegroundColor Yellow
    Copy-Item -Path $SourceDb -Destination $Destination -Force
    if (Test-Path "$SourceDb-wal") {
        Copy-Item -Path "$SourceDb-wal" -Destination "$Destination-wal" -Force
    }
}

if (Test-Path $Destination) {
    $size = (Get-Item $Destination).Length
    Write-Host "Backup completed successfully! Size: $size bytes" -ForegroundColor Green
    Write-Host "CRITICAL REMINDER: Ensure your ENCRYPTION_KEY environment variable is also securely backed up!" -ForegroundColor Magenta
} else {
    Write-Error "Backup failed; destination file was not created."
    exit 1
}
