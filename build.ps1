<#
.SYNOPSIS
    MMYCodeSwitch-API 一键打包脚本
.DESCRIPTION
    自动编译前端 + Tauri 后端，生成单文件 Windows exe 安装包
.PARAMETER Mode
    构建模式: "release" (默认) 或 "dev"
.PARAMETER Clean
    是否先清理旧的构建产物
.EXAMPLE
    .\build.ps1                    # Release 模式构建
    .\build.ps1 -Mode dev          # 开发模式构建（更快，但文件更大）
    .\build.ps1 -Clean             # 清理后重新构建
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

# 检查环境依赖
Write-Host "[1/5] 检查环境依赖..." -ForegroundColor Yellow

$hasNode = Get-Command npm -ErrorAction SilentlyContinue
if (-not $hasNode) {
    Write-Host "❌ 未找到 Node.js/npm，请先安装 Node.js" -ForegroundColor Red
    exit 1
}

$hasRust = (Get-Command cargo -ErrorAction SilentlyContinue) -or ($env:RUSTUP_HOME)
if (-not $hasRust) {
    Write-Host "❌ 未找到 Rust/Cargo，请先安装 Rust 工具链" -ForegroundColor Red
    exit 1
}

$hasTauriCli = Get-Command tauri -ErrorAction SilentlyContinue
if (-not $hasTauriCli) {
    Write-Host "⚠️  未检测到全局 Tauri CLI，将使用 npx 执行" -ForegroundColor Yellow
}

Write-Host "✅ Node.js: $(node --version)" -ForegroundColor Green
Write-Host "✅ Cargo:  $(cargo --version)" -ForegroundColor Green
Write-Host ""

# 清理旧产物
if ($Clean) {
    Write-Host "[2/5] 清理旧构建产物..." -ForegroundColor Yellow
    if (Test-Path "$ProjectRoot\dist") {
        Remove-Item -Recurse -Force "$ProjectRoot\dist" | Out-Null
        Write-Host "   已清理 dist/" -ForegroundColor Gray
    }
    if (Test-Path "$ProjectRoot\src-tauri\target") {
        # 只清理 release 或 debug 目录以加快速度
        if ($Mode -eq "release") {
            if (Test-Path "$ProjectRoot\src-tauri\target\release") {
                Remove-Item -Recurse -Force "$ProjectRoot\src-tauri\target\release" | Out-Null
                Write-Host "   已清理 src-tauri/target/release/" -ForegroundColor Gray
            }
        } else {
            if (Test-Path "$ProjectRoot\src-tauri\target\debug") {
                Remove-Item -Recurse -Force "$ProjectRoot\src-tauri\target\debug" | Out-Null
                Write-Host "   已清理 src-tauri/target/debug/" -ForegroundColor Gray
            }
        }
    }
    Write-Host "✅ 清理完成" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "[2/5] 跳过清理（使用 -Clean 参数可清理旧产物）" -ForegroundColor Gray
    Write-Head ""
}

# 安装前端依赖
Write-Host "[3/5] 检查并安装前端依赖..." -ForegroundColor Yellow
Push-Location $ProjectRoot
if (-not (Test-Path "$ProjectRoot\node_modules")) {
    Write-Host "   首次运行，安装 npm 依赖..."
    npm install
    if ($LASTEXITCODE -ne 0) { Write-Host "❌ npm install 失败" -ForegroundColor Red; Pop-Location; exit 1 }
} else {
    Write-Host "   node_modules 已存在，跳过安装" -ForegroundColor Gray
}
Pop-Location
Write-Host "✅ 前端依赖就绪" -ForegroundColor Green
Write-Host ""

# 执行 Tauri 构建
Write-Host "[4/5] 开始 Tauri 构建... (这需要一些时间)" -ForegroundColor Yellow
Push-Location $ProjectRoot

if ($Mode -eq "release") {
    if ($hasTauriCli) {
        tauri build
    } else {
        npx tauri build
    }
} else {
    if ($hasTauriCli) {
        tauri build --debug
    } else {
        npx tauri build --debug
    }
}

$buildExitCode = $LASTEXITCODE
Pop-Location

if ($buildExitCode -ne 0) {
    Write-Host "" 
    Write-Host "❌ 构建失败！请检查上方错误信息" -ForegroundColor Red
    exit $buildExitCode
}
Write-Host ""
Write-Host "✅ Tauri 构建完成" -ForegroundColor Green
Write-Host ""

# 定位输出文件
Write-Host "[5/5] 定位输出文件..." -ForegroundColor Yellow

if ($Mode -eq "release") {
    $exePath = Get-ChildItem -Path "$ProjectRoot\src-tauri\target\release\bundle\nsis\" -Filter "*.exe" -ErrorAction SilentlyContinue |
               Sort-Object LastWriteTime -Descending |
               Select-Object -First 1
    
    $msiPath = Get-ChildItem -Path "$ProjectRoot\src-tauri\target\release\bundle\msi\" -Filter "*.msi" -ErrorAction SilentlyContinue |
               Sort-Object LastWriteTime -Descending |
               Select-Object -First 1
} else {
    $exePath = Get-ChildItem -Path "$ProjectRoot\src-tauri\target\debug\bundle\nsis\" -Filter "*.exe" -ErrorAction SilentlyContinue |
               Sort-Object LastWriteTime -Descending |
               Select-Object -First 1
}

Write-Host ""
Write-Host "==============================================" -ForegroundColor Green
Write-Host "  🎉 打包成功！" -ForegroundColor Green
Write-Host "==============================================" -ForegroundColor Green
Write-Host ""

if ($exePath) {
    Write-Host "📦 NSIS 安装包 (.exe):" -ForegroundColor White
    Write-Host "   $($exePath.FullName)" -ForegroundColor Cyan
    Write-Host "   大小: $([math]::Round($exePath.Length / 1MB, 2)) MB" -ForegroundColor Gray
    Write-Host ""
}

if ($msiPath) {
    Write-Host "📦 MSI 安装包 (.msi):" -ForegroundColor White
    Write-Host "   $($msiPath.FullName)" -ForegroundColor Cyan
    Write-Host "   大小: $([math]::Round($msiPath.Length / 1MB, 2)) MB" -ForegroundColor Gray
    Write-Host ""
}

Write-Host "提示: 双击 .exe 文件即可运行安装程序" -ForegroundColor DarkGray
Write-Host ""

# 可选：自动打开文件夹
$openFolder = Read-Host "是否打开输出目录? (Y/n)"
if ($openFolder -ne "n" -and $openFolder -ne "N") {
    if ($Mode -eq "release") {
        explorer.exe "$ProjectRoot\src-tauri\target\release\bundle"
    } else {
        explorer.exe "$ProjectRoot\src-tauri\target\debug\bundle"
    }
}
