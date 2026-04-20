@echo off
chcp 65001 >nul 2>&1
title MMYCodeSwitch-API Build
powershell -ExecutionPolicy Bypass -File "%~dp0build.ps1"
pause
