# Rule Cache 功能说明

## 概述

**Rule Cache**（规则缓存）是 creskyDNS 0.1.0 版本新增的内存规则缓存机制，用于加速 DNS 查询解析。当域名匹配到某条规则后，系统会将该域名对应的规则和上游信息缓存到内存中，后续查询时可直接使用，无需再次进行规则匹配。

## 功能特性

### 核心特性
- ✅ **内存缓存**：规则映射存储在内存中，查询速度极快（< 0.1ms）
- ✅ **自动清理**：系统 reload 时自动清空所有 rule.cache 内容
- ✅ **极简格式**：仅缓存 rule 和 upstream 映射，内存占用极少
- ✅ **热点优化**：高频查询的域名自动缓存规则，大幅提升性能

## 数据格式

### 缓存格式

```
|rule|upstream|
```

**字段说明**：
- `rule`：匹配到的规则 ID（如 "china_domains"、"global_domains"、"adblock"）
- `upstream`：对应的上游 DNS 标签名称（如 "ali"、"google"、"ad_hole"）
- **严格用 `|` 作为分隔符**

### 示例

```
|china_domains|ali|
|global_domains|google|
|adblock|ad_hole|
|local_domains|local_dns|
```

## DNS 解析流程

### 解析顺序（三层架构）

```
收到 DNS 查询请求
    ↓
┌─────────────────────────────────────────────────┐
│ 1. Rule Cache（内存规则缓存）                    │
│    ↓ 命中                                       │
│    直接使用缓存的 upstream 解析 ─────────────────┤
│    ↓ 未命中                                     │
├─────────────────────────────────────────────────┤
│ 2. DNS Cache（DNS 缓存）                        │
│    ↓ 命中                                       │
│    返回缓存的 DNS 结果 ──────────────────────────┤
│    ↓ 未命中                                     │
├─────────────────────────────────────────────────┤
│ 3. Rules 规则匹配                               │
│    ↓                                           │
│    匹配成功 → 写入 Rule Cache                    │
│    ↓                                           │
│    使用对应 upstream 查询                        │
│    ↓                                           │
│    将结果写入 DNS Cache                          │
│    ↓                                           │
└─────────────────返回查询结果 ──────────────────┘
```

### 详细流程说明

#### 第一层：Rule Cache（最快）
- **查询时间**：< 0.1ms
- **命中条件**：之前已解析过该域名
- **处理逻辑**：
  1. 在内存中查找域名对应的 rule 和 upstream
  2. 如果找到，直接使用该 upstream 进行 DNS 查询
  3. 跳过复杂的规则匹配过程

#### 第二层：DNS Cache
- **查询时间**：< 1ms
- **命中条件**：DNS 结果在 TTL 有效期内
- **处理逻辑**：
  1. 检查 DNS 缓存中是否有该域名的解析结果
  2. 如果有且未过期，直接返回缓存的 IP 地址
  3. 无需查询上游 DNS

#### 第三层：Rules 规则匹配
- **查询时间**：10-100ms（取决于规则复杂度和上游延迟）
- **触发条件**：Rule Cache 和 DNS Cache 均未命中
- **处理逻辑**：
  1. 按照规则组和优先级进行域名匹配
  2. 找到匹配的规则后，**写入 Rule Cache**
  3. 使用对应的 upstream 查询 DNS
  4. 将查询结果写入 DNS Cache
  5. 返回查询结果

## 缓存生命周期

### 写入时机
- **自动写入**：每当域名匹配到一条规则后，立即写入 rule.cache
- **写入内容**：规则 ID 和对应的 upstream 标签名称
- **存储位置**：内存中（不持久化到文件）

### 清空时机
- **触发条件**：系统 reload（重新加载配置）时
- **清空范围**：所有 rule.cache 内容
- **清空原因**：确保规则配置变更后，旧的缓存不会影响新的规则匹配

### 无 TTL 机制
- Rule Cache **没有过期时间**
- 缓存内容仅在 reload 时清空
- 这样设计是因为规则匹配结果通常不会在运行时改变

## 性能对比

| 解析方式 | 耗时估算 | 说明 |
|---------|---------|------|
| **Rule Cache 命中** | < 0.1ms | 内存查找，最快 |
| **DNS Cache 命中** | < 1ms | 无需上游查询 |
| **规则匹配 + 查询** | 10-100ms | 需要规则匹配和上游查询 |

