<#
.SYNOPSIS
    One-click setup for the Salenie Modal inference endpoint.
    Run this ONCE after installing Genie Recorder.

.DESCRIPTION
    This script:
      1. Verifies Python is installed (or installs it via winget)
      2. Creates a dedicated Python venv for the Modal CLI
      3. Installs the Modal package
      4. Opens your browser to authenticate with Modal
      5. Prompts you for your HuggingFace token
      6. Creates the required Modal secret (API_TOKEN + HF_TOKEN)
      7. Deploys the Salenie Phi-4 inference endpoint to Modal's cloud
      8. Writes the endpoint URL + API token directly into Genie Recorder's config

    After this script completes, open Genie Recorder → Config → AI Analysis
    and just toggle it ON. Everything else is pre-filled.

.EXAMPLE
    Right-click setup-modal.ps1 → "Run with PowerShell"
#>

$ErrorActionPreference = "Stop"
$VenvDir   = "$env:LOCALAPPDATA\GenieRecorder\modal-venv"
$ConfigDir = "$env:APPDATA\com.demo.genierecorder"
$ConfigFile = "$ConfigDir\config.json"
$ScriptDir  = Split-Path -Parent $MyInvocation.MyCommand.Definition

# ── Helpers ─────────────────────────────────────────────────────────────────

function Write-Step($n, $msg) {
    Write-Host ""
    Write-Host "[$n] $msg" -ForegroundColor Cyan
}

function Write-OK($msg)    { Write-Host "    ✓ $msg" -ForegroundColor Green }
function Write-Warn($msg)  { Write-Host "    ! $msg" -ForegroundColor Yellow }
function Write-Err($msg)   { Write-Host "    ✗ $msg" -ForegroundColor Red }

function Prompt-Secret($prompt) {
    $secure = Read-Host $prompt -AsSecureString
    [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    )
}

# ── Banner ───────────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "╔══════════════════════════════════════════════╗" -ForegroundColor Magenta
Write-Host "║     Salenie Modal Setup — Genie Recorder     ║" -ForegroundColor Magenta
Write-Host "╚══════════════════════════════════════════════╝" -ForegroundColor Magenta
Write-Host ""
Write-Host "  This sets up the Phi-4 AI analysis endpoint on Modal's cloud."
Write-Host "  You need a free Modal account: https://modal.com"
Write-Host "  You need a HuggingFace account with access to:"
Write-Host "  https://huggingface.co/Mike-Benis/Salenie-Phi4-v1"
Write-Host ""
Read-Host "  Press Enter to begin (Ctrl+C to cancel)"

# ── Step 1: Python ───────────────────────────────────────────────────────────

Write-Step "1/7" "Checking Python installation..."

$python = $null
foreach ($cmd in @("python3.11", "python3.10", "python3", "python")) {
    try {
        $ver = & $cmd --version 2>&1
        if ($ver -match "3\.(10|11)") {
            $python = $cmd
            Write-OK "Found: $ver ($cmd)"
            break
        }
    } catch {}
}

if (-not $python) {
    Write-Warn "Python 3.10/3.11 not found — attempting to install via winget..."
    try {
        winget install Python.Python.3.11 --silent --accept-source-agreements --accept-package-agreements
        $python = "python"
        Write-OK "Python 3.11 installed."
    } catch {
        Write-Err "Could not auto-install Python."
        Write-Host ""
        Write-Host "  Please install Python 3.11 manually from https://python.org"
        Write-Host "  Make sure to check 'Add to PATH' during installation."
        exit 1
    }
}

# ── Step 2: Create venv ──────────────────────────────────────────────────────

Write-Step "2/7" "Creating Modal venv at $VenvDir..."

if (Test-Path "$VenvDir\Scripts\python.exe") {
    Write-OK "Venv already exists — skipping."
} else {
    & $python -m venv $VenvDir
    Write-OK "Venv created."
}

$pip    = "$VenvDir\Scripts\pip.exe"
$modal  = "$VenvDir\Scripts\modal.exe"
$venvPy = "$VenvDir\Scripts\python.exe"

# ── Step 3: Install Modal ────────────────────────────────────────────────────

Write-Step "3/7" "Installing Modal CLI..."
& $pip install --quiet --upgrade modal
Write-OK "Modal installed."

# ── Step 4: Authenticate ─────────────────────────────────────────────────────

Write-Step "4/7" "Authenticating with Modal..."
Write-Host ""
Write-Host "  A browser window will open. Log in or sign up at modal.com."
Write-Host "  After authenticating, return to this window."
Write-Host ""
Read-Host "  Press Enter to open the browser"

