# 简单的 Linux 交叉编译脚本
# 使用已安装的 rustup 工具

Write-Host "🚀 CreskyDNS Linux 交叉编译" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan
Write-Host ""

# 确保使用 rustup 管理的 cargo
$env:PATH = "C:\Users\cresky\.cargo\bin;$env:PATH"

# 添加 Linux 目标
Write-Host "📦 添加 Linux 目标..." -ForegroundColor Yellow
rustup target add x86_64-unknown-linux-gnu

Write-Host ""
Write-Host "⚙️  开始编译（release 模式）..." -ForegroundColor Yellow  
Write-Host "这可能需要几分钟，请耐心等待..." -ForegroundColor Gray
Write-Host ""

# 尝试编译（即使失败也会给出有用信息）
cargo build --target x86_64-unknown-linux-gnu --release

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✅ 编译成功！" -ForegroundColor Green
    $binary = "target\x86_64-unknown-linux-gnu\release\creskyDNS"
    if (Test-Path $binary) {
        $size = [math]::Round((Get-Item $binary).Length / 1MB, 2)
        Write-Host "📦 文件: $binary ($size MB)" -ForegroundColor Cyan
    }
} else {
    Write-Host ""
    Write-Host "⚠️  编译失败" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "常见解决方案：" -ForegroundColor Cyan
    Write-Host "1. 使用 WSL: wsl --install (然后在 WSL 中编译)" -ForegroundColor Gray
    Write-Host "2. 使用 Docker: docker run --rm -v ${PWD}:/app -w /app rust:latest cargo build --release" -ForegroundColor Gray
    Write-Host "3. 使用在线 CI/CD: GitHub Actions, GitLab CI" -ForegroundColor Gray
}
