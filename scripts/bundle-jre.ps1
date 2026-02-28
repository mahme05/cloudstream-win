# bundle-jre.ps1
#
# Downloads a portable JRE and strips it down with jlink to the minimum
# modules needed by cloudstream-bridge.jar.  The result is placed at
# src-tauri/resources/jre/ where Tauri will bundle it into the installer.
#
# Prerequisites:
#   - JDK 21+ installed (for jlink)
#   - Internet access (downloads Eclipse Temurin JDK)
#
# Run from repo root:
#   powershell -ExecutionPolicy Bypass -File scripts/bundle-jre.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$SCRIPT_DIR   = Split-Path -Parent $MyInvocation.MyCommand.Definition
$REPO_ROOT    = Split-Path -Parent $SCRIPT_DIR
$OUTPUT_DIR   = Join-Path $REPO_ROOT "src-tauri\resources\jre"
$TEMP_DIR     = Join-Path $env:TEMP "cs-jre-build"

# ── Temurin JDK 21 (Windows x64) — used as source for jlink ─────────────────
$JDK_URL  = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.3%2B9/OpenJDK21U-jdk_x64_windows_hotspot_21.0.3_9.zip"
$JDK_ZIP  = Join-Path $TEMP_DIR "temurin-jdk21.zip"
$JDK_DIR  = Join-Path $TEMP_DIR "jdk21"

# ── Modules required by the bridge JAR ───────────────────────────────────────
# Determined by:  jdeps --print-module-deps cloudstream-bridge.jar
$MODULES = @(
    "java.base",           # core
    "java.net.http",       # HttpClient (OkHttp fallback)
    "java.logging",        # java.util.logging (slf4j-simple)
    "java.xml",            # XML parsing (CloudStream extensions)
    "java.naming",         # JNDI (OkHttp TLS)
    "jdk.crypto.ec",       # Elliptic-curve TLS (most HTTPS sites need this)
    "jdk.crypto.cryptoki"  # PKCS#11 (needed on some Windows versions)
) -join ","

Write-Host ""
Write-Host "==================================================================="
Write-Host "  CloudStream Win - JRE Bundler"
Write-Host "==================================================================="
Write-Host ""

if (-not (Test-Path $TEMP_DIR)) { New-Item -ItemType Directory $TEMP_DIR | Out-Null }

# ── Step 1: download Temurin JDK 21 ──────────────────────────────────────────
if (-not (Test-Path $JDK_ZIP)) {
    Write-Host "[1/4] Downloading Eclipse Temurin JDK 21..."
    Invoke-WebRequest -Uri $JDK_URL -OutFile $JDK_ZIP -UseBasicParsing
} else {
    Write-Host "[1/4] JDK zip already cached."
}

# ── Step 2: extract ───────────────────────────────────────────────────────────
if (-not (Test-Path $JDK_DIR)) {
    Write-Host "[2/4] Extracting JDK..."
    Expand-Archive -Path $JDK_ZIP -DestinationPath $JDK_DIR
} else {
    Write-Host "[2/4] JDK already extracted."
}

$JLINK = Get-ChildItem -Recurse -Filter "jlink.exe" -Path $JDK_DIR |
         Select-Object -First 1 -ExpandProperty FullName

if (-not $JLINK) { Write-Error "jlink.exe not found in $JDK_DIR"; exit 1 }
Write-Host "    Found jlink: $JLINK"

# ── Step 3: build minimal JRE with jlink ─────────────────────────────────────
Write-Host "[3/4] Running jlink (modules: $MODULES)..."

if (Test-Path $OUTPUT_DIR) { Remove-Item -Recurse -Force $OUTPUT_DIR }

$JDK_HOME = Split-Path -Parent (Split-Path -Parent $JLINK)

& $JLINK `
    --module-path "$JDK_HOME\jmods" `
    --add-modules $MODULES `
    --strip-debug `
    --no-man-pages `
    --no-header-files `
    --compress=2 `
    --output $OUTPUT_DIR

if ($LASTEXITCODE -ne 0) { Write-Error "jlink failed ($LASTEXITCODE)"; exit $LASTEXITCODE }

# ── Step 4: done ─────────────────────────────────────────────────────────────
$SIZE_MB = [math]::Round(
    (Get-ChildItem -Recurse $OUTPUT_DIR | Measure-Object -Property Length -Sum).Sum / 1MB, 1
)

Write-Host ""
Write-Host "[4/4] Done!"
Write-Host "    Output : $OUTPUT_DIR"
Write-Host "    Size   : ~${SIZE_MB} MB  (a full JDK is ~320 MB)"
Write-Host ""
Write-Host "The jre/ folder is bundled into your Tauri installer automatically."
Write-Host "No JVM installation required on the end user's machine."
Write-Host ""
