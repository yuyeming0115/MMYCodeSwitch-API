@echo off
chcp 65001 >nul 2>&1
title MMYCodeSwitch-API Build
echo.
echo ==============================================
echo   Select Build Type:
echo.
echo   [1] Portable (recommended) - run directly, no install
echo   [2] Installer (NSIS)       - standard setup program
echo   [3] Single File EXE        - one exe file (needs 7-Zip)
echo   [4] Custom options         - advanced parameters
echo ==============================================
echo.
set /p choice="Enter choice (1-4) or press Enter for default [1]: "

if "%choice%"=="" set choice=1
if "%choice%"=="1" (
    powershell -ExecutionPolicy Bypass -File "%~dp0build.ps1" -Portable
) else if "%choice%"=="2" (
    powershell -ExecutionPolicy Bypass -File "%~dp0build.ps1"
) else if "%choice%"=="3" (
    powershell -ExecutionPolicy Bypass -File "%~dp0build.ps1" -SingleFile
) else if "%choice%"=="4" (
    echo.
    echo Available: .\build.ps1 [-Portable] [-SingleFile] [-Clean]
    echo Example: .\build.ps1 -Portable -Clean
    echo.
    powershell -ExecutionPolicy Bypass -Command "Set-Location '%~dp0'; .\build.ps1"
) else (
    echo Invalid choice. Running portable mode...
    powershell -ExecutionPolicy Bypass -File "%~dp0build.ps1" -Portable
)
pause
