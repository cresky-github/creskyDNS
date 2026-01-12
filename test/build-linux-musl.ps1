# 下载并使用 musl-cross 工具链编译 Linux 版本
# 自动下载交叉编译工具链并编译

$ErrorActionPreference = "Stop"

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "   CreskyDNS Linux 交叉编译脚本" -ForegroundColor Cyan  
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

# 工具链下载地址
$toolchainUrl = "https://musl.cc/x86_64-linux-musl-cross.tgz"
$toolchainDir = "$env:USERPROFILE\.cargo\x86_64-linux-musl"
$toolchainBin = "$toolchainDir\bin"

# 检查工具链是否已下载
if (!(Test-Path "$toolchainBin\x86_64-linux-musl-gcc.exe")) {
    Write-Host "📦 首次运行，需要下载交叉编译工具链 (~40MB)..." -ForegroundColor Yellow
    Write-Host "下载地址: $toolchainUrl" -ForegroundColor Gray
    
    # 创建目录
    New-Item -ItemType Directory -Force -Path $toolchainDir | Out-Null
    
    # 下载
    $zipFile = "$env:TEMP\musl-cross.tgz"
    Write-Host "正在下载..." -ForegroundColor Yellow
    try {
        Invoke-WebRequest -Uri $toolchainUrl -OutFile $zipFile -UseBasicParsing
        Write-Host "✅ 下载完成" -ForegroundColor Green
    } catch {
        Write-Host "❌ 下载失败: $_" -ForegroundColor Red
        Write-Host "" 
        Write-Host "备选方案：" -ForegroundColor Yellow
        Write-Host "1. 手动下载: $toolchainUrl" -ForegroundColor Gray
        Write-Host "2. 解压到: $toolchainDir" -ForegroundColor Gray
        exit 1
    }
    
    # 解压
    Write-Host "正在解压工具链..." -ForegroundColor Yellow
    try {
        tar -xzf $zipFile -C $env:USERPROFILE\.cargo
        Write-Host "✅ 解压完成" -ForegroundColor Green
    } catch {
        Write-Host "❌ 解压失败。请确保系统中有 tar 命令" -ForegroundColor Red
        exit 1
    }
    
    Remove-Item $zipFile -Force
}

Write-Host ""
Write-Host "🔧 配置编译环境..." -ForegroundColor Yellow

# 设置环境变量
$env:PATH = "$toolchainBin;$env:PATH"
$env:CC_x86_64_unknown_linux_musl = "$toolchainBin\x86_64-linux-musl-gcc.exe"
$env:AR_x86_64_unknown_linux_musl = "$toolchainBin\x86_64-linux-musl-ar.exe"
$env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "$toolchainBin\x86_64-linux-musl-gcc.exe"

# 添加目标
Write-Host "检查 Rust 目标..." -ForegroundColor Yellow
rustup target add x86_64-unknown-linux-musl 2>&1 | Out-Null

Write-Host ""
Write-Host "🚀 开始编译..." -ForegroundColor Green
Write-Host ""

# 编译
cargo build --target x86_64-unknown-linux-musl --release

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "=====================================" -ForegroundColor Green
    Write-Host "   ✅ 编译成功！" -ForegroundColor Green
    Write-Host "=====================================" -ForegroundColor Green
    Write-Host ""
    
    $binaryPath = "target\x86_64-unknown-linux-musl\release\creskyDNS"
    if (Test-Path $binaryPath) {
        $fileInfo = Get-Item $binaryPath
        Write-Host "📦 Linux 二进制文件信息:" -ForegroundColor Cyan
        Write-Host "   路径: $binaryPath" -ForegroundColor Gray
        Write-Host "   大小: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Gray
        Write-Host "   时间: $($fileInfo.LastWriteTime)" -ForegroundColor Gray
        Write-Host ""
        Write-Host "💡 使用方法:" -ForegroundColor Cyan
        Write-Host "   1. 将文件上传到 Linux 服务器" -ForegroundColor Gray
        Write-Host "   2. 添加执行权限: chmod +x creskyDNS" -ForegroundColor Gray
        Write-Host "   3. 运行: ./creskyDNS config.yaml" -ForegroundColor Gray
    }
} else {
    Write-Host ""
    Write-Host "❌ 编译失败！" -ForegroundColor Red
    exit 1
}
