# Cache 输出文件功能说明

## 概述

为了方便调试和优化缓存策略，creskyDNS 0.1.0 版本新增了缓存输出文件功能。每个缓存配置可通过 `output` 字段指定一个输出文件路径，用于记录当前缓存的内容及其生存时间。

## 功能描述

### 1. 配置语法

```yaml
cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/main.cache.txt"  # 可选，缓存输出文件路径
```

- **output**：可选字段，指定缓存输出文件的路径
- 无 `output` 字段时，不生成缓存文件
- 支持相对路径（相对于应用运行目录）和绝对路径

### 2. 输出文件格式

```
|cache ID|rule ID|domain|ttl|
```

**字段说明**：
- **cache ID**：缓存配置的标识符（如 "main"、"local"）
- **rule ID**：该域名匹配的规则 ID（如 "china_domains"、"global_domains"）
- **domain**：被缓存的域名
- **ttl**：该条目的剩余生存时间（秒）
- **严格用 `|` 作为分隔符**

### 3. 文件维护规则

- **追加新条目**：新缓存条目追加到文件末尾，避免重复记录
- **删除过期条目**：当 `ttl` 值归 0 时，从文件中删除该条目
- **文件重建**：应用启动时，若文件存在则进行验证和清理

### 4. 冷启动机制

**冷启动**是指应用启动或重启时从缓存输出文件快速恢复缓存内容的过程。

#### 工作原理

```
应用启动
    ↓
读取 cache.output 文件
    ↓
解析文件中的每条记录 (domain, cache ID, rule ID, ttl)
    ↓
根据 rule ID 确定对应的 upstream
    ↓
使用指定的 upstream 进行 DNS 查询（刷新解析）
    ↓
将结果导入缓存
    ↓
缓存恢复完成，应用就绪
```

#### 启动流程

1. **扫描缓存文件**：应用启动时自动查找所有配置的 `cache.output` 文件
2. **验证文件内容**：检查文件格式，跳过无效或过期条目（TTL ≤ 0）
3. **获取域名列表**：从文件中提取所有有效的域名记录
4. **查询解析**：
   - 根据 rule ID 查找对应的 upstream
   - 使用该 upstream 对每个域名进行 DNS 查询
   - 获取新的 IP 地址和 TTL
5. **更新缓存**：将查询结果写入内存缓存
6. **输出通知**：更新缓存文件中的 TTL（基于查询结果）

#### 配置参数

```yaml
cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/main.cache.txt"  # 冷启动时自动加载此文件
    # 冷启动相关配置（可选）
    cold_start:
      enabled: true           # 是否启用冷启动（默认 true）
      timeout: 5000           # 冷启动超时时间（毫秒，默认 5000）
      parallel: 10            # 并发查询数（默认 10）
```

#### 冷启动的优势

- ✅ **快速恢复**：应用重启后无需重新查询，直接从文件加载缓存
- ✅ **无缝切换**：用户感受不到应用重启的影响
- ✅ **流量优化**：减少应用启动时的外网查询流量
- ✅ **可靠备份**：缓存数据持久化，防止重启丢失

#### 冷启动的限制

- ⏱️ **TTL 限制**：文件中的 TTL 可能不准确（因为文件保存时间与启动时间有间隔）
- 🔄 **必须刷新**：系统会重新查询以获取最新的 IP 地址和准确的 TTL
- 📊 **查询并发限制**：为避免冲击上游 DNS，冷启动查询采用受限并发

### 5. 使用场景

#### 调试和监控
- 实时查看缓存状态
- 监控缓存命中率和覆盖的域名数量
- 验证 TTL 配置是否合理

#### 规则优化
- 分析高频查询域名
- 识别应该添加到特定规则的新域名
- 评估不同规则的生效情况

#### 性能分析
- 离线分析 DNS 查询模式
- 识别缓存热点数据
- 评估缓存大小是否合适

#### 故障恢复
- 应用重启时快速恢复缓存内容
- 减少启动时的外网查询
- 提高应用可用性和稳定性

