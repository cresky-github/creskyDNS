# 使用 Docker 编译 Linux 版本的 CreskyDNS
# 需要安装 Docker Desktop

Write-Host "开始为 Linux 编译 CreskyDNS..." -ForegroundColor Cyan

# 检查 Docker 是否可用
if (!(Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Host "错误: 未找到 Docker。请先安装 Docker Desktop。" -ForegroundColor Red
    Write-Host "下载地址: https://www.docker.com/products/docker-desktop" -ForegroundColor Yellow
    exit 1
}

# 检查 Docker 是否运行
try {
    docker ps | Out-Null
} catch {
    Write-Host "错误: Docker 未运行。请启动 Docker Desktop。" -ForegroundColor Red
    exit 1
}

# 使用 Rust 官方镜像进行编译
Write-Host "使用 Rust 官方 Docker 镜像进行编译..." -ForegroundColor Green
Write-Host "这可能需要几分钟时间，首次运行会下载镜像..." -ForegroundColor Yellow

docker run --rm `
    -v "${PWD}:/workspace" `
    -w /workspace `
    rust:latest `
    bash -c "cargo build --release && strip target/release/creskyDNS"

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✅ 编译成功！" -ForegroundColor Green
    Write-Host "Linux 二进制文件位于: target/release/creskyDNS" -ForegroundColor Cyan
    
    # 显示文件信息
    if (Test-Path "target/release/creskyDNS") {
        $fileInfo = Get-Item "target/release/creskyDNS"
        Write-Host "`n文件大小: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Green
        Write-Host "修改时间: $($fileInfo.LastWriteTime)" -ForegroundColor Green
    }
} else {
    Write-Host "`n❌ 编译失败！" -ForegroundColor Red
    exit 1
}
