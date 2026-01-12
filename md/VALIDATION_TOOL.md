# 域名列表验证工具

## 概述

`validate_domain_lists.py` 是一个 Python 脚本，用于验证域名列表文件是否符合规范格式。

## 功能

- ✅ 检查域名格式有效性
- ✅ 检测禁止的前缀和后缀
- ✅ 检测协议、路径、端口等非法内容
- ✅ 统计有效域名数量
- ✅ 提供详细的错误报告

## 安装

该脚本仅使用 Python 标准库，无需安装额外依赖。

**要求**：Python 3.6+

## 使用方法

### 检查指定文件

```bash
python validate_domain_lists.py direct_domains.txt proxy_domains.txt
```

### 检查所有默认文件

如果没有指定文件，脚本会自动检查以下文件（如果存在）：
- `direct_domains.txt`
- `proxy_domains.txt`
- `adblock_domains.txt`
- `custom_domains.txt`

```bash
python validate_domain_lists.py
```

## 输出示例

### 成功验证

```
检查文件: direct_domains.txt
============================================================

统计信息:
  总行数: 31
  有效域名: 30 ✓
  注释行: 1
  空行: 0
  无效行: 0

✅ 所有域名格式都是有效的!

============================================================
✅ 所有文件验证通过!
```

### 发现错误

```
检查文件: bad_domains.txt
============================================================

统计信息:
  总行数: 5
  有效域名: 2 ✓
  注释行: 1
  空行: 0
  无效行: 2

❌ 发现 2 个错误:
  - 行 3: 不应该有前缀 '*.' - *.google.com
  - 行 5: 无效的域名格式 - google@example.com

============================================================
❌ 部分文件验证失败，请检查上面的错误信息
```

## 检查项

### 禁止的前缀
- `*.` - 通配符前缀
- `||` - Adblock 格式
- `^` - 正则表达式锚
- `@@||` - Adblock 例外格式

### 禁止的后缀
- `$` - 结尾标记
- `|` - 管道符
- `/*` - 路径符

### 禁止的内容
- 协议（`http://`, `https://` 等）
- 路径（`/path/to/resource`）
- 端口（`:80`, `:443` 等）
- 赋值（`domain=127.0.0.1`）
- 无效的域名字符

## 返回码

- `0` - 所有文件验证通过
- `1` - 至少有一个文件验证失败或文件不存在

## 示例场景

### 修复问题文件

1. **运行验证**：
   ```bash
   python validate_domain_lists.py my_domains.txt
   ```

2. **查看错误报告**，例如：
   ```
   ❌ 发现 3 个错误:
     - 行 5: 不应该有前缀 '*.' - *.example.com
     - 行 12: 不应该包含路径 - google.com/search
     - 行 18: 不应该包含端口 - api.example.com:8080
   ```

3. **修复错误**：
   - 删除 `*.` 前缀：`example.com`
   - 删除路径：`google.com`
   - 删除端口：`api.example.com`

4. **重新验证**：
   ```bash
   python validate_domain_lists.py my_domains.txt
   ```

## 与 CI/CD 集成

可以在 CI/CD 管道中使用验证脚本：

```yaml
# 示例 GitHub Actions
- name: 验证域名列表
  run: python validate_domain_lists.py
```

```bash
# 示例 GitLab CI
validate_domains:
  script:
    - python validate_domain_lists.py
```

## 常见问题

### Q: 如何忽略某些行？
A: 在行首添加 `#` 注释符，这行将被忽略。

### Q: 可以在命令行中加入多个文件吗？
A: 可以，用空格分隔：
```bash
python validate_domain_lists.py file1.txt file2.txt file3.txt
```

### Q: 脚本支持哪些编码？
A: 目前仅支持 UTF-8 编码。如果文件使用其他编码，请先转换。

### Q: 如何处理国际化域名（IDN）？
A: 国际化域名应该以 ASCII 兼容编码（Punycode）格式存储，例如 `xn--e1afmkfd.com`（пример.com）。

## 相关文档

- [域名列表格式说明](DOMAIN_LIST_FORMAT.md)
- [快速参考](DOMAIN_LIST_QUICK_REF.md)
- [更新说明](DOMAIN_LIST_UPDATE.md)
