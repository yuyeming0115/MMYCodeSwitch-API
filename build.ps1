<#
.SYNOPSIS
    MMYCodeSwitch-API Build Script
.DESCRIPTION
    One-click build script for Tauri app packaging
.PARAMETER Mode
    Build mode: "release" (default) or "dev"
.PARAMETER Clean
    Clean old build artifacts before building
.EXAMPLE
    .\build.ps1
    .\build.ps1 -Mode dev
    .\build.ps1 -Clean
#>

param(
    [ValidateSet("release", "dev")]
    [string]$Mode = "release",
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host ""
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host "  MMYCodeSwitch-API Build Script" -ForegroundColor Cyan
Write-Host "  Mode: $($Mode.ToUpper())" -ForegroundColor Cyan
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Check dependencies
Write-Host "[1/5] Checking dependencies..." -ForegroundColor Yellow

$hasNode = Get-Command npm -ErrorAction SilentlyContinue
if (-not $hasNode) {
    Write-Host "[ERROR] Node.js/npm not found. Please install Node.js first." -ForegroundColor Red
    exit 1
}

$hasRust = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $hasRust) {
    Write-Host "[ERROR] Rust/Cargo not found. Please install Rust toolchain first." -ForegroundColor Red
    exit 1
}
Write-Host "[OK] Node.js: $(node --version)" -ForegroundColor Green
Write-Host "[OK] Cargo:  $(cargo --version)" -ForegroundColor Green
Write-Host ""

# Step 2: Clean old artifacts
if ($Clean) {
    Write-Host "[2/5] Cleaning old build artifacts..." -ForegroundColor Yellow
    if (Test-Path "$ProjectRoot\dist") {
        Remove-Item -Recurse -Force "$ProjectRoot\dist" | Out-Null
        Write-Host "   Cleaned dist/" -ForegroundColor Gray
    }
    if ($Mode -eq "release") {
        if (Test-Path "$ProjectRoot\src-tauri\target\release") {
            Remove-Item -Recurse -Force "$ProjectRoot\src-tauri\target\release" | Out-Null
            Write-Host "   Cleaned src-tauri/target/release/" -ForegroundColor Gray
        }
    } else {
        if (Test-Path "$ProjectRoot\src-tauri\target\debug") {
            Remove-Item -Recurse -Force "$ProjectRoot\src-tauri\target\debug" | Out-Null
            Write-Host "   Cleaned src-tauri/target/debug/" -ForegroundColor Gray
        }
    }
    Write-Host "[OK] Cleanup done" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "[2/5] Skipping cleanup (use -Clean to enable)" -ForegroundColor Gray
    Write-Host ""
}

# Step 3: Install frontend dependencies
Write-Host "[3/5] Checking frontend dependencies..." -ForegroundColor Yellow
Push-Location $ProjectRoot
if (-not (Test-Path "$ProjectRoot\node_modules")) {
    Write-Host "   First run, installing npm dependencies..."
    npm install
    if ($LASTEXITCODE -ne 0) { Write-Host "[ERROR] npm install failed" -ForegroundColor Red; Pop-Location; exit 1 }
} else {
    Write-Host "   node_modules exists, skipping install" -ForegroundColor Gray
}
Pop-Location
Write-Host "[OK] Frontend dependencies ready" -ForegroundColor Green
Write-Host ""

# Step 4: Run Tauri build
Write-Host "[4/5] Running Tauri build... (this may take a while)" -ForegroundColor Yellow
Push-Location $ProjectRoot

if ($Mode -eq "release") {
    npx tauri build
} else {
    npx tauri build --debug
}

$buildResult = $LASTEXITCODE
Pop-Location

if ($buildResult -ne 0) {
    Write-Host ""
    Write-Host "[ERROR] Build failed! Please check errors above." -ForegroundColor Red
    exit $buildResult
}
Write-Host ""
Write-Host "[OK] Tauri build completed successfully" -ForegroundColor Green
Write-Host ""

# Step 5: Locate output files
Write-Host "[5/5] Locating output files..." -ForegroundColor Yellow

$exePath = $null
$msiPath = $null

if ($Mode -eq "release") {
    $nsisDir = "$ProjectRoot\src-tauri\target\release\bundle\nsis"
    $msiDir = "$ProjectRoot\src-tauri\target\release\bundle\msi"
} else {
    $nsisDir = "$ProjectRoot\src-tauri\target\debug\bundle\nsis"
}

if (Test-Path $nsisDir) {
    $exePath = Get-ChildItem -Path $nsisDir -Filter "*.exe" -ErrorAction SilentlyContinue |
               Sort-Object LastWriteTime -Descending |
               Select-Object -First 1
}

if ($Mode -eq "release" -and (Test-Path $msiDir)) {
    $msiPath = Get-ChildItem -Path $msiDir -Filter "*.msi" -ErrorAction SilentlyContinue |
               Sort-Object LastWriteTime -Descending |
               Select-Object -First 1
}

Write-Host ""
Write-Host "==============================================" -ForegroundColor Green
Write-Host "  Build Success!" -ForegroundColor Green
Write-Host "==============================================" -ForegroundColor Green
Write-Host ""

if ($exePath) {
    Write-Host "NSIS Installer (.exe):" -ForegroundColor White
    Write-Host "   $($exePath.FullName)" -ForegroundColor Cyan
    Write-Host "   Size: $([math]::Round($exePath.Length / 1MB, 2)) MB" -ForegroundColor Gray
    Write-Host ""
}

if ($msiPath) {
    Write-Host "MSI Installer (.msi):" -ForegroundColor White
    Write-Host "   $($msiPath.FullName)" -ForegroundColor Cyan
    Write-Host "   Size: $([math]::Round($msiPath.Length / 1MB, 2)) MB" -ForegroundColor Gray
    Write-Host ""
}

Write-Host "Double-click .exe to run installer." -ForegroundColor DarkGray
Write-Host ""

# Ask to open folder
$openFolder = Read-Host "Open output directory? (Y/n)"
if ($openFolder -ne "n" -and $openFolder -ne "N") {
    if ($Mode -eq "release") {
        explorer.exe "$ProjectRoot\src-tauri\target\release\bundle"
    } elseif (Test-Path "$ProjectRoot\src-tauri\target\debug\bundle") {
        explorer.exe "$ProjectRoot\src-tauri\target\debug\bundle"
    }
}
