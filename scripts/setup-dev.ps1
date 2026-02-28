# setup-dev.ps1
# Run this ONCE to install the tools needed for development.
# After this, use scripts/build-all.ps1 to build and then cargo tauri dev.
#
# Run from repo root:
#   powershell -ExecutionPolicy Bypass -File scripts/setup-dev.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "==================================================================="
Write-Host "  CloudStream Win - Dev Environment Setup"
Write-Host "==================================================================="
Write-Host ""

# Check Rust
Write-Host "[1/3] Checking Rust..."
try {
    $rustVersion = & rustc --version 2>&1
    Write-Host "      $rustVersion"
} catch {
    Write-Host "ERROR: Rust not found."
    Write-Host "Install from https://rustup.rs"
    exit 1
}

# Install cargo-tauri v2
Write-Host ""
Write-Host "[2/3] Installing tauri-cli v2..."
Write-Host "      (This may take a few minutes the first time)"
& cargo install tauri-cli --version "^2.0" --locked
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to install tauri-cli"
    exit 1
}
Write-Host "      OK -- 'cargo tauri' is now available"

# Install npm deps
Write-Host ""
Write-Host "[3/3] Installing npm dependencies..."
$REPO_ROOT = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Definition)
Push-Location $REPO_ROOT
try {
    & npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: npm install failed"
        exit 1
    }
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "==================================================================="
Write-Host "  Setup complete! Next steps:"
Write-Host ""
Write-Host "  1. Build the bridge + JRE:"
Write-Host "       powershell -ExecutionPolicy Bypass -File scripts/build-all.ps1"
Write-Host ""
Write-Host "  2. Start dev mode:"
Write-Host "       cargo tauri dev"
Write-Host "==================================================================="
Write-Host ""
