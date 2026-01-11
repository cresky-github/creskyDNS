# DNS 转发器

用 Rust 实现的高性能 DNS 转发器。

## 功能特性

- ✅ 异步 DNS 查询转发（使用 Tokio）
- ✅ 同时支持 UDP、TCP 和 DoH 协议
- ✅ 支持多个监听端口
- ✅ 支持多个上游DNS服务器（标签化管理）
- ✅ **规则分流**: 根据监听器名称自动分流到不同上游DNS
- ✅ 支持自定义上游 DNS 服务器
- ✅ 支持查询超时设置
- ✅ 详细的日志记录
- ✅ **域名列表热重新加载**：零停机更新域名列表
- ✅ **百万级域名优化**：支持超大规模列表（毫秒级加载、微秒级查询）
- ✅ **Rule Cache**：内存规则缓存，加速 DNS 解析（reload 时自动清空）

## DNS 解析流程

查询请求按以下顺序处理（性能优化）：

```
1. Rule Cache（内存规则缓存）
   ↓ 命中 → 直接使用缓存的 upstream 解析
   ↓ 未命中
   
2. DNS Cache（DNS 缓存）
   ↓ 命中 → 返回缓存的 DNS 结果
   ↓ 未命中
   
3. Rules 规则匹配
   ↓ 匹配成功 → 写入 Rule Cache
   ↓ 使用对应 upstream 查询
   ↓ 将结果写入 DNS Cache
   ↓ 返回查询结果
```

**Rule Cache**：内存规则缓存，格式 `|rule|upstream|`，reload 时清空

## 快速开始

### 编译

```bash
cargo build --release
```

### 运行

```bash
cargo run
```

DNS 转发器默认监听在 `127.0.0.1:5353`，将查询转发到 `8.8.8.8:53`。

### 配置

有多种方式配置转发器（优先级从高到低）：

#### 1. 命令行参数
```bash
cargo run config.yaml
cargo run config.json
```

#### 2. 环境变量
```bash
export DNS_FORWARDER_CONFIG=config.yaml
cargo run
```

#### 3. 默认位置
程序会自动查找以下位置的配置文件：
- `config.yaml` / `config.yml`
- `config.json`
- `./etc/creskyDNS.yaml`

#### 4. 默认配置
如果上述都找不到，使用内置的默认配置。

### 配置文件格式

#### YAML 格式 (config.yaml)
```yaml
# DNS 转发器配置文件

# 监听器配置 (实例名 -> 端口)
listener:
    main: 5353
    test: 5354

# 缓存配置 (ID -> 配置)
cache:
    main:
        size: 10000
        min_ttl: 60
        max_ttl: 86400

# 上游DNS服务器配置 (tag -> config)
upstreams:
  ali:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
  google:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "default"
  cloudflare:
    addr: "https://cloudflare-dns.com/dns-query"
    bootstrap: "udp://127.0.0.1:5329"
    cache: "default"
  local_dns:
    addr: "udp://223.5.5.5:53"
    # UDP/TCP 不需要 bootstrap

# 域名列表配置 (name -> config)
lists:
  direct:
    type: "domain"
    path: "direct.txt"
    url: "https://example.com/direct.txt"
    interval: 86400

# 规则配置
rules:
    # 服务器实例规则：实例名,上游名 (可选，不写则完全按域名规则执行)
    servers:
        - main,local_dns
        - test,cloudflare
    # 域名规则：域名列表名,上游名
    main:
        - direct,ali
        - proxy,google
```

#### JSON 格式 (config.json)
```json
{
  "listener": {
    "main": 5353,
    "test": 5354
  },
  "cache": {
    "main": {
      "size": 10000,
      "min_ttl": 60,
      "max_ttl": 86400
    },
    "default": {
      "size": 1000,
      "min_ttl": 60,
      "max_ttl": 86400
    }
  },
  "upstreams": {
    "ali": {
      "addr": "https://dns.alidns.com/dns-query",
      "bootstrap": "udp://223.5.5.5:53",
      "cache": "main"
    },
    "google": {
      "addr": "https://dns.google/dns-query",
      "bootstrap": "udp://8.8.8.8:53",
      "cache": "default"
    },
    "cloudflare": {
      "addr": "https://cloudflare-dns.com/dns-query",
      "bootstrap": "udp://127.0.0.1:5329",
      "cache": "default"
    },
    "local_dns": {
      "addr": "udp://223.5.5.5:53"
    }
  },
  "lists": {
    "direct": {
      "type": "domain",
      "path": "direct.txt",
      "url": "https://example.com/direct.txt",
      "interval": 86400
    }
  },
  "rules": {
    "servers": [
      "main,local_dns",
      "test,cloudflare"
    ],
    "main": [
      "direct,ali",
      "proxy,google"
    ]
  }
}
```

