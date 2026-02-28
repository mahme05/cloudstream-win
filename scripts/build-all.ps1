# build-all.ps1
# Run from repo root:
#   powershell -ExecutionPolicy Bypass -File scripts/build-all.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$REPO_ROOT  = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Definition)
$BRIDGE_DIR = Join-Path $REPO_ROOT "cloudstream-bridge"
$JRE_DIR    = Join-Path $REPO_ROOT "src-tauri\resources\jre"
$RESOURCES_DIR = Join-Path $REPO_ROOT "src-tauri\resources"

Write-Host ""
Write-Host "==================================================================="
Write-Host "  CloudStream Win - Full Build"
Write-Host "==================================================================="
Write-Host ""

# Make sure resources dir exists
if (-not (Test-Path $RESOURCES_DIR)) {
    New-Item -ItemType Directory -Path $RESOURCES_DIR | Out-Null
}

# Step 1: Check Java is available
Write-Host "[0/3] Checking prerequisites..."
try {
    $javaVersion = & java -version 2>&1
    Write-Host "      Java found: $($javaVersion[0])"
} catch {
    Write-Host ""
    Write-Host "ERROR: Java not found on PATH."
    Write-Host "Install JDK 11+ from https://adoptium.net and add it to your PATH."
    exit 1
}

# Check npm
try {
    $npmVersion = & npm --version 2>&1
    Write-Host "      npm found: $npmVersion"
} catch {
    Write-Host "ERROR: npm not found. Install Node.js from https://nodejs.org"
    exit 1
}

# Check cargo-tauri is installed
$tauriInstalled = & cargo tauri --version 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "[!] cargo-tauri not found. Installing now..."
    & cargo install tauri-cli --version "^2.0"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Failed to install tauri-cli"
        exit 1
    }
}
Write-Host "      cargo-tauri found"

Write-Host ""

# Step 2: Build the Kotlin bridge JAR
Write-Host "[1/3] Building cloudstream-bridge.jar..."
Push-Location $BRIDGE_DIR
try {
    & ".\gradlew.bat" deployToTauri --no-daemon
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Gradle build failed"
        exit 1
    }
} finally {
    Pop-Location
}
Write-Host "      OK -> src-tauri/resources/cloudstream-bridge.jar"
Write-Host ""

# Step 3: Bundle JRE (skip if already exists)
Write-Host "[2/3] Checking bundled JRE..."
if (Test-Path $JRE_DIR) {
    Write-Host "      JRE already exists at src-tauri/resources/jre/ -- skipping."
    Write-Host "      Delete that folder and re-run to rebuild."
} else {
    Write-Host "      Bundling minimal JRE with jlink..."
    $bundleScript = Join-Path $REPO_ROOT "scripts\bundle-jre.ps1"
    & powershell -ExecutionPolicy Bypass -File $bundleScript
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: JRE bundling failed"
        exit 1
    }
}
Write-Host ""

# Step 4: Install npm dependencies
Write-Host "[3/3] Installing npm dependencies..."
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
Write-Host "  Build complete! Run one of:"
Write-Host ""
Write-Host "    cargo tauri dev      <- hot-reload dev mode"
Write-Host "    cargo tauri build    <- production installer"
Write-Host "==================================================================="
Write-Host ""
