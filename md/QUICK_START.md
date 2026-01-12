# 🚀 域名列表快速启动指南

## 5 分钟快速开始

### 1️⃣ 理解格式（1 分钟）

```text
✅ 正确
google.com
www.example.com
api.service.io

❌ 错误
*.google.com
||www.example.com
google.com:80
google.com$
```

**记住**：纯域名，无任何前后缀或特殊字符！

---

### 2️⃣ 创建文件（1 分钟）

创建 `my_domains.txt`：
```text
# 我的自定义域名列表
google.com
facebook.com
example.com
```

---

### 3️⃣ 配置应用（1 分钟）

编辑 `config.yaml`：
```yaml
lists:
  custom:
    type: "domain"
    format: "text"
    path: "my_domains.txt"      # 指定文件路径
    domains: []                  # 可留空或作为备用
```

---

### 4️⃣ 验证文件（1 分钟）（可选）

```bash
python validate_domain_lists.py my_domains.txt
```

输出：
```
✅ 所有域名格式都是有效的!
```

---

### 5️⃣ 启动应用（1 分钟）

```bash
cargo run --release
```

日志输出：
```
域名列表 'custom' 从文件 'my_domains.txt' 加载成功: 3 个域名
```

---

## 📋 常见任务

### 任务 1：添加域名

```text
# 编辑文件，添加一行
api.example.com
```

### 任务 2：注释掉某个域名

```text
# twitter.com  # 暂时禁用
facebook.com
```

### 任务 3：检查文件是否有效

```bash
python validate_domain_lists.py direct_domains.txt
```

### 任务 4：更新已加载的列表

1. 修改文件
2. 重启应用
3. 检查日志确认加载

---

## 💡 核心概念

### 域名匹配如何工作？

假设列表中有 `google.com`：

| 查询 | 结果 |
|------|------|
| `google.com` | ✅ 匹配 |
| `www.google.com` | ✅ 匹配 |
| `mail.google.com` | ✅ 匹配 |
| `notgoogle.com` | ❌ 不匹配 |

### 为什么不支持前缀？

```text
❌ 错误方式：
*.google.com    <- 通配符
||google.com    <- adblock 格式
^google.com     <- 正则表达式

✅ 正确方式：
google.com      <- 直接域名，会自动匹配所有子域名
```

应用会自动处理子域名匹配，无需添加通配符。

---

## 🔍 故障排除

### 问题：文件不被加载

**检查清单**：
1. 文件是否存在？
2. 文件路径是否正确？
3. 文件是否有读取权限？
4. 查看应用日志

### 问题：域名不匹配

**检查清单**：
1. 文件是否被成功加载？（查看日志）
2. 域名格式是否正确？（运行验证脚本）
3. 是否需要重启应用？

### 问题：验证脚本报错

**常见错误**：
```
行 5: 不应该有前缀 '*.' - *.example.com
```

**解决方法**：删除 `*.` 前缀，改为 `example.com`

---

## 📚 深入学习

想了解更多？查看这些文档：

| 文档 | 内容 |
|------|------|
| [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) | 详细规范 |
| [DOMAIN_LIST_QUICK_REF.md](DOMAIN_LIST_QUICK_REF.md) | 速查表 |
| [VALIDATION_TOOL.md](VALIDATION_TOOL.md) | 验证工具详解 |

---

## ⚡ 最常用的命令

```bash
# 验证所有文件
python validate_domain_lists.py

# 验证指定文件
python validate_domain_lists.py direct_domains.txt

# 构建项目
cargo build --release

# 运行应用
cargo run

# 查看日志（如果运行在后台）
tail -f app.log
```

---

## 🎯 最佳实践 3 点

### 1. 保持格式干净
```text
✅ 好
google.com
facebook.com

❌ 坏
*.google.com
||facebook.com
```

### 2. 添加有用的注释
```text
# 国外社交媒体
facebook.com
twitter.com

# 开发工具
github.com
```

### 3. 定期验证
```bash
# 每次修改后都验证
python validate_domain_lists.py
```

---

## 🆘 获取帮助

1. **检查日志** - 查看应用启动时的输出
2. **运行验证** - 使用 `validate_domain_lists.py` 检查文件
3. **查看文档** - 找不到答案时查阅相关文档
4. **查阅示例** - 参考 `direct_domains.txt` 等示例文件

---

**准备好开始了吗？** 👉 按照上面的 5 步快速开始！