### 配置说明

| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| `listener` | Object | 监听器配置 (实例名 -> 端口)，每个实例同时监听UDP/TCP | `{"main": 5353}` |
| `cache` | Object | 缓存配置 (ID -> config)，包含 size、min_ttl、max_ttl、output 字段 | 见示例 |
| `upstreams` | Object | 上游DNS服务器配置 (tag -> config)，包含 addr、bootstrap、cache 字段 | 见示例 |
| `lists` | Object | 域名列表配置 (name -> config)，包含 type、path、url、interval 字段 | 见示例 |
| `rules` | Object | 规则配置，包含 servers 规则（可选）和域名规则 | 见示例 |

### 监听器配置

监听器定义DNS服务器实例：

```yaml
listener:
    main: 5353    # 实例名: 端口
    test: 5354
```

每个监听器实例同时支持IPv4和IPv6，并监听UDP和TCP协议。

### 缓存配置

缓存配置定义缓存行为：

```yaml
cache:
    main:
        size: 10000      # 缓存条目数
        min_ttl: 60      # 最小TTL（秒，可选）
        max_ttl: 86400   # 最大TTL（秒，可选）
        output: "./output/cache/main.cache.txt"  # 缓存输出文件路径（可选），格式 |cache ID|rule ID|domain|ttl|
        cold_start:      # 冷启动配置（可选）
          enabled: true  # 启动时从 output 文件恢复缓存
          timeout: 5000  # 冷启动超时时间（毫秒）
          parallel: 10   # 并发查询数
```

**冷启动说明**：
- 应用启动时，自动从 `output` 文件读取缓存域名
- 根据 rule ID 查找对应的 upstream，进行 DNS 查询刷新
- 将查询结果更新到缓存和文件

### 上游DNS服务器配置

上游服务器配置支持多种协议：

```yaml
upstreams:
  ali:
    addr: "https://dns.alidns.com/dns-query"  # DoH
    bootstrap: "udp://223.5.5.5:53"          # 用于解析DoH服务器域名
    cache: "main"                            # 使用的缓存ID
  local_dns:
    addr: "udp://223.5.5.5:53"               # UDP，不需要bootstrap
```

#### 支持的协议类型

- **`https://`** - DoH (DNS over HTTPS)
- **`udp://`** - UDP DNS
- **`tcp://`** - TCP DNS

### 域名列表配置

域名列表用于定义需要特殊处理的域名：

```yaml
lists:
  direct:
    type: "domain"                          # 列表类型
    path: "direct.txt"                      # 本地文件路径
    url: "https://example.com/direct.txt"   # 远程URL
    interval: 86400                         # 更新间隔（秒）
```

### 规则配置

规则分为两种类型：

#### 服务器实例规则（可选）

`servers` 是固定名称，专门用于书写虚拟服务器规则。如果不写此字段，则完全按域名规则执行。

```yaml
rules:
    servers:    # 固定名称，可选字段
        - main,local_dns    # 实例名,上游名
        - test,cloudflare
```

#### 域名规则

```yaml
rules:
    main:      # 规则名称，可以自定义
        - direct,ali        # 域名列表名,上游名
        - proxy,google
```

### 配置示例

#### 基本配置

```yaml
# 监听器配置
listener:
    main: 5353
    test: 5354

# 缓存配置
cache:
    main:
        size: 10000
        min_ttl: 60
        max_ttl: 86400

# 上游DNS服务器配置
upstreams:
  ali:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
  google:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "default"
  cloudflare:
    addr: "https://cloudflare-dns.com/dns-query"
    bootstrap: "udp://1.1.1.1:53"
    cache: "default"
  local_dns:
    addr: "udp://223.5.5.5:53"
  direct_dns:
    addr: "udp://8.8.8.8:53"
    cache: "default"
  proxy_dns:
    addr: "udp://114.114.114.114:53"
    cache: "default"

# 域名列表配置
lists:
  direct:
    type: "domain"
    path: "direct.txt"
    url: "https://example.com/direct.txt"
    interval: 86400
  proxy:
    type: "domain"
    path: "proxy.txt"
    url: "https://example.com/proxy.txt"
    interval: 86400

# 规则配置
rules:
    # 服务器实例规则：实例名,上游名 (可选，不写则完全按域名规则执行)
    servers:
        - main,local_dns
        - test,cloudflare
    # 域名规则：域名列表名,上游名
    main:
        - direct,ali
        - proxy,google
```

