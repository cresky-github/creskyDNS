# 03 - 缓存模块

## 📋 目录

- [概述](#概述)
- [配置说明](#配置说明)
- [缓存策略](#缓存策略)
- [缓存输出文件](#缓存输出文件)
- [冷启动机制](#冷启动机制)
- [规则缓存](#规则缓存)
- [Domain Cache 规范](#domain-cache-规范)
- [使用场景](#使用场景)
- [性能优化](#性能优化)
- [最佳实践](#最佳实践)

---

## 概述

creskyDNS 提供强大的多级缓存机制，包括 DNS 缓存（Domain Cache）和规则缓存（Rule Cache），大幅提升查询性能并减少上游负载。

### 核心特性

✅ **多级缓存**：Rule Cache + Domain Cache 两层缓存  
✅ **LRU 淘汰**：最近最少使用条目优先淘汰  
✅ **TTL 控制**：灵活的 min_ttl 和 max_ttl 配置  
✅ **缓存输出**：支持将缓存内容输出到文件  
✅ **冷启动**：应用重启时快速恢复缓存  
✅ **高性能**：内存缓存，查询延迟 < 1ms  
✅ **自动清理**：过期条目自动删除

---

## 配置说明

### 基本配置

```yaml
cache:
  main:
    size: 10000           # 缓存条目数
    min_ttl: 60           # 最小 TTL（秒）
    max_ttl: 86400        # 最大 TTL（秒）
  
  local:
    size: 1000
    min_ttl: 300
    max_ttl: 604800
```

### 配置字段详解

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| **size** | integer | ✅ | 无 | 缓存条目最大数量 |
| **min_ttl** | integer | ✅ | 无 | 最小 TTL（秒），强制最小缓存时间 |
| **max_ttl** | integer | ✅ | 无 | 最大 TTL（秒），限制最大缓存时间 |
| **output** | string | 否 | 无 | 缓存输出文件路径（调试用） |
| **cold_start** | object | 否 | 无 | 冷启动配置 |

### 配置示例

#### 示例 1：标准缓存配置

```yaml
cache:
  main:
    size: 10000
    min_ttl: 60           # 至少缓存 1 分钟
    max_ttl: 86400        # 最多缓存 1 天
```

#### 示例 2：快速缓存（短 TTL）

```yaml
cache:
  fast:
    size: 5000
    min_ttl: 30           # 至少缓存 30 秒
    max_ttl: 1800         # 最多缓存 30 分钟
```

#### 示例 3：持久缓存（长 TTL）

```yaml
cache:
  persistent:
    size: 50000
    min_ttl: 300          # 至少缓存 5 分钟
    max_ttl: 604800       # 最多缓存 7 天
```

---

## 缓存策略

### LRU 淘汰策略

当缓存达到上限时，采用 LRU（Least Recently Used）策略淘汰最久未使用的条目。

```
缓存已满（size = 10000）
    ↓
新条目需要加入
    ↓
淘汰最久未访问的条目
    ↓
加入新条目
```

### TTL 控制

#### min_ttl：最小 TTL

强制设置最小缓存时间，防止 TTL 过短导致频繁查询。

```yaml
cache:
  main:
    min_ttl: 60   # 即使上游返回 TTL=10，也缓存 60 秒
```

**工作原理**：
```
上游返回 TTL=30
    ↓
min_ttl=60 (配置)
    ↓
实际使用 TTL=60（取较大值）
```

#### max_ttl：最大 TTL

限制最大缓存时间，防止 TTL 过长导致数据过期。

```yaml
cache:
  main:
    max_ttl: 86400   # 即使上游返回 TTL=604800，也只缓存 86400 秒
```

**工作原理**：
```
上游返回 TTL=604800（7天）
    ↓
max_ttl=86400（1天，配置）
    ↓
实际使用 TTL=86400（取较小值）
```

#### TTL 计算公式

```
实际 TTL = max(min_ttl, min(上游 TTL, max_ttl))
```

**示例**：
```yaml
cache:
  main:
    min_ttl: 60
    max_ttl: 86400

# 情况 1：上游 TTL = 30
# 实际 TTL = max(60, min(30, 86400)) = max(60, 30) = 60

# 情况 2：上游 TTL = 300
# 实际 TTL = max(60, min(300, 86400)) = max(60, 300) = 300

# 情况 3：上游 TTL = 604800（7天）
# 实际 TTL = max(60, min(604800, 86400)) = max(60, 86400) = 86400
```

---

## 缓存输出文件

### 功能说明

每个缓存配置可通过 `output` 字段指定输出文件，用于记录当前缓存的内容。

### 配置方式

```yaml
cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/main.cache.txt"  # 缓存输出文件
```

### 文件格式

```
|cache ID|rule ID|domain|ttl|
```

**字段说明**：
- `cache ID`：缓存配置的标识符（如 "main"）
- `rule ID`：该域名匹配的规则 ID
- `domain`：被缓存的域名
- `ttl`：该条目的剩余生存时间（秒）
- **严格用 `|` 作为分隔符**

### 文件示例

```
|main|china_domains|example.com|3600|
|main|china_domains|test.cn|7200|
|main|global_domains|google.com|300|
|main|adblock|ad.doubleclick.net|1800|
|local|local_domains|localhost|86400|
```

### 文件维护

- **追加新条目**：新缓存条目追加到文件末尾
- **删除过期条目**：当 TTL 归 0 时，从文件中删除该条目
- **应用启动时重建**：验证并清理文件内容

### 使用场景

- 📊 **调试监控**：实时查看缓存状态
- 🔍 **规则优化**：分析高频域名，优化规则
- 📈 **性能分析**：识别缓存热点数据
- 💾 **故障恢复**：应用重启时快速恢复

---

## 冷启动机制

### 功能概述

**冷启动**（Cold Start）是应用启动时从缓存输出文件自动恢复缓存内容的过程。

### 工作流程

```
应用启动
    ↓
读取 cache.output 文件
    ↓
解析文件中的记录 (cache ID, rule ID, domain, ttl)
    ↓
根据 rule ID 找到对应的 upstream
    ↓
使用该 upstream 进行 DNS 查询（刷新解析）
    ↓
将查询结果导入缓存
    ↓
缓存恢复完成，应用就绪
```

### 配置方式

```yaml
cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/main.cache.txt"
    cold_start:
      enabled: true           # 是否启用冷启动
      timeout: 5000           # 超时时间（毫秒）
      parallel: 10            # 并发查询数
```

### 配置参数

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| **enabled** | boolean | `true` | 是否启用冷启动功能 |
| **timeout** | integer | `5000` | 冷启动超时时间（毫秒） |
| **parallel** | integer | `10` | 并发查询数 |

### 优势

- ✅ **快速恢复**：应用重启后无需重新查询
- ✅ **无缝服务**：用户感受不到应用重启
- ✅ **流量优化**：减少启动时的外网查询
- ✅ **可靠备份**：缓存数据持久化

### 限制

- ⏱️ **TTL 不准确**：文件中的 TTL 可能过期
- 🔄 **必须刷新**：系统会重新查询获取最新 IP 和 TTL
- 📊 **并发限制**：避免冲击上游 DNS

### 完整配置示例

```yaml
cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
    output: "./output/cache/main.cache.txt"
    cold_start:
      enabled: true
      timeout: 5000
      parallel: 10

upstreams:
  ali:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"
  
  google:
    addr: "https://dns.google/dns-query"
    cache: "main"

rules:
  main:
    - china_domains,ali
    - global_domains,google
```

**冷启动流程**：
1. 读取 `./output/cache/main.cache.txt`
2. 解析：`|main|china_domains|example.com|3600|`
3. 根据 `china_domains` 找到 `ali` upstream
4. 使用 `ali` 查询 `example.com`
5. 更新缓存和文件

---

## 规则缓存

### Rule Cache 概述

**Rule Cache**（规则缓存）是内存中的规则映射缓存，用于加速 DNS 查询。

### 缓存格式

```
|rule|upstream|
```

**示例**：
```
|china_domains|ali|
|global_domains|google|
|adblock|ad_hole|
```

### 生命周期

- **写入时机**：每当域名匹配到规则后，立即写入
- **清空时机**：系统 reload 时清空
- **存储位置**：仅内存，不持久化

### DNS 解析流程

```
收到 DNS 查询请求
    ↓
1. 检查 Rule Cache（内存规则缓存）
    ↓ 命中
    直接使用缓存的 upstream 解析 ─────────┐
    ↓ 未命中                            ↓
2. 检查 Domain Cache（DNS 缓存）         ↓
    ↓ 命中                            ↓
    返回缓存的 DNS 结果 ──────────────────┤
    ↓ 未命中                            ↓
3. 按 Rules 规则进行匹配                 ↓
    ↓                                  ↓
    匹配成功 → 写入 Rule Cache            ↓
    ↓                                  ↓
    使用对应 upstream 查询                ↓
    ↓                                  ↓
    将结果写入 Domain Cache               ↓
    ↓                                  ↓
    └────────────返回查询结果 ──────────┘
```

### 性能对比

| 解析方式 | 耗时估算 | 说明 |
|---------|---------|------|
| **Rule Cache 命中** | < 0.1ms | 内存查找，最快 |
| **Domain Cache 命中** | < 1ms | 无需上游查询 |
| **规则匹配 + 查询** | 10-100ms | 需要规则匹配和上游查询 |

### 使用示例

```
第一次查询 www.google.com：
  1. Rule Cache: 未命中
  2. Domain Cache: 未命中
  3. Rules 匹配: global_domains → upstream=google
  4. 写入 Rule Cache: |global_domains|google|
  5. 查询 upstream=google，获取 IP
  6. 写入 Domain Cache
  7. 返回结果
  耗时：~50ms

第二次查询 www.google.com（DNS Cache 未过期）：
  1. Rule Cache: 命中 |global_domains|google|
  2. Domain Cache: 命中，直接返回
  ✅ 耗时：< 1ms（性能提升 50 倍）

第三次查询 www.google.com（DNS Cache 已过期）：
  1. Rule Cache: 命中 |global_domains|google|
  2. Domain Cache: 未命中
  3. 直接使用 upstream=google 查询（跳过规则匹配）
  4. 更新 Domain Cache
  5. 返回结果
  ✅ 耗时：~10ms（省略规则匹配，性能提升 5 倍）
```

---

## Domain Cache 规范

### 记录格式

```
|cache ID|rule|domain|ttl|IP(上游返回的其它内容)|
```

**字段说明**：
- `cache ID`：缓存实例标识（例如 `main`）
- `rule`：命中的规则（为 lists 中的域名/模式）
- `domain`：实际被解析的完整域名
- `ttl`：单位秒；当 TTL 归 0 时，从文件中删除
- `IP(上游返回的其它内容)`：解析结果与上游的其它原始信息

**示例**：
```
|main|ads.example.com|img.ads.example.com|292|203.0.113.5,203.0.113.6(A)|
|main|*.example.net|a.b.example.net|360|2001:db8::5(AAAA)|
```

### 解析流程

**解析顺序**（rule.cache → domain.cache → rules）：

1. **检查 rule.cache**：
   - 基于传入域名，进行轻量规则匹配，得到 `rule`
   - 若该 `rule` 存在于 `rule.cache`，获取对应的 `upstream`

2. **检查 domain.cache**：
   - 携带上一步得到的 `rule` 与当前请求的 `domain`，在 `domain.cache` 中查找
   - 命中且 TTL 有效则返回缓存的 `IP(其它内容)`

3. **按 rules 正常解析**：
   - 依既有规则进行上游查询
   - 成功后：
     - 写入/更新 `rule.cache`（`|rule|upstream|`）
     - 写入/更新 `domain.cache`（`|cache ID|rule|domain|ttl|IP(...)|`）

---

## 使用场景

### 场景 1：高并发查询

**需求**：支持每秒数千次 DNS 查询

**配置**：
```yaml
cache:
  main:
    size: 50000           # 大容量缓存
    min_ttl: 300          # 较长的最小 TTL
    max_ttl: 86400
```

**效果**：
- 缓存命中率 > 90%
- 平均响应时间 < 5ms
- 上游查询减少 90%

### 场景 2：开发环境（短缓存）

**需求**：快速迭代，需要实时获取最新 DNS 结果

**配置**：
```yaml
cache:
  dev:
    size: 1000
    min_ttl: 0            # 不强制缓存
    max_ttl: 300          # 最多缓存 5 分钟
```

### 场景 3：生产环境（稳定缓存）

**需求**：稳定性优先，减少上游查询

**配置**：
```yaml
cache:
  prod:
    size: 20000
    min_ttl: 300          # 至少缓存 5 分钟
    max_ttl: 86400        # 最多缓存 1 天
    output: "./output/cache/prod.cache.txt"
    cold_start:
      enabled: true
      timeout: 10000
      parallel: 20
```

### 场景 4：分级缓存

**需求**：不同类型的查询使用不同的缓存策略

**配置**：
```yaml
cache:
  # 快速缓存（短 TTL）
  fast:
    size: 5000
    min_ttl: 30
    max_ttl: 1800
  
  # 标准缓存（中等 TTL）
  standard:
    size: 10000
    min_ttl: 60
    max_ttl: 86400
  
  # 持久缓存（长 TTL）
  persistent:
    size: 50000
    min_ttl: 300
    max_ttl: 604800

upstreams:
  fast_upstream:
    addr: "udp://8.8.8.8:53"
    cache: "fast"
  
  standard_upstream:
    addr: "https://dns.google/dns-query"
    cache: "standard"
  
  persistent_upstream:
    addr: "https://dns.alidns.com/dns-query"
    cache: "persistent"
```

---

## 性能优化

### 1. 缓存大小选择

| 场景 | 推荐 size | 说明 |
|------|----------|------|
| 个人使用 | 1,000-5,000 | 足够日常使用 |
| 小型企业 | 5,000-10,000 | 支持数百用户 |
| 中型企业 | 10,000-50,000 | 支持数千用户 |
| 大型企业 | 50,000-100,000 | 支持万级用户 |

### 2. TTL 配置建议

| 场景 | min_ttl | max_ttl | 说明 |
|------|---------|---------|------|
| 开发环境 | 0 | 300 | 快速迭代 |
| 测试环境 | 60 | 3600 | 平衡性能和实时性 |
| 生产环境 | 300 | 86400 | 稳定性优先 |
| 高负载环境 | 600 | 604800 | 性能优先 |

### 3. 内存估算

**单条缓存记录大小**：
- 域名：平均 30 字节
- IP 地址：16 字节
- 元数据：20 字节
- 总计：约 66 字节/条

**容量估算**：
```
1,000 条 ≈ 66 KB
10,000 条 ≈ 660 KB
100,000 条 ≈ 6.6 MB
```

### 4. 缓存命中率优化

✅ **提升命中率的方法**：
- 增大缓存容量（size）
- 适当延长 min_ttl
- 预热常用域名（冷启动）
- 分析热点域名并优化

---

## 最佳实践

### 1. 缓存容量配置

✅ **推荐**：
- 根据实际查询量配置容量
- 预留 20-30% 的余量
- 定期监控缓存使用率

❌ **不推荐**：
- 缓存过小（频繁淘汰）
- 缓存过大（内存浪费）

### 2. TTL 配置

✅ **推荐**：
- 生产环境：min_ttl >= 300
- 开发环境：max_ttl <= 300
- 根据业务特点调整

❌ **不推荐**：
- min_ttl = 0（频繁查询上游）
- max_ttl 过长（数据过期风险）

### 3. 缓存输出

✅ **推荐**：
- 生产环境启用 output 和 cold_start
- 定期分析缓存文件
- 监控缓存命中率

❌ **不推荐**：
- 开发环境启用（文件频繁变化）
- 不清理过期文件

### 4. 监控和调优

✅ **推荐**：
- 监控缓存命中率
- 分析热点域名
- 定期优化配置

---

## 相关文档

- [01-LOG.md](01-LOG.md) - 日志模块
- [02-LISTENER.md](02-LISTENER.md) - 监听器模块
- [04-UPSTREAMS.md](04-UPSTREAMS.md) - 上游服务器模块
- [05-LISTS.md](05-LISTS.md) - 列表模块
- [06-RULES.md](06-RULES.md) - 规则模块
