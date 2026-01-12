# CreskyDNS Linux 编译指南

由于项目依赖 `ring` crate，需要 C 编译器进行交叉编译。在 Windows 上编译 Linux 版本有以下几种方案：

## 方案 1：使用 GitHub Actions（推荐）⭐

创建 `.github/workflows/build.yml`，让 GitHub 自动编译 Linux 版本。

**优点**：
- 完全自动化，无需本地配置
- 免费，编译速度快
- 可以同时编译多个平台

**步骤**：
1. 提交代码到 GitHub
2. GitHub Actions 会自动编译
3. 从 Releases 页面下载编译好的二进制文件

## 方案 2：使用 WSL2（适合本地开发）

在 Windows Subsystem for Linux 中编译。

**安装 WSL**：
```powershell
# 管理员权限运行
wsl --install
```

**在 WSL 中编译**：
```bash
# 进入 WSL
wsl

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译项目
cd /mnt/d/Workspace/creskyDNS
cargo build --release

# 编译完成后，文件在：target/release/creskyDNS
```

## 方案 3：使用 Docker

需要安装 Docker Desktop。

```powershell
# 使用官方 Rust 镜像编译
docker run --rm -v "${PWD}:/workspace" -w /workspace rust:latest cargo build --release

# 编译完成后，文件在：target/release/creskyDNS
```

## 方案 4：使用在线编译服务

1. **repl.it** - 在线 IDE，可以直接编译 Rust 项目
2. **Gitpod** - 云端开发环境
3. **Codespaces** - GitHub 的云端开发环境

## 方案 5：在 Linux 服务器上直接编译

如果你有 Linux 服务器访问权限：

```bash
# 上传源代码到服务器
scp -r . user@server:/path/to/creskyDNS

# SSH 登录
ssh user@server

# 安装 Rust（如果还没有）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 编译
cd /path/to/creskyDNS
cargo build --release

# 编译完成，二进制文件在：target/release/creskyDNS
```

## 推荐方案对比

| 方案 | 难度 | 速度 | 适用场景 |
|------|------|------|----------|
| GitHub Actions | ⭐ 简单 | 快 | 开源项目，需要 CI/CD |
| WSL2 | ⭐⭐ 中等 | 快 | 本地开发，频繁编译 |
| Docker | ⭐⭐ 中等 | 中 | 有 Docker 环境 |
| Linux 服务器 | ⭐⭐⭐ 复杂 | 快 | 有服务器访问权限 |

## 当前状态

✅ 已成功编译 Windows 版本：`target/debug/creskyDNS.exe`  
❌ Linux 版本需要使用上述方案之一

## 快速开始（推荐 WSL2）

```powershell
# 1. 安装 WSL（需要管理员权限）
wsl --install

# 2. 重启计算机

# 3. 打开 WSL，安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 4. 进入项目目录并编译
cd /mnt/d/Workspace/creskyDNS
cargo build --release

# 5. 编译完成！
ls -lh target/release/creskyDNS
```

编译后的 Linux 二进制文件可以直接在任何 Linux 系统上运行（兼容 glibc 2.17+）。
