<#
.SYNOPSIS
    Builds the stt_server.exe sidecar and copies it into the Tauri resources folder.

.DESCRIPTION
    Run this ONCE before building the Tauri release installer.
    The resulting exe is bundled into the Genie Recorder installer — end users
    never need Python installed.

.REQUIREMENTS
    - Python 3.10 or 3.11 in PATH (or activate your venv first)
    - pip install -r requirements-stt.txt
    - pip install pyinstaller

.EXAMPLE
    cd server
    pip install -r requirements-stt.txt
    pip install pyinstaller
    .\build_sidecar.ps1
#>

$ErrorActionPreference = "Stop"

$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot    = Split-Path -Parent $ScriptDir
$ResourceDir = Join-Path $RepoRoot "src-tauri\resources"
$DistExe     = Join-Path $ScriptDir "dist\stt_server\stt_server.exe"
$DestExe     = Join-Path $ResourceDir "stt_server.exe"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Salenie STT Server — Sidecar Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# --- Check PyInstaller is available ---
if (-not (Get-Command pyinstaller -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] pyinstaller not found. Install it:" -ForegroundColor Red
    Write-Host "        pip install pyinstaller" -ForegroundColor Yellow
    exit 1
}

# --- Run PyInstaller ---
Write-Host "[1/3] Building stt_server.exe with PyInstaller..." -ForegroundColor Green
Set-Location $ScriptDir
pyinstaller stt_server.spec --clean --noconfirm

if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] PyInstaller failed (exit $LASTEXITCODE)" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $DistExe)) {
    Write-Host "[ERROR] Expected output not found: $DistExe" -ForegroundColor Red
    exit 1
}

# --- Copy to Tauri resources ---
Write-Host "[2/3] Copying to Tauri resources..." -ForegroundColor Green
if (-not (Test-Path $ResourceDir)) {
    New-Item -ItemType Directory -Force $ResourceDir | Out-Null
}

# The stt_server folder (with all DLLs) needs to be in resources/
$DistDir  = Join-Path $ScriptDir "dist\stt_server"
$DestDir  = Join-Path $ResourceDir "stt_server"

if (Test-Path $DestDir) { Remove-Item $DestDir -Recurse -Force }
Copy-Item $DistDir $DestDir -Recurse -Force

Write-Host "[3/3] Done!" -ForegroundColor Green
Write-Host ""
Write-Host "  Sidecar at: $DestDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Next step: npm run tauri build" -ForegroundColor Yellow
Write-Host "  The stt_server will be bundled in the installer automatically." -ForegroundColor Yellow
Write-Host ""
