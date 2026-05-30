#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Bumps the Savant project version. Single source of truth: VERSION file at repo root.

.DESCRIPTION
    Reads the new version from the VERSION file (or from -NewVersion arg),
    then syncs it to:
    - Cargo.toml [workspace.package] (propagates to all 28 Rust crates)
    - dashboard/package.json
    - dashboard/package-lock.json
    - crates/desktop/src-tauri/tauri.conf.json

    If -NewVersion is provided, it writes the VERSION file first.
    If omitted, it reads whatever is already in VERSION.

.PARAMETER NewVersion
    The new version string (e.g., "0.5.0"). Writes to VERSION, then syncs.

.PARAMETER SkipVerify
    Skip the cargo check + tsc verification step.

.EXAMPLE
    # Bump to 0.5.0 (writes VERSION + syncs all 4 targets)
    ./scripts/bump-version.ps1 0.5.0

    # Sync from existing VERSION file (no arg)
    ./scripts/bump-version.ps1

    # Bump without verification
    ./scripts/bump-version.ps1 1.0.0 -SkipVerify
#>

param(
    [Parameter(Mandatory = $false, Position = 0)]
    [string]$NewVersion,

    [switch]$SkipVerify
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$VersionFile = Join-Path $Root "VERSION"

# ─── Resolve version ──────────────────────────────────────────────────
if ($NewVersion) {
    # Write new version to VERSION file (single source of truth)
    Set-Content $VersionFile -Value $NewVersion -NoNewline
    Write-Host "  VERSION file: $NewVersion" -ForegroundColor Cyan
} else {
    # Read from VERSION file
    if (-not (Test-Path $VersionFile)) {
        Write-Error "No VERSION file found and no -NewVersion argument provided."
        exit 1
    }
    $NewVersion = (Get-Content $VersionFile -Raw).Trim()
    Write-Host "  VERSION file: $NewVersion (existing)" -ForegroundColor Cyan
}

# ─── Detect old version from Cargo.toml ───────────────────────────────
$rootToml = Get-Content "$Root\Cargo.toml" -Raw
if ($rootToml -match '\[workspace\.package\][\s\S]*?version\s*=\s*"(\d+\.\d+\.\d+)"') {
    $OldVersion = $Matches[1]
} else {
    Write-Error "Could not detect current version from Cargo.toml [workspace.package]"
    exit 1
}

if ($OldVersion -eq $NewVersion) {
    Write-Host "  Already at v$NewVersion. Nothing to sync." -ForegroundColor DarkGray
    exit 0
}

Write-Host ""
Write-Host "  Savant Version Bump: $OldVersion -> $NewVersion" -ForegroundColor Yellow
Write-Host "  Source: VERSION file" -ForegroundColor DarkGray
Write-Host "  =============================================" -ForegroundColor Yellow
Write-Host ""

$updated = 0

# ─── 1. Root Cargo.toml [workspace.package] ───────────────────────────
$escapedOld = [regex]::Escape($OldVersion)
$newRootContent = $rootToml -replace "(\[workspace\.package\][\s\S]*?version\s*=\s*`")$escapedOld(`")", "`${1}$NewVersion`${2}"
if ($newRootContent -ne $rootToml) {
    Set-Content "$Root\Cargo.toml" -Value $newRootContent -NoNewline
    Write-Host "  OK  Cargo.toml [workspace.package] -> $NewVersion" -ForegroundColor Green
    Write-Host "      (28 Rust crates inherit automatically)" -ForegroundColor DarkGray
    $updated++
} else {
    Write-Error "Failed to update Cargo.toml"
    exit 1
}

# ─── 2. dashboard/package.json ────────────────────────────────────────
$pkgPath = Join-Path $Root "dashboard\package.json"
$pkgContent = Get-Content $pkgPath -Raw
$newPkgContent = $pkgContent -replace "`"$OldVersion`"", "`"$NewVersion`""
if ($newPkgContent -ne $pkgContent) {
    Set-Content $pkgPath -Value $newPkgContent -NoNewline
    Write-Host "  OK  dashboard\package.json -> $NewVersion" -ForegroundColor Green
    $updated++
}

# ─── 3. dashboard/package-lock.json ──────────────────────────────────
$lockPath = Join-Path $Root "dashboard\package-lock.json"
$lockContent = Get-Content $lockPath -Raw
$newLockContent = $lockContent -replace "`"$OldVersion`"", "`"$NewVersion`""
if ($newLockContent -ne $lockContent) {
    Set-Content $lockPath -Value $newLockContent -NoNewline
    Write-Host "  OK  dashboard\package-lock.json -> $NewVersion" -ForegroundColor Green
    $updated++
}

# ─── 4. Tauri config ─────────────────────────────────────────────────
$tauriPath = Join-Path $Root "crates\desktop\src-tauri\tauri.conf.json"
$tauriContent = Get-Content $tauriPath -Raw
$newTauriContent = $tauriContent -replace $escapedOld, $NewVersion
if ($newTauriContent -ne $tauriContent) {
    Set-Content $tauriPath -Value $newTauriContent -NoNewline
    Write-Host "  OK  crates\desktop\src-tauri\tauri.conf.json -> $NewVersion" -ForegroundColor Green
    $updated++
}

# ─── Summary ──────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  =============================================" -ForegroundColor Yellow
Write-Host "  Synced $updated files from VERSION -> v$NewVersion" -ForegroundColor Yellow
Write-Host "  =============================================" -ForegroundColor Yellow
Write-Host ""

# ─── Verify ───────────────────────────────────────────────────────────
if (-not $SkipVerify) {
    Write-Host "  Verifying..." -ForegroundColor Cyan

    Push-Location $Root
    $checkResult = & cargo check --workspace 2>&1
    Pop-Location
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  cargo check: PASS" -ForegroundColor Green
    } else {
        Write-Error "cargo check FAILED:`n$checkResult"
        exit 1
    }

    Push-Location "$Root\dashboard"
    $tscResult = & npx tsc --noEmit 2>&1
    Pop-Location
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  tsc: PASS" -ForegroundColor Green
    } else {
        Write-Error "tsc FAILED:`n$tscResult"
        exit 1
    }
}

Write-Host ""
Write-Host "  Done: $OldVersion -> $NewVersion" -ForegroundColor Green
Write-Host ""