& $modal setup

Write-OK "Modal authentication complete."

# ── Step 5: Collect secrets ──────────────────────────────────────────────────

Write-Step "5/7" "Collecting your secrets..."
Write-Host ""
Write-Host "  HuggingFace token — get from https://huggingface.co/settings/tokens"
Write-Host "  (needs READ access to Mike-Benis/Salenie-Phi4-v1)"
Write-Host ""
$hfToken  = Prompt-Secret "  HuggingFace token (hf_...)"
Write-Host ""
Write-Host "  API Token — choose any strong random string."
Write-Host "  This protects your endpoint. Keep it secret."
Write-Host ""
$apiToken = Prompt-Secret "  Choose an API token"

# ── Step 6: Create Modal secret ──────────────────────────────────────────────

Write-Step "6/7" "Creating Modal secret 'salenie-api-secret'..."

# Delete existing secret if present (ignore error)
& $modal secret delete salenie-api-secret 2>$null

& $modal secret create salenie-api-secret `
    "HF_TOKEN=$hfToken" `
    "API_TOKEN=$apiToken"

Write-OK "Secret created."

# ── Step 7: Deploy ───────────────────────────────────────────────────────────

Write-Step "7/7" "Deploying Salenie Phi-4 to Modal..."
Write-Host ""
Write-Host "  This downloads the model weights on first run (~14 GB)."
Write-Host "  Subsequent deploys are instant (weights cached in Modal Volume)."
Write-Host ""

$deployOutput = & $modal deploy "$ScriptDir\modal_serve.py" 2>&1
Write-Host $deployOutput

# Extract the generate endpoint URL from deploy output
$generateUrl = ($deployOutput | Select-String "salenie-generate\.modal\.run").Matches.Value
if (-not $generateUrl) {
    # Try to get it from modal app list
    $listOutput  = & $modal app list 2>&1
    $generateUrl = ($listOutput | Select-String "salenie-generate\.modal\.run").Matches.Value
}

if ($generateUrl) {
    $generateUrl = $generateUrl.Trim()
    Write-OK "Endpoint: $generateUrl"
} else {
    Write-Warn "Could not auto-detect endpoint URL."
    Write-Host "  Run: modal app list" -ForegroundColor Yellow
    Write-Host "  Look for the URL ending in '--salenie-generate.modal.run'" -ForegroundColor Yellow
    $generateUrl = Read-Host "  Paste the generate endpoint URL"
}

# ── Write config ─────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "  Writing config to Genie Recorder..." -ForegroundColor Cyan

$endpointValue = "$generateUrl $apiToken"

# Read existing config or start fresh
$cfg = @{}
if (Test-Path $ConfigFile) {
    try {
        $cfg = Get-Content $ConfigFile -Raw | ConvertFrom-Json
        # ConvertFrom-Json returns PSCustomObject, convert to hashtable
        $cfg = @{} + ($cfg | Get-Member -MemberType NoteProperty |
                       ForEach-Object { @{ $_.Name = $cfg.($_.Name) } } |
                       ForEach-Object { $_ })
    } catch {}
}

# Ensure analysis section exists
if (-not $cfg.ContainsKey("analysis")) {
    $cfg["analysis"] = @{}
}
$cfg["analysis"]["enabled"]           = $true
$cfg["analysis"]["inferenceEndpoint"] = $endpointValue
if (-not $cfg["analysis"]["whisperModel"]) { $cfg["analysis"]["whisperModel"] = "base" }
if (-not $cfg["analysis"]["sttPort"])      { $cfg["analysis"]["sttPort"]      = 8765 }

New-Item -ItemType Directory -Force $ConfigDir | Out-Null
$cfg | ConvertTo-Json -Depth 10 | Set-Content $ConfigFile -Encoding UTF8

Write-OK "Config saved to $ConfigFile"

# ── Done ─────────────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "╔══════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║              Setup Complete! ✓               ║" -ForegroundColor Green
Write-Host "╚══════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "  Endpoint:  $generateUrl" -ForegroundColor White
Write-Host "  API Token: (saved securely in config)" -ForegroundColor White
Write-Host ""
Write-Host "  Open Genie Recorder → hover the genie → click ⬡"
Write-Host "  Go to AI Analysis tab → toggle ON → click Test Services"
Write-Host ""
Write-Host "  On first call, Modal spins up a GPU container (~15-20 s)."
Write-Host "  Subsequent calls take ~5 s."
Write-Host ""