**性能提升**：
- Rule Cache 命中时，相比规则匹配，性能提升 **100-1000 倍**
- 对于高频查询的域名（如公司内网域名、常用网站），效果显著

## 使用场景

### 场景 1：高频域名查询

**典型案例**：公司内部频繁访问的域名

```
第一次查询 mail.company.com：
  1. Rule Cache: 未命中
  2. DNS Cache: 未命中
  3. Rules 匹配: local_domains → upstream=local_dns
  4. 写入 Rule Cache: |local_domains|local_dns|
  5. 查询 upstream=local_dns，获取 IP: 192.168.1.100
  6. 写入 DNS Cache（TTL: 300s）
  7. 返回结果
  耗时：~50ms

第二次查询 mail.company.com（5 分钟内）：
  1. Rule Cache: 命中 |local_domains|local_dns|
  2. DNS Cache: 命中，返回 192.168.1.100
  ✅ 耗时：< 1ms（性能提升 50 倍）

第三次查询 mail.company.com（DNS Cache 过期后）：
  1. Rule Cache: 命中 |local_domains|local_dns|
  2. DNS Cache: 未命中（TTL 已过期）
  3. 直接使用 upstream=local_dns 查询（跳过规则匹配）
  4. 更新 DNS Cache
  5. 返回结果
  ✅ 耗时：~10ms（省略规则匹配，性能提升 5 倍）
```

### 场景 2：系统 Reload

**触发条件**：配置文件更新，需要重新加载

```
Reload 前：
  Rule Cache 中有 1000 条缓存记录

Reload 操作：
  1. 系统接收到 reload 信号
  2. 清空所有 Rule Cache 内容
  3. 重新加载配置文件
  4. 更新规则和上游配置
  5. Rule Cache 为空

Reload 后查询 mail.company.com：
  1. Rule Cache: 已清空，未命中
  2. DNS Cache: 可能命中（如果未过期）
  3. 重新进行规则匹配
  4. 写入新的 Rule Cache
  ✅ 确保使用最新的规则配置
```

### 场景 3：规则变更

**案例**：将某域名从 local_dns 改为 ali

```yaml
# 变更前配置
rules:
  main:
    - local_domains,local_dns  # mail.company.com 在此列表

# 变更后配置
rules:
  main:
    - local_domains,ali  # mail.company.com 使用新的 upstream
```

**处理流程**：
```
变更前 Rule Cache: |local_domains|local_dns|

执行 Reload：
  ✅ Rule Cache 清空

变更后查询 mail.company.com：
  1. Rule Cache: 未命中（已清空）
  2. 规则匹配: local_domains → upstream=ali（新配置）
  3. 写入 Rule Cache: |local_domains|ali|
  4. 使用新的 upstream=ali 查询
  ✅ 正确使用新的 upstream
```

## 内存占用

### 单条记录大小估算

```
格式: |rule|upstream|
示例: |china_domains|ali|

字段大小:
- 分隔符: 2 字节（两个 |）
- rule: 平均 20 字节
- upstream: 平均 10 字节
- 总计: ~32 字节/条
```

### 容量估算

| 缓存记录数 | 内存占用 |
|-----------|---------|
| 1,000 条 | ~32 KB |
| 10,000 条 | ~320 KB |
| 100,000 条 | ~3.2 MB |
| 1,000,000 条 | ~32 MB |

**结论**：即使缓存百万条记录，内存占用也仅 32MB，对现代服务器来说微不足道。

## 实现要点

### 数据结构（建议）

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Rule Cache 结构
pub struct RuleCache {
    // domain -> (rule, upstream)
    cache: Arc<RwLock<HashMap<String, (String, String)>>>,
}

impl RuleCache {
    // 查询缓存
    pub async fn get(&self, domain: &str) -> Option<(String, String)> {
        let cache = self.cache.read().await;
        cache.get(domain).cloned()
    }

    // 写入缓存
    pub async fn set(&self, domain: String, rule: String, upstream: String) {
        let mut cache = self.cache.write().await;
        cache.insert(domain, (rule, upstream));
    }

