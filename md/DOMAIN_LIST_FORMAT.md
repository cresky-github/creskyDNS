# 域名列表文件格式说明

## 概述

域名列表文件用于配置 DNS 转发器中的域名匹配规则。这些文件采用纯文本格式，每行一个域名。

## 文件格式

### 基本格式

```
每行一个域名，无前后缀
```

**无前后缀** 表示：
- 不需要添加任何前缀（如 `*.` 或 `^`）
- 不需要添加任何后缀（如 `$` 或 `/*`）
- 直接写入域名本身

### 示例

```text
# 注释行（以 # 开头的行会被忽略）

com
google.com
www.google.com
api.example.com
```

## 详细说明

### 允许的内容

1. **域名**：任何有效的域名
   ```
   example.com
   subdomain.example.com
   www.google.com
   ```

2. **注释**：以 `#` 开头的行
   ```
   # 这是一个注释
   # 列表类型：直连域名
   ```

3. **空行**：可以有空行，会被自动跳过
   ```
   example.com
   
   another.com
   ```

### 不允许的内容

❌ **不要添加前缀**：
```
*.google.com      ❌ 不需要通配符前缀
^google.com       ❌ 不需要正则表达式前缀
||google.com      ❌ 不需要 adblock 格式的前缀
```

❌ **不要添加后缀**：
```
google.com$       ❌ 不需要结尾标记
google.com/       ❌ 不需要路径符
```

❌ **不要混用格式**：
```
127.0.0.1 google.com   ❌ 不需要 IP 地址
google.com=127.0.0.1   ❌ 不需要赋值格式
```

## 匹配行为

当配置中指定了域名列表文件时，DNS 转发器会：

1. **加载文件**：读取文件中的所有有效域名
2. **清除配置中的域名**：使用文件中的域名列表替换配置中硬编码的域名
3. **匹配查询**：使用前缀匹配（后缀匹配）来检查查询域名

### 匹配示例

假设域名列表包含 `google.com`：

| 查询域名 | 是否匹配 | 说明 |
|---------|---------|------|
| `google.com` | ✅ | 完全匹配 |
| `www.google.com` | ✅ | 前缀匹配（子域名） |
| `mail.google.com` | ✅ | 前缀匹配（子域名） |
| `notgoogle.com` | ❌ | 没有完整匹配 |
| `google.com.fake` | ❌ | 后缀不匹配 |

## 配置示例

### YAML 配置

```yaml
lists:
  direct:
    type: direct
    format: text
    path: ./direct_domains.txt      # 指定文件路径
  
  proxy:
    type: proxy
    format: text
    path: ./proxy_domains.txt       # 指定文件路径
  
  adblock:
    type: adblock
    format: text
    path: ./adblock_domains.txt     # 指定文件路径
```

### 文件路径

路径可以是：
- **相对路径**：相对于应用程序的工作目录
  ```yaml
  path: ./direct_domains.txt
  path: domains/direct.txt
  ```

- **绝对路径**：完整的文件系统路径
  ```yaml
  path: /etc/creskyDNS/direct_domains.txt
  path: C:\creskyDNS\lists\direct.txt
  ```

## 预定义列表文件

项目包含以下预定义的域名列表：

1. **direct_domains.txt**：国内直连域名列表
2. **proxy_domains.txt**：需要代理的国外域名列表
3. **adblock_domains.txt**：广告屏蔽域名列表
4. **custom_domains.txt**：用户自定义域名列表

## 最佳实践

### 1. 组织清晰

```text
# 国内站点
baidu.com
qq.com
weibo.com

# 国外站点
google.com
facebook.com
```

### 2. 避免重复

列表中的重复项不会导致错误，但会降低效率：
```text
google.com
google.com    ❌ 避免重复
```

### 3. 性能考虑

- 文件越小，加载越快
- 建议将相关的域名组织到一个文件中
- 可以创建多个列表文件，在配置中分别指定

### 4. 维护

```text
# 更新时间：2024-01-01
# 来源：示例列表

google.com
facebook.com
# twitter.com  # 暂时禁用
```

## 故障排除

### 域名未被加载

1. **检查文件路径**：确保配置中指定的路径正确
2. **检查文件权限**：确保应用有读取权限
3. **检查文件编码**：使用 UTF-8 编码
4. **查看日志**：应用启动时会输出域名加载的日志信息

### 域名匹配不工作

1. **确认格式**：没有前后缀
2. **确认编码**：检查是否有隐藏的特殊字符
3. **检查拼写**：确保域名拼写正确

## 相关链接

- [配置文件说明](README.md)
- [规则匹配行为](RULE_MATCHING.md)
