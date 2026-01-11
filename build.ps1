# 确保 MinGW-w64 在 PATH 中（用于 GNU 工具链编译）
if ($env:PATH -notlike "*C:\msys64\mingw64\bin*") {
    $env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
    Write-Host "已添加 MinGW-w64 到 PATH" -ForegroundColor Green
}

# 使用 GNU 工具链编译项目
cargo build $args
