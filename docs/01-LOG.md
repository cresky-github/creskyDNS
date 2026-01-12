# 01 - 日志模块

## 📋 目录

- [概述](#概述)
- [配置说明](#配置说明)
- [日志格式](#日志格式)
- [日志级别](#日志级别)
- [文件管理](#文件管理)
- [使用示例](#使用示例)
- [日志分析](#日志分析)
- [最佳实践](#最佳实践)

---

## 概述

creskyDNS 内置了功能完整的日志系统，支持：

✅ **多级别日志**：trace / debug / info / warn / error  
✅ **自动轮转**：按时间或大小自动切分  
✅ **备份管理**：自动清理旧日志  
✅ **结构化格式**：管道符分隔，便于解析  
✅ **高性能**：异步写入，不阻塞主线程  
✅ **灵活配置**：可动态开关

---

## 配置说明

### YAML 配置示例

```yaml
log:
  enabled: true                          # 是否启用日志
  path: "./logs/creskyDNS.log"           # 日志文件路径
  level: "info"                          # 日志级别
  max_time: 7d                           # 最大保存时间
  max_size: 100MB                        # 单文件最大大小
  max_backups: 14                        # 最大备份数量
```

### 配置字段详解

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| **enabled** | boolean | 否 | `true` | 是否启用日志功能 |
| **path** | string | 否 | `./logs/creskyDNS.log` | 日志文件存放路径（支持相对和绝对路径） |
| **level** | string | 否 | `info` | 日志记录级别（trace/debug/info/warn/error） |
| **max_time** | string | 否 | `7d` | 日志文件最大保存时间（例如：3d、7d、30d） |
| **max_size** | string | 否 | `10MB` | 单个日志文件的最大大小（例如：10MB、100MB、1GB） |
| **max_backups** | integer | 否 | `5` | 最大备份文件数量（超过后删除最旧的） |

### 配置示例

#### 示例 1：开发环境（详细日志）

```yaml
log:
  enabled: true
  path: "./logs/dev.log"
  level: "debug"              # 详细调试信息
  max_time: 1d                # 1 天轮转
  max_size: 50MB              # 较大的文件
  max_backups: 3              # 保留 3 个备份
```

#### 示例 2：生产环境（精简日志）

```yaml
log:
  enabled: true
  path: "/var/log/creskyDNS/app.log"
  level: "warn"               # 只记录警告和错误
  max_time: 7d                # 7 天轮转
  max_size: 100MB             # 更大的文件
  max_backups: 10             # 保留更多备份
```

#### 示例 3：禁用日志

```yaml
log:
  enabled: false              # 关闭日志（提升性能）
```

#### 示例 4：最小配置（使用默认值）

```yaml
log:
  level: "info"               # 只指定日志级别，其他使用默认值
```

---

## 日志格式

### 标准格式

每行日志采用**管道符（`|`）分隔**的结构化格式：

```
|日期|时间|日志级别|进程名|模块名称|日志内容|
```

### 字段说明

| 字段 | 格式 | 示例 | 说明 |
|------|------|------|------|
| **日期** | `YYYY-MM-DD` | `2026-01-10` | 年-月-日 |
| **时间** | `HH:MM:SS,mmm` | `14:35:42,123` | 时:分:秒,毫秒（毫秒固定 3 位） |
| **日志级别** | 大写字母 | `INFO` | TRACE/DEBUG/INFO/WARN/ERROR |
| **进程名** | 字符串 | `creskyDNS` | 应用程序名称 |
| **模块名称** | 字符串 | `dns_resolver` | 代码模块或组件名称 |
| **日志内容** | 字符串 | `查询域名: google.com` | 实际的日志消息 |

### 格式示例

```log
|2026-01-10|14:35:42,123|INFO|creskyDNS|main|DNS 转发器启动成功|
|2026-01-10|14:35:42,156|INFO|creskyDNS|listener|监听器 'main' 端口: 5353|
|2026-01-10|14:35:43,001|DEBUG|creskyDNS|list_loader|域名列表 'direct' 加载: 1245 个域名|
|2026-01-10|14:36:15,789|DEBUG|creskyDNS|dns_resolver|查询: www.google.com (A)|
|2026-01-10|14:36:15,792|DEBUG|creskyDNS|rule_matcher|匹配规则: direct → ali_dns|
|2026-01-10|14:36:15,845|INFO|creskyDNS|dns_resolver|响应: 142.250.185.68|
|2026-01-10|14:38:00,234|WARN|creskyDNS|cache|缓存已满，淘汰最旧条目|
|2026-01-10|14:40:12,567|ERROR|creskyDNS|upstream|上游 DNS 连接失败: timeout|
```

### 格式优势

✅ **易于解析**：管道符分隔，适合 awk、grep、cut 等工具  
✅ **结构化**：每个字段位置固定，便于程序化处理  
✅ **高精度时间**：毫秒级时间戳，便于性能分析  
✅ **清晰可读**：人工阅读也很友好

---

## 日志级别

### 级别说明

| 级别 | 英文 | 用途 | 典型场景 |
|------|------|------|----------|
| **TRACE** | 跟踪 | 最详细的跟踪信息 | 代码执行路径、函数调用栈 |
| **DEBUG** | 调试 | 调试信息 | 变量值、中间结果、详细流程 |
| **INFO** | 信息 | 常规信息 | 启动、配置加载、正常操作 |
| **WARN** | 警告 | 警告信息 | 配置异常、性能降低、可恢复错误 |
| **ERROR** | 错误 | 错误信息 | 连接失败、解析错误、致命异常 |

### 级别过滤

设置日志级别后，**只记录该级别及更高级别的日志**：

```yaml
# level: "info"
# 将记录: INFO + WARN + ERROR
# 不记录: TRACE + DEBUG
```

| 配置级别 | 记录的级别 |
|----------|-----------|
| `trace` | TRACE + DEBUG + INFO + WARN + ERROR |
| `debug` | DEBUG + INFO + WARN + ERROR |
| `info` | INFO + WARN + ERROR |
| `warn` | WARN + ERROR |
| `error` | ERROR |

### 级别选择建议

| 环境 | 推荐级别 | 原因 |
|------|----------|------|
| **开发环境** | `debug` 或 `trace` | 需要详细调试信息 |
| **测试环境** | `debug` | 便于发现问题 |
| **生产环境** | `info` | 平衡性能和可观测性 |
| **高负载生产** | `warn` | 减少 I/O，只记录异常 |
| **紧急调试** | `trace` | 临时开启详细日志 |

### 示例日志内容

#### TRACE 级别

```log
|2026-01-10|14:35:42,123|TRACE|creskyDNS|dns_resolver|进入函数: resolve_domain()|
|2026-01-10|14:35:42,124|TRACE|creskyDNS|dns_resolver|参数: domain=google.com, record_type=A|
|2026-01-10|14:35:42,125|TRACE|creskyDNS|rule_matcher|开始匹配规则...|
```

#### DEBUG 级别

```log
|2026-01-10|14:35:42,130|DEBUG|creskyDNS|cache|缓存查询: google.com → 未命中|
|2026-01-10|14:35:42,135|DEBUG|creskyDNS|upstream|选择上游: ali_dns (https://dns.alidns.com)|
|2026-01-10|14:35:42,145|DEBUG|creskyDNS|dns_resolver|解析结果: 142.250.185.68|
```

#### INFO 级别

```log
|2026-01-10|14:35:42,000|INFO|creskyDNS|main|DNS 转发器启动成功|
|2026-01-10|14:35:42,050|INFO|creskyDNS|listener|监听器 'main' 绑定端口: 5353|
|2026-01-10|14:35:43,000|INFO|creskyDNS|list_loader|域名列表 'direct' 加载成功: 1245 个域名|
```

#### WARN 级别

```log
|2026-01-10|14:38:00,234|WARN|creskyDNS|cache|缓存使用率 95%，接近上限|
|2026-01-10|14:39:15,456|WARN|creskyDNS|upstream|上游 DNS 响应慢: 1500ms|
|2026-01-10|14:40:30,789|WARN|creskyDNS|list_loader|域名列表格式异常，已跳过 3 行|
```

#### ERROR 级别

```log
|2026-01-10|14:40:12,567|ERROR|creskyDNS|upstream|上游 DNS 连接失败: timeout after 5s|
|2026-01-10|14:41:25,890|ERROR|creskyDNS|listener|监听器端口冲突: 5353 已被占用|
|2026-01-10|14:42:00,123|ERROR|creskyDNS|config|配置文件解析失败: invalid YAML syntax|
```

---

## 文件管理

### 自动轮转

日志文件支持两种轮转策略：

#### 1. 按时间轮转（max_time）

```yaml
log:
  max_time: 3d    # 3 天后自动创建新文件
```

**工作原理**：
- 每隔 `max_time` 时间，创建新的日志文件
- 旧文件重命名：`creskyDNS.log.2026-01-10`

**时间格式**：
- `1d` = 1 天
- `7d` = 7 天
- `30d` = 30 天

#### 2. 按大小轮转（max_size）

```yaml
log:
  max_size: 10MB    # 文件达到 10MB 后自动创建新文件
```

**工作原理**：
- 文件大小达到 `max_size` 时，创建新文件
- 旧文件重命名：`creskyDNS.log.1`, `creskyDNS.log.2`

**大小格式**：
- `10MB` = 10 兆字节
- `100MB` = 100 兆字节
- `1GB` = 1 吉字节

### 备份清理

```yaml
log:
  max_backups: 5    # 最多保留 5 个备份文件
```

**工作原理**：
- 当备份文件数超过 `max_backups` 时，删除最旧的
- 保证磁盘空间不会无限增长

**文件命名示例**：
```
creskyDNS.log              # 当前日志
creskyDNS.log.2026-01-10   # 按日期轮转的备份
creskyDNS.log.1            # 按序号轮转的备份
creskyDNS.log.2
creskyDNS.log.3
```

### 磁盘空间计算

**公式**：
```
最大磁盘占用 ≈ max_size × (max_backups + 1)
```

**示例**：
```yaml
log:
  max_size: 100MB
  max_backups: 10

# 最大磁盘占用 ≈ 100MB × 11 = 1100MB ≈ 1.1GB
```

---

## 使用示例

### 示例 1：查看实时日志

```bash
# Linux/macOS
tail -f logs/creskyDNS.log

# Windows PowerShell
Get-Content logs/creskyDNS.log -Wait -Tail 20
```

### 示例 2：搜索错误日志

```bash
# 查找所有错误
grep "|ERROR|" logs/creskyDNS.log

# 查找特定模块的错误
grep "|ERROR|.*|upstream|" logs/creskyDNS.log
```

### 示例 3：统计查询数量

```bash
# 统计今天的查询次数
grep "2026-01-10" logs/creskyDNS.log | grep "查询:" | wc -l

# 按小时统计
grep "2026-01-10" logs/creskyDNS.log | cut -d'|' -f2 | cut -d':' -f1 | sort | uniq -c
```

### 示例 4：分析响应时间

```bash
# 提取所有响应时间日志
grep "响应时间:" logs/creskyDNS.log | cut -d'|' -f6
```

---

## 日志分析

### 使用 awk 分析

#### 1. 统计各级别日志数量

```bash
awk -F'|' '{print $3}' logs/creskyDNS.log | sort | uniq -c
```

**输出示例**：
```
   1234 INFO
    567 DEBUG
     89 WARN
     12 ERROR
```

#### 2. 提取特定时间段日志

```bash
awk -F'|' '$2 >= "14:00:00" && $2 <= "15:00:00"' logs/creskyDNS.log
```

#### 3. 统计各模块日志数量

```bash
awk -F'|' '{print $5}' logs/creskyDNS.log | sort | uniq -c | sort -rn
```

### 使用 Python 分析

```python
import re
from collections import Counter

# 解析日志文件
def parse_log(filename):
    with open(filename, 'r', encoding='utf-8') as f:
        for line in f:
            parts = line.strip().split('|')
            if len(parts) >= 6:
                yield {
                    'date': parts[1],
                    'time': parts[2],
                    'level': parts[3],
                    'process': parts[4],
                    'module': parts[5],
                    'message': parts[6] if len(parts) > 6 else ''
                }

# 统计错误类型
errors = Counter()
for entry in parse_log('logs/creskyDNS.log'):
    if entry['level'] == 'ERROR':
        errors[entry['module']] += 1

print("错误统计:", errors.most_common())
```

### 使用 Logstash/Fluentd

**Logstash 配置示例**：

```ruby
input {
  file {
    path => "/path/to/logs/creskyDNS.log"
    start_position => "beginning"
  }
}

filter {
  dissect {
    mapping => {
      "message" => "|%{date}|%{time}|%{level}|%{process}|%{module}|%{log_message}|"
    }
  }
}

output {
  elasticsearch {
    hosts => ["localhost:9200"]
    index => "creskyDNS-%{+YYYY.MM.dd}"
  }
}
```

---

## 最佳实践

### 1. 日志级别选择

✅ **推荐做法**：
- 生产环境默认使用 `info`
- 遇到问题时动态调整为 `debug`（如果支持热重新加载）
- 高负载场景使用 `warn` 减少 I/O

❌ **不推荐**：
- 生产环境长期使用 `trace`（日志量过大）
- 生产环境使用 `error`（信息不足）

### 2. 文件轮转配置

✅ **推荐做法**：
- 根据日志量合理设置 `max_size`（通常 50-100MB）
- 根据保留需求设置 `max_time`（通常 7-30 天）
- 保留足够的备份数（`max_backups: 7-14`）

❌ **不推荐**：
- 文件过大（> 1GB，难以打开和分析）
- 备份过少（< 3，问题追溯困难）

### 3. 性能优化

✅ **推荐做法**：
- 使用异步日志写入
- 高负载场景提升日志级别
- 定期清理旧日志

❌ **不推荐**：
- 同步阻塞式日志（影响性能）
- 无限制保留日志（占满磁盘）

### 4. 安全和隐私

✅ **推荐做法**：
- 不记录敏感信息（密码、密钥）
- 设置适当的文件权限（Linux: 640）
- 定期备份重要日志

❌ **不推荐**：
- 记录完整的请求内容（可能包含隐私）
- 日志文件权限过于宽松

### 5. 日志监控

✅ **推荐做法**：
- 监控 ERROR 级别日志
- 设置日志告警（如错误率超过阈值）
- 定期分析日志趋势

---

## 配置模板

### 开发环境

```yaml
log:
  enabled: true
  path: "./logs/dev.log"
  level: "debug"
  max_time: 1d
  max_size: 50MB
  max_backups: 3
```

### 测试环境

```yaml
log:
  enabled: true
  path: "./logs/test.log"
  level: "debug"
  max_time: 3d
  max_size: 100MB
  max_backups: 5
```

### 生产环境

```yaml
log:
  enabled: true
  path: "/var/log/creskyDNS/app.log"
  level: "info"
  max_time: 7d
  max_size: 100MB
  max_backups: 14
```

### 高负载生产环境

```yaml
log:
  enabled: true
  path: "/var/log/creskyDNS/app.log"
  level: "warn"
  max_time: 7d
  max_size: 200MB
  max_backups: 10
```

---

## 相关文档

- [02-LISTENER.md](02-LISTENER.md) - 监听器模块
- [03-CACHE.md](03-CACHE.md) - 缓存模块
- [04-UPSTREAMS.md](04-UPSTREAMS.md) - 上游服务器模块
- [05-LISTS.md](05-LISTS.md) - 列表模块
- [06-RULES.md](06-RULES.md) - 规则模块
