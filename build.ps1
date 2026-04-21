<#
.SYNOPSIS
    MMYCodeSwitch-API Build Script
.DESCRIPTION
    One-click build script supporting installer & portable builds.
.PARAMETER Mode
    Build mode: "release" (default) or "dev"
.PARAMETER Portable
    Build portable version (single folder, run without install)
.PARAMETER SingleFile
    Build single-file exe using 7-Zip self-extracting archive
.PARAMETER Clean
    Clean old build artifacts before building
.EXAMPLE
    .\build.ps1                          # NSIS installer
    .\build.ps1 -Portable               # Portable folder (run directly)
    .\build.ps1 -SingleFile             # Single .exe file (requires 7z)
    .\build.ps1 -Clean                  # Clean then rebuild
#>

param(
    [ValidateSet("release", "dev")]
    [string]$Mode = "release",
    [switch]$Portable,
    [switch]$SingleFile,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host ""
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host "  MMYCodeSwitch-API Build Script" -ForegroundColor Cyan
Write-Host "  Mode: $($Mode.ToUpper())" -ForegroundColor Cyan
if ($Portable) { Write-Host "  Type: Portable (no-install)" -ForegroundColor Cyan }
if ($SingleFile) { Write-Host "  Type: Single-File EXE" -ForegroundColor Cyan }
if (-not $Portable -and -not $SingleFile) { Write-Host "  Type: Installer (NSIS)" -ForegroundColor Cyan }
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Check dependencies
Write-Host "[1/6] Checking dependencies..." -ForegroundColor Yellow

$hasNode = Get-Command npm -ErrorAction SilentlyContinue
if (-not $hasNode) {
    Write-Host "[ERROR] Node.js/npm not found." -ForegroundColor Red; exit 1
}

$hasRust = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $hasRust) {
    Write-Host "[ERROR] Rust/Cargo not found." -ForegroundColor Red; exit 1
}
Write-Host "[OK] Node.js: $(node --version)" -ForegroundColor Green
Write-Host "[OK] Cargo:  $(cargo --version)" -ForegroundColor Green

if ($SingleFile) {
    $has7z = Get-Command 7z -ErrorAction SilentlyContinue
    if (-not $has7z) {
        Write-Host "[ERROR] 7-Zip not found. Install it or use -Portable instead." -ForegroundColor Red
        exit 1
    }
    Write-Host "[OK] 7-Zip:   $($has7z.Source)" -ForegroundColor Green
}
Write-Host ""

# Step 2: Clean old artifacts
if ($Clean) {
    Write-Host "[2/6] Cleaning old build artifacts..." -ForegroundColor Yellow
    if (Test-Path "$ProjectRoot\dist") { Remove-Item -Recurse -Force "$ProjectRoot\dist" | Out-Null }
    if ($Mode -eq "release") {
        if (Test-Path "$ProjectRoot\src-tauri\target\release") { Remove-Item -Recurse -Force "$ProjectRoot\src-tauri\target\release" | Out-Null }
        if (Test-Path "$ProjectRoot\dist-portable") { Remove-Item -Recurse -Force "$ProjectRoot\dist-portable" | Out-Null }
        if (Test-Path "$ProjectRoot\dist-single") { Remove-Item -Recurse -Force "$ProjectRoot\dist-single" | Out-Null }
    } else {
        if (Test-Path "$ProjectRoot\src-tauri\target\debug") { Remove-Item -Recurse -Force "$ProjectRoot\src-tauri\target\debug" | Out-Null }
    }
    Write-Host "[OK] Cleanup done" -ForegroundColor Green
} else {
    Write-Host "[2/6] Skipping cleanup (use -Clean to enable)" -ForegroundColor Gray
}
Write-Host ""

# Step 3: Frontend dependencies
Write-Host "[3/6] Checking frontend dependencies..." -ForegroundColor Yellow
Push-Location $ProjectRoot
if (-not (Test-Path "$ProjectRoot\node_modules")) {
    Write-Host "   Installing npm dependencies..."
    npm install
    if ($LASTEXITCODE -ne 0) { Write-Host "[ERROR] npm install failed" -ForegroundColor Red; Pop-Location; exit 1 }
} else {
    Write-Host "   node_modules exists, skipping" -ForegroundColor Gray
}
Pop-Location
Write-Host "[OK] Ready" -ForegroundColor Green
Write-Host ""