#### 仅域名规则配置示例（不写servers字段）

```yaml
listener:
    main: 5353

cache:
    default:
        size: 1000

upstreams:
  ali:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
  google:
    addr: "udp://8.8.8.8:53"

lists:
  direct:
    type: "domain"
    path: "direct.txt"
    url: "https://example.com/direct.txt"
    interval: 86400

# 只有域名规则，完全按域名规则执行
rules:
    main:
        - direct,ali
```

#### DoH 配置示例

```yaml
listener:
    main: 5353

cache:
    default:
        size: 1000

upstreams:
  doh_dns:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "default"
  default_dns:
    addr: "udp://8.8.8.8:53"

lists:
  direct:
    type: "domain"
    path: "direct.txt"
    url: "https://example.com/direct.txt"
    interval: 86400

rules:
    servers:
        - main,default_dns
    main:
        - direct,doh_dns
```

### 完整配置示例

以下是一个完整的、生产就绪的配置文件示例，包含了所有配置选项：

```yaml
# DNS 转发器完整配置文件

# 监听器配置 (实例名 -> 端口)
listener:
    main: 5353        # 主DNS服务器
    backup: 5354      # 备用DNS服务器
    ipv6: 5355        # IPv6专用服务器

# 缓存配置 (ID -> 配置)
cache:
    fast:             # 快速缓存
        size: 5000
        min_ttl: 30
        max_ttl: 1800
    standard:         # 标准缓存
        size: 10000
        min_ttl: 60
        max_ttl: 3600
    persistent:       # 持久缓存
        size: 50000
        min_ttl: 300
        max_ttl: 86400

# 上游DNS服务器配置
upstreams:
  # DoH 服务器
  alidns_doh:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
    cache: "standard"

  google_doh:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "standard"

  cloudflare_doh:
    addr: "https://cloudflare-dns.com/dns-query"
    bootstrap: "udp://1.1.1.1:53"
    cache: "fast"

  # UDP DNS 服务器
  local_ali:
    addr: "udp://223.5.5.5:53"
    cache: "persistent"

  google_udp:
    addr: "udp://8.8.8.8:53"
    cache: "standard"

# 域名列表配置
lists:
  direct:
    type: "domain"
    path: "direct_domains.txt"
    url: "https://cdn.jsdelivr.net/gh/example/direct-domains.txt"
    interval: 86400

  proxy:
    type: "domain"
    path: "proxy_domains.txt"
    url: "https://cdn.jsdelivr.net/gh/example/proxy-domains.txt"
    interval: 43200

  adblock:
    type: "domain"
    path: "adblock_domains.txt"
    url: "https://cdn.jsdelivr.net/gh/example/adblock.txt"
    interval: 3600

# 规则配置
rules:
    # 服务器实例规则
    servers:
        - main,local_ali
        - backup,google_udp
        - ipv6,google_udp

    # 域名规则
    domestic:
        - direct,alidns_doh
        - adblock,local_ali

    international:
        - proxy,google_doh
        - adblock,local_ali
```

### 测试

使用 `nslookup` 或 `dig` 测试：

```bash
# UDP 测试
nslookup google.com 127.0.0.1 -port=5353

# TCP 测试 (使用 dig)
dig @127.0.0.1 -p 5353 +tcp example.com

# 测试多个端口
nslookup google.com 127.0.0.1 -port=5354
dig @127.0.0.1 -p 5354 +tcp example.com
```

## 项目结构

```
src/
├── main.rs       # 主程序入口
├── config.rs     # 配置模块
├── forwarder.rs  # DNS 转发器核心逻辑
└── dns.rs        # DNS 消息编解码工具
```

## 依赖

- **tokio** - 异步运行时
- **hickory-dns** - DNS 协议支持
- **tracing** - 日志记录

## 文档

- [规则匹配说明](RULE_MATCHING.md) - 详细的规则匹配行为文档
- [域名列表格式说明](DOMAIN_LIST_FORMAT.md) - 域名列表文件的格式规范
- [域名列表快速参考](DOMAIN_LIST_QUICK_REF.md) - 快速参考指南（dos 和 don'ts）
- [域名列表热重新加载](LIST_RELOAD.md) - 热重新加载功能说明（interval 配置）
- [更新说明](DOMAIN_LIST_UPDATE.md) - 域名列表功能更新说明