## 配置示例

### 基础示例

```yaml
cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/main.cache.txt"
  
  local:
    size: 1000
    min_ttl: 300
    max_ttl: 604800
    output: "./output/cache/local.cache.txt"
```

### 多缓存配置示例

```yaml
cache:
  # 快速缓存（短 TTL）
  fast:
    size: 5000
    min_ttl: 30
    max_ttl: 1800
    output: "./output/cache/fast.cache.txt"
  
  # 标准缓存（中等 TTL）
  standard:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/standard.cache.txt"
    # 启用冷启动功能
    cold_start:
      enabled: true
      timeout: 5000
      parallel: 10
  
  # 持久缓存（长 TTL）
  persistent:
    size: 50000
    min_ttl: 300
    max_ttl: 604800
    output: "./output/cache/persistent.cache.txt"
    # 冷启动配置：长超时，低并发
    cold_start:
      enabled: true
      timeout: 10000          # 更长的超时，大文件需要更多时间
      parallel: 5             # 较低的并发，避免上游压力过大
```

### 冷启动完整配置示例

```yaml
# 当启用冷启动时，应用启动流程如下：
cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/main.cache.txt"
    cold_start:
      enabled: true                    # ← 启用冷启动
      timeout: 5000                    # 最多等待 5 秒
      parallel: 10                     # 同时查询 10 个域名

# 上游 DNS 配置
upstreams:
  ali:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"
  
  google:
    addr: "https://dns.google/dns-query"
    cache: "main"

# 规则配置
rules:
  main:
    - china_domains,ali              # rule ID = "china_domains", upstream = "ali"
    - global_domains,google          # rule ID = "global_domains", upstream = "google"

# 当有 cache.output 文件时：
# 1. 读取 ./output/cache/main.cache.txt
#    内容示例：
#    |main|china_domains|example.com|3600|
#    |main|global_domains|google.com|300|
#
# 2. 解析每条记录：
#    - domain=example.com, rule_id=china_domains → 使用 upstream=ali 查询
#    - domain=google.com, rule_id=global_domains → 使用 upstream=google 查询
#
# 3. 并发查询（最多 10 个同时）
#
# 4. 将结果更新到缓存和文件中
#    新的缓存文件示例：
#    |main|china_domains|example.com|3600|
#    |main|global_domains|google.com|120|   # TTL 已更新
```

## 文件示例

```
|main|china_domains|example.com|3600|
|main|china_domains|test.cn|7200|
|local|local_domains|localhost|86400|
|main|global_domains|google.com|300|
|main|adblock|ad.doubleclick.net|1800|
```

## 相关文件位置

本功能已在以下文档中详细记录：

| 文件 | 位置 | 说明 |
|-----|------|------|
| [config.yaml](config.yaml) | 第 23-35 行 | 完整配置示例 |
| [CONFIG_EXAMPLES.md](CONFIG_EXAMPLES.md) | 缓存配置部分 | 字段说明表 |
| [PROJECT_FEATURES.md](PROJECT_FEATURES.md) | 第 5.3 节 | 详细功能说明 |
| [README.md](README.md) | 缓存配置部分 | 配置说明 |
| [USAGE.md](USAGE.md) | 第 2 节 | 使用说明 |

## 注意事项

1. **文件目录**：确保 `output` 指定的目录存在，或由应用自动创建
2. **权限要求**：应用需要对输出目录的读写权限
3. **磁盘空间**：根据缓存大小和 TTL 配置，合理估算文件大小
4. **性能影响**：启用缓存输出会有轻微性能开销，可选择性启用
5. **文件编码**：输出文件使用 UTF-8 编码，支持中文域名

## 更新历史

- **v0.1.0** (2026-01-10)：新增缓存输出文件功能
  - 支持自定义输出文件路径
  - 标准化输出格式：`|cache ID|rule ID|domain|ttl|`
  - 自动删除过期缓存条目（TTL 归 0）
  - 支持所有缓存配置（main、local、custom 等）