# Step 4: Compile only (skip bundle for portable/single)
Write-Host "[4/6] Compiling application... (this takes a while)" -ForegroundColor Yellow
Push-Location $ProjectRoot

if ($Mode -eq "release") {
    npx tauri build --no-bundle
} else {
    npx tauri build --debug --no-bundle
}

$buildResult = $LASTEXITCODE
Pop-Location

if ($buildResult -ne 0) {
    Write-Host ""
    Write-Host "[ERROR] Compilation failed!" -ForegroundColor Red
    exit $buildResult
}
Write-Host ""
Write-Host "[OK] Compilation done" -ForegroundColor Green
Write-Host ""

# Determine binary path
if ($Mode -eq "release") {
    $binaryExe = "$ProjectRoot\src-tauri\target\release\tauri-app.exe"
} else {
    $binaryExe = "$ProjectRoot\src-tauri\target\debug\tauri-app.exe"
}

if (-not (Test-Path $binaryExe)) {
    Write-Host "[ERROR] Binary not found: $binaryExe" -ForegroundColor Red
    exit 1
}

# Step 5: Create output
if ($Portable -or $SingleFile) {
    # --- Portable / Single File ---
    Write-Host "[5/6] Creating portable package..." -ForegroundColor Yellow

    $productName = "MMYCodeSwitch-API"
    if ($Mode -eq "release") {
        $portableDir = "$ProjectRoot\$productName-Portable"
    } else {
        $portableDir = "$ProjectRoot\$productName-Portable-debug"
    }

    # Create fresh portable dir
    if (Test-Path $portableDir) { Remove-Item -Recurse -Force $portableDir | Out-Null }
    New-Item -ItemType Directory -Path $portableDir | Out-Null

    # Copy the exe
    Copy-Item $binaryExe "$portableDir\$productName.exe"

    # Find and copy WebView2 + runtime DLLs from the same dir as the exe
    $sourceDir = Split-Path $binaryExe -Parent
    $dllFiles = Get-ChildItem -Path $sourceDir -Filter "*.dll" -ErrorAction SilentlyContinue
    foreach ($dll in $dllFiles) {
        Copy-Item $dll.FullName "$portableDir\$($dll.Name)"
    }

    # Create a simple readme in the portable folder
    $readmeContent = @"
$productName - Portable Version
==============================

Double-click `$productName.exe to run.

No installation required. Works on Windows 10+.

Data is stored alongside this executable.
"@
    Set-Content -Path "$portableDir\Readme.txt" -Value $readmeContent -Encoding UTF8

    $totalSize = (Get-ChildItem -Recurse $portableDir | Measure-Object -Property Length -Sum).Sum
    Write-Host "[OK] Portable created: $portableDir" -ForegroundColor Green
    Write-Host "     Size: $([math]::Round($totalSize / 1MB, 2)) MB" -ForegroundColor Gray
    Write-Host ""

    # Step 6: If SingleFile, wrap into SFX
    if ($SingleFile) {
        Write-Host "[6/6] Creating single-file EXE (7z SFX)..." -ForegroundColor Yellow

        $outputName = "$productName-$($Mode).exe"
        $outputPath = "$ProjectRoot\$outputName"

        # Remove old
        if (Test-Path $outputPath) { Remove-Item -Force $outputPath }

        # Use 7z to create self-extracting archive
        Push-Location $portableDir
        & 7z a -t7z -m0=LZMA2 -mx=9 "-$outputPath" * | Out-Null
        Pop-Location

        if (Test-Path $outputPath) {
            $sfxSize = [math]::Round((Get-Item $outputPath).Length / 1MB, 2)
            Write-Host ""
            Write-Host "==============================================" -ForegroundColor Green
            Write-Host "  Build Success! (Single File)" -ForegroundColor Green
            Write-Host "==============================================" -ForegroundColor Green
            Write-Host ""
            Write-Host "Single EXE:" -ForegroundColor White
            Write-Host "   $outputPath" -ForegroundColor Cyan
            Write-Host "   Size: $sfxSize MB" -ForegroundColor Gray
            Write-Host ""
            Write-Host "Double-click to extract and run." -ForegroundColor DarkGray

            $openIt = Read-Host "Open folder? (Y/n)"
            if ($openIt -ne "n" -and $openIt -ne "N") {
                explorer.exe $ProjectRoot
            }
        } else {
            Write-Host "[WARN] 7z SFX creation failed. Portable folder still available." -ForegroundColor Yellow
        }
    } else {
        # Just portable, show result
        Write-Host ""
        Write-Host "==============================================" -ForegroundColor Green
        Write-Host "  Build Success! (Portable)" -ForegroundColor Green
        Write-Host "==============================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "Portable folder:" -ForegroundColor White
        Write-Host "   $portableDir" -ForegroundColor Cyan
        Write-Host "   Size: $([math]::Round($totalSize / 1MB, 2)) MB" -ForegroundColor Gray
        Write-Host ""
        Write-Host "Open folder and double-click $productName.exe to run." -ForegroundColor DarkGray
        Write-Host ""

        $openIt = Read-Host "Open folder? (Y/n)"
        if ($openIt -ne "n" -and $openIt -ne "N") {
            explorer.exe $portableDir
        }
    }

} else {
    # --- Standard Installer (NSIS) ---
    Write-Host "[5/6] Bundling NSIS installer..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    if ($Mode -eq "release") {
        npx tauri build
    } else {
        npx tauri build --debug
    }
    $bundleResult = $LASTEXITCODE
    Pop-Location

    # Locate outputs
    Write-Host "[6/6] Locating output files..." -ForegroundColor Yellow
    $nsisExe = $null
    $msiExe = $null

    if ($Mode -eq "release") {
        $nsisDir = "$ProjectRoot\src-tauri\target\release\bundle\nsis"
        $msiDir = "$ProjectRoot\src-tauri\target\release\bundle\msi"
    } else {
        $nsisDir = "$ProjectRoot\src-tauri\target\debug\bundle\nsis"
    }

    if (Test-Path $nsisDir) {
        $nsisExe = Get-ChildItem $nsisDir -Filter "*.exe" -ErrorAction SilentlyContinue |
                   Sort-Object LastWriteTime -Descending | Select-Object -First 1
    }
    if ($msiDir -and (Test-Path $msiDir)) {
        $msiExe = Get-ChildItem $msiDir -Filter "*.msi" -ErrorAction SilentlyContinue |
                   Sort-Object LastWriteTime -Descending | Select-Object -First 1
    }

    Write-Host ""
    Write-Host "==============================================" -ForegroundColor Green
    Write-Host "  Build Success!" -ForegroundColor Green
    Write-Host "==============================================" -ForegroundColor Green
    Write-Host ""

    if ($nsisExe) {
        Write-Host "NSIS Installer (.exe):" -ForegroundColor White
        Write-Host "   $($nsisExe.FullName)" -ForegroundColor Cyan
        Write-Host "   Size: $([math]::Round($nsisExe.Length / 1MB, 2)) MB" -ForegroundColor Gray
        Write-Host ""
    }

    if ($msiExe) {
        Write-Host "MSI Installer (.msi):" -ForegroundColor White
        Write-Host "   $($msiExe.FullName)" -ForegroundColor Cyan
        Write-Host "   Size: $([math]::Round($msiExe.Length / 1MB, 2)) MB" -ForegroundColor Gray
        Write-Host ""
    }

    Write-Host "Double-click .exe to install." -ForegroundColor DarkGray
    Write-Host ""

    $openIt = Read-Host "Open output folder? (Y/n)"
    if ($openIt -ne "n" -and $openIt -ne "N") {
        if ($Mode -eq "release") {
            explorer.exe "$ProjectRoot\src-tauri\target\release\bundle"
        } elseif (Test-Path "$ProjectRoot\src-tauri\target\debug\bundle") {
            explorer.exe "$ProjectRoot\src-tauri\target\debug\bundle"
        }
    }
}
