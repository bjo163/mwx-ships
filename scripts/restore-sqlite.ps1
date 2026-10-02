<#
.SYNOPSIS
  Moonships SQLite Restore Script (PowerShell)
.DESCRIPTION
  Safely restores Moonships state from an existing SQLite backup file into the target path.
  Performs PRAGMA integrity_check to verify database validity.
.PARAMETER BackupFile
  Path to the SQLite backup file to restore.
.PARAMETER TargetDb
  Target active database path (defaults to data/moonships.sqlite).
#>
param(
    [Parameter(Mandatory=$true)]
    [string]$BackupFile,
    [string]$TargetDb = "data/moonships.sqlite"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  MOONSHIPS SQLITE DATABASE RESTORE" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

if (-not (Test-Path $BackupFile)) {
    Write-Error "Backup file not found at: $BackupFile"
    exit 1
}

$targetDir = Split-Path -Parent $TargetDb
if ($targetDir -and (-not (Test-Path $targetDir))) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

# If existing target DB exists, create safety rollback copy
if (Test-Path $TargetDb) {
    $safetyCopy = "$TargetDb.pre-restore-$(Get-Date -Format 'yyyyMMddHHmmss')"
    Write-Host "Archiving existing active database to $safetyCopy..." -ForegroundColor Yellow
    Copy-Item -Path $TargetDb -Destination $safetyCopy -Force
}

Write-Host "Restoring from $BackupFile to $TargetDb..." -ForegroundColor Yellow
Copy-Item -Path $BackupFile -Destination $TargetDb -Force

# Remove any stale WAL/SHM from previous run
if (Test-Path "$TargetDb-wal") { Remove-Item -Path "$TargetDb-wal" -Force }
if (Test-Path "$TargetDb-shm") { Remove-Item -Path "$TargetDb-shm" -Force }

# Verify integrity if sqlite3 is installed
$sqliteCli = Get-Command sqlite3 -ErrorAction SilentlyContinue
if ($sqliteCli) {
    Write-Host "Running PRAGMA integrity_check..." -ForegroundColor Yellow
    $check = sqlite3 $TargetDb "PRAGMA integrity_check;"
    if ($check -eq "ok") {
        Write-Host "Integrity verification passed: ok" -ForegroundColor Green
    } else {
        Write-Error "Integrity check failed: $check"
        exit 1
    }
}

Write-Host "Restore operation completed successfully!" -ForegroundColor Green