    // 清空缓存（reload 时调用）
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}
```

### Reload 逻辑

```rust
// 处理 reload 信号
async fn handle_reload(rule_cache: &RuleCache) {
    // 1. 清空 Rule Cache
    rule_cache.clear().await;
    
    // 2. 重新加载配置文件
    let new_config = load_config().await?;
    
    // 3. 更新规则和上游
    update_rules_and_upstreams(new_config).await?;
    
    log::info!("Reload completed, rule cache cleared");
}
```

## 注意事项

### ⚠️ Reload 清空机制
- 系统 reload 时会**清空所有 rule.cache**，确保规则变更生效
- Reload 后首次查询会稍慢（需要重新规则匹配），后续查询恢复正常
- 建议在业务低峰期进行 reload 操作

### ⚠️ 内存存储
- Rule Cache 仅存储在**内存**中，不持久化到文件
- 应用重启后，Rule Cache 自动清空
- 这是设计特性，确保配置变更能够立即生效

### ⚠️ 无 TTL 机制
- Rule Cache **没有过期时间**，只在 reload 时清空
- 如果规则配置变更，**必须执行 reload** 才能清空旧缓存
- 不支持自动刷新或定时过期

### ⚠️ 与 DNS Cache 的区别

| 特性 | Rule Cache | DNS Cache |
|-----|-----------|----------|
| **存储内容** | rule → upstream 映射 | domain → IP 映射 |
| **存储位置** | 内存 | 内存 + 可选文件 |
| **TTL 机制** | 无，仅 reload 清空 | 有，基于 DNS TTL |
| **清空条件** | Reload 时 | TTL 归 0 时 |
| **性能** | < 0.1ms | < 1ms |

## 最佳实践

### 1. 合理规划 Reload 时机
- 在业务低峰期执行 reload
- 避免频繁 reload（会清空 Rule Cache，影响性能）
- 批量修改配置后一次性 reload

### 2. 监控缓存命中率
- 记录 Rule Cache 命中次数
- 分析高频查询域名
- 优化规则配置以提高命中率

### 3. 配合 DNS Cache 使用
- Rule Cache 和 DNS Cache 互补
- Rule Cache 优化规则匹配速度
- DNS Cache 优化 DNS 查询速度

### 4. 测试配置变更
- 变更规则配置后，先测试
- 确认 reload 后规则生效
- 验证 Rule Cache 正确清空

## 故障排查

### 问题 1：规则变更后未生效

**症状**：修改规则配置后，查询仍使用旧的 upstream

**原因**：未执行 reload，Rule Cache 中仍是旧的映射

**解决方案**：
```bash
# 发送 reload 信号（具体命令取决于实现）
kill -HUP <pid>
# 或
systemctl reload creskyDNS
```

### 问题 2：性能未提升

**症状**：启用 Rule Cache 后，性能无明显提升

**可能原因**：
1. 查询的域名每次都不同（无法命中缓存）
2. DNS Cache TTL 过短，频繁过期
3. 规则匹配本身已很快，Rule Cache 优势不明显

**解决方案**：
- 分析查询日志，确认域名重复率
- 适当延长 DNS Cache 的 min_ttl
- 优化规则配置，减少规则数量

### 问题 3：内存占用过高

**症状**：Rule Cache 占用内存超过预期

**可能原因**：
- 查询的域名种类过多
- 未定期 reload，缓存持续累积

**解决方案**：
- 监控 Rule Cache 大小
- 定期 reload 清空缓存
- 考虑实现 LRU 淘汰机制（高级特性）

## 相关文档

- [PROJECT_FEATURES.md](PROJECT_FEATURES.md) - 第 6 节：规则缓存详细说明
- [CONFIG_EXAMPLES.md](CONFIG_EXAMPLES.md) - DNS 解析流程说明
- [README.md](README.md) - 快速开始和解析流程
- [CACHE_OUTPUT_FEATURE.md](CACHE_OUTPUT_FEATURE.md) - DNS Cache 功能说明

## 版本历史

- **v0.1.0** (2026-01-10)：新增 Rule Cache 功能
  - 内存规则缓存机制
  - Reload 时自动清空
  - 三层 DNS 解析架构（Rule Cache → DNS Cache → Rules）
  - 格式：`|rule|upstream|`