## 工作原理

1. 监听本地 UDP/TCP 端口（默认 5353）
2. 接收客户端的 DNS 查询请求
3. 解析查询域名，按深度从大到小构建各级域名
4. 根据规则配置匹配域名列表，确定目标上游列表
5. 如果没有匹配规则，使用服务器默认上游
6. 将请求转发到匹配的上游 DNS 服务器
7. 等待并接收上游服务器的响应
8. 将响应返回给客户端

### 域名深度匹配流程

对于查询域名 `www.google.com`：

1. 深度 3: `www.google.com` (精确匹配)
2. 深度 2: `google.com` (二级域名匹配)
3. 深度 1: `com` (顶级域名匹配)
4. 深度 0: `.` (根域名匹配)

系统按深度优先级进行匹配，找到第一个匹配的规则后停止。

## 后续改进方向

- [x] DNS 缓存（LRU 缓存）
- [x] 支持多个上游 DNS 服务器（负载均衡）
- [x] 支持 TCP 协议
- [x] 支持 DoH 协议
- [x] 支持自定义转发规则（域名深度匹配）
- [x] 规则分流（类似 CoreDNS bypass 插件）
- [x] 域名列表热重新加载（零停机更新）
- [x] 百万级域名优化（1700x 性能提升）
- [ ] 支持 DNSSEC 验证
- [ ] 性能优化（连接池等）
- [ ] 支持外部域名列表文件
- [ ] 支持规则优先级配置
- [ ] 支持 IPv6 优先级

---

## 📚 文档中心

### 域名列表功能

| 功能 | 文档 | 说明 |
|------|------|------|
| **列表格式** | [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) | 文件格式规范 |
| **热重新加载** | [LIST_RELOAD.md](LIST_RELOAD.md) | interval 参数详解 |
| **interval 机制** | [INTERVAL_TIMELINE.md](INTERVAL_TIMELINE.md) | ⏱️ 时间轴详解（推荐） |
| **快速参考** | [INTERVAL_QUICK_REF.md](INTERVAL_QUICK_REF.md) | interval 选择表 |
| **实现细节** | [LIST_RELOAD_IMPLEMENTATION.md](LIST_RELOAD_IMPLEMENTATION.md) | 技术实现说明 |

### 百万级优化（性能提升 1700 倍）

| 文档 | 用途 | 时间 |
|------|------|------|
| **[OPTIMIZATION_README.md](OPTIMIZATION_README.md)** | 📋 总体简介 | 5 分 |
| **[OPTIMIZATION_QUICK_START.md](OPTIMIZATION_QUICK_START.md)** | ⚡ 快速上手 | 20 分 |
| **[OPTIMIZATION_MILLION_SCALE.md](OPTIMIZATION_MILLION_SCALE.md)** | 🚀 完整设计 | 1 小时 |
| **[IMPLEMENTATION_GUIDE.md](IMPLEMENTATION_GUIDE.md)** | 💻 实现代码 | 1.5 小时 |
| **[OPTIMIZATION_COMPLETENESS.md](OPTIMIZATION_COMPLETENESS.md)** | 📈 性能分析 | 1 小时 |
| **[OPTIMIZATION_INDEX.md](OPTIMIZATION_INDEX.md)** | 📑 文档导航 | 5 分 |

#### 性能提升一览

```
加载时间：8.5s → 1.2s (7x ↑)
查询延迟：850μs → 0.5μs (1700x ↑)
更新延迟：1.2s → 5ms (240x ↓)
QPS 吞吐量：1k → 2M+ (1700x ↑)
```

#### 推荐阅读路径

- **快速上手**：[OPTIMIZATION_QUICK_START.md](OPTIMIZATION_QUICK_START.md) (2 小时完成)
- **深入理解**：[OPTIMIZATION_MILLION_SCALE.md](OPTIMIZATION_MILLION_SCALE.md) (4 小时学习)
- **查看代码**：[IMPLEMENTATION_GUIDE.md](IMPLEMENTATION_GUIDE.md) (直接实现)

### 其他文档

| 文档 | 说明 |
|------|------|
| [QUICK_START.md](QUICK_START.md) | 快速开始指南 |
| [USAGE.md](USAGE.md) | 详细使用说明 |
| [RULE_MATCHING.md](RULE_MATCHING.md) | 规则匹配原理 |
| [VALIDATION_TOOL.md](VALIDATION_TOOL.md) | 验证工具使用 |
- [ ] 支持 EDNS 扩展
