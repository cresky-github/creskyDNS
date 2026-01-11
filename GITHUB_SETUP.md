# 🚀 使用 GitHub Actions 自动编译 CreskyDNS

## 步骤 1: 安装 Git

### 下载并安装 Git for Windows
访问：https://git-scm.com/download/win

或使用命令安装：
```powershell
winget install --id Git.Git -e --source winget
```

安装完成后，**重启 PowerShell 或 VS Code**。

---

## 步骤 2: 初始化 Git 仓库

```powershell
# 进入项目目录
cd D:\Workspace\creskyDNS

# 初始化 Git 仓库
git init

# 配置用户信息（首次使用 Git 需要）
git config --global user.name "你的用户名"
git config --global user.email "你的邮箱@example.com"

# 添加所有文件
git add .

# 提交
git commit -m "Initial commit: CreskyDNS DNS Forwarder"
```

---

## 步骤 3: 在 GitHub 上创建仓库

1. 访问 https://github.com/new
2. 填写仓库信息：
   - **Repository name**: `creskyDNS`
   - **Description**: `智能 DNS 转发器 - Smart DNS Forwarder`
   - 选择 **Public** 或 **Private**
   - **不要**勾选 "Initialize this repository with a README"
3. 点击 **Create repository**

---

## 步骤 4: 推送代码到 GitHub

创建仓库后，GitHub 会显示推送命令。复制并执行：

```powershell
# 添加远程仓库（替换为你的用户名）
git remote add origin https://github.com/你的用户名/creskyDNS.git

# 推送代码
git branch -M main
git push -u origin main
```

如果提示输入用户名和密码：
- 用户名：你的 GitHub 用户名
- 密码：需要使用 **Personal Access Token** (不是 GitHub 密码)

### 创建 Personal Access Token:
1. 访问：https://github.com/settings/tokens
2. 点击 **Generate new token (classic)**
3. 勾选 `repo` 权限
4. 点击生成，复制 token（只显示一次！）
5. 在密码处粘贴 token

---

## 步骤 5: 查看自动编译

推送成功后：

1. 访问你的 GitHub 仓库
2. 点击 **Actions** 标签
3. 你会看到编译任务正在运行 🚀

编译大约需要 **5-10 分钟**，完成后：
- 点击任务名称
- 在 **Artifacts** 部分下载编译好的文件

### 编译产物包括：
- ✅ creskyDNS-linux-x86_64
- ✅ creskyDNS-linux-x86_64-musl (静态链接)
- ✅ creskyDNS-linux-aarch64
- ✅ creskyDNS-windows-x86_64.exe
- ✅ creskyDNS-macos-x86_64
- ✅ creskyDNS-macos-arm64

---

## 步骤 6: 创建 Release (可选)

如果想创建正式版本：

```powershell
# 创建标签
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会自动创建 Release 并上传所有编译好的文件！

---

## 🎉 完成！

现在每次推送代码，GitHub 都会自动编译所有平台的版本。

### 快速命令参考：

```powershell
# 日常开发流程
git add .
git commit -m "你的提交信息"
git push

# 创建新版本
git tag v0.1.1
git push origin v0.1.1
```

---

## 📝 注意事项

1. `.gitignore` 文件会自动忽略 `target/` 目录（编译产物）
2. 第一次编译可能较慢（需要下载依赖）
3. 后续编译会使用缓存，速度更快
4. 编译产物保存 90 天

---

## 🔧 故障排查

### Git 推送失败？
- 检查网络连接
- 确认使用 Personal Access Token 而不是密码
- 尝试使用 SSH 方式：https://docs.github.com/cn/authentication/connecting-to-github-with-ssh

### GitHub Actions 编译失败？
- 查看 Actions 日志，会显示详细错误信息
- 常见问题：依赖版本冲突、网络超时

### 需要帮助？
- GitHub Actions 文档：https://docs.github.com/cn/actions
- Rust 交叉编译：https://rust-lang.github.io/rustup/cross-compilation.html
