# ⚙️ 完整配置文件示例

## 📋 目录

- [快速开始](#快速开始)
- [完整配置示例](#完整配置示例)
- [分场景配置](#分场景配置)
- [高级配置](#高级配置)
- [配置说明](#配置说明)
- [验证配置](#验证配置)

---

## 快速开始

### 最小化配置

```yaml
# 文件: config.yaml

listener:
  main: 5353

upstreams:
  google:
    addr: "https://dns.google/dns-query"

rules:
  main:
    - .,google
```

**说明**：
- 监听 5353 端口
- 所有查询转发到 Google DNS
- 无域名列表，无缓存

---

## 完整配置示例

### 标准配置（推荐）

```yaml
# ============================================================
# creskyDNS 完整配置示例
# ============================================================

# ============================================================
# 1. 监听器配置
# ============================================================
listener:
  main: 5353        # 主监听器，监听本机所有网卡的 5353 端口
  backup: 5354      # 备用监听器（可选）
  test: 5355        # 测试监听器（可选）

# ============================================================
# 2. 上游 DNS 配置
# ============================================================
upstreams:
  # Google DNS
  google:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"        # DoH 初始化用的 bootstrap DNS
    cache: "main"                        # 使用 main 缓存
    timeout: 5000                        # 超时时间（毫秒）

  # Cloudflare DNS
  cloudflare:
    addr: "https://dns.cloudflare.com/dns-query"
    bootstrap: "udp://1.1.1.1:53"
    cache: "main"
    timeout: 5000

  # 阿里 DNS
  ali:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
    timeout: 5000

  # 114 DNS
  dns114:
    addr: "https://114.114.114.114:8443"
    bootstrap: "udp://114.114.114.114:53"
    cache: "main"
    timeout: 5000

  # 黑洞 DNS（拦截广告）
  blocked:
    addr: "udp://127.0.0.1:1"            # 指向本地无效地址，实现拦截
    cache: "main"
    timeout: 1000

  # 本地 DNS
  local:
    addr: "udp://192.168.1.1:53"         # 内网 DNS
    cache: "local"
    timeout: 3000

  # 备用 DNS
  backup:
    addr: "udp://8.8.4.4:53"
    cache: "main"
    timeout: 5000

# ============================================================
# 3. 域名列表配置
# ============================================================
lists:
  # 国内域名列表
  direct:
    type: "domain"
    format: "text"                       # 纯文本格式
    path: "./lists/china_domains.txt"
    interval: 3600                       # 1 小时倒计时
    description: "国内网站域名"

  # 代理域名列表
  proxy:
    type: "domain"
    format: "text"
    path: "./lists/proxy_domains.txt"
    interval: 3600
    description: "国际网站域名"

  # 广告域名列表
  adblock:
    type: "domain"
    format: "text"
    path: "./lists/adblock_domains.txt"
    interval: 86400                      # 1 天倒计时
    description: "广告域名"

  # 本地域名列表
  local_domains:
    type: "domain"
    format: "text"
    path: "./lists/local_domains.txt"
    interval: 0                          # 立即重新加载
    description: "本地开发域名"

  # IP CIDR 列表（新功能）
  china_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_ips.txt"
    interval: 86400                      # 1 天倒计时
    description: "国内 IP 地址段"

# ============================================================
# 4. 缓存配置
# ============================================================
cache:
  main:
    size: 10000                          # 缓存条目数
    min_ttl: 60                          # 最小 TTL（秒）
    max_ttl: 86400                       # 最大 TTL（秒）
    output: "./output/cache/main.cache.txt"  # 可选，缓存输出文件，格式 |cache ID|rule ID|domain|ttl|
    cold_start:                          # 冷启动配置（可选）
      enabled: true                      # 启用冷启动，应用启动时从 output 文件恢复缓存
      timeout: 5000                      # 冷启动超时时间（毫秒）
      parallel: 10                       # 并发查询数，避免冲击上游 DNS

  local:
    size: 1000
    min_ttl: 300                         # 本地 DNS 缓存较长
    max_ttl: 604800                      # 一周

# ============================================================
# 5. 规则配置
# ============================================================
rules:
  # 主规则组
  main:
    # 国内域名规则
    - direct,ali                         # 国内域名 → 阿里 DNS
    
    # 代理域名规则
    - proxy,google                       # 代理域名 → Google DNS
    
    # 广告拦截规则
    - adblock,blocked                    # 广告域名 → 黑洞 DNS
    
    # 默认规则
    - .,cloudflare                       # 其他域名 → Cloudflare DNS

  # 本地监听器规则
  servers:
    - main,ali                           # main 监听器 → 阿里 DNS
    - backup,google                      # backup 监听器 → Google DNS
    - test,cloudflare                    # test 监听器 → Cloudflare DNS

# ============================================================
# 6. 日志配置
# ============================================================
log:
  enabled: true
  path: "./logs/creskyDNS.log"
  level: "info"                          # debug/info/warn/error
  max_time: 7d                           # 7 天轮转
  max_size: 100MB                        # 单文件最大 100MB
  max_backups: 14                        # 保留 14 个备份

# ============================================================
# 7. 应用配置（可选）
# ============================================================
app:
  name: "creskyDNS"
  description: "高性能 DNS 转发器"
  version: "0.1.0"
```

### 配置说明

#### 监听器配置
```yaml
listener:
  main: 5353        # 监听 0.0.0.0:5353 (UDP 和 TCP)
  backup: 5354
  test: 5355
```

#### 上游 DNS 配置
| 字段 | 说明 | 必填 |
|------|------|------|
| `addr` | DNS 服务器地址（支持多种协议） | ✅ |
| `bootstrap` | DoH 初始化用的 bootstrap DNS | 否 |
| `cache` | 使用的缓存配置名称 | 否 |
| `timeout` | 请求超时（毫秒） | 否 |

#### 缓存配置
| 字段 | 说明 | 必填 |
|------|------|------|
| `size` | 缓存条目数量 | ✅ |
| `min_ttl` | 最小 TTL（秒） | ✅ |
| `max_ttl` | 最大 TTL（秒） | ✅ |
| `output` | 缓存输出文件路径，格式 `\|cache ID\|rule ID\|domain\|ttl\|` | 否 |

**缓存输出文件说明**：
- 如果指定 `output` 字段，系统会将缓存条目保存到指定文件
- 新条目追加到文件末尾
- 当 TTL 归 0 时，从文件中删除该条目
- 无 `output` 字段时，不生成缓存文件

**冷启动配置 (`cold_start`，可选）**：

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `enabled` | 是否启用冷启动功能 | `true` |
| `timeout` | 冷启动超时时间（毫秒） | `5000` |
| `parallel` | 并发查询数 | `10` |

**冷启动说明**：
- 应用启动时，自动读取 `output` 文件中的域名记录
- 根据 rule ID 找到对应的 upstream，使用该 upstream 重新查询
- 将查询结果导入缓存，用最新的 IP 和 TTL 更新缓存文件
- `parallel` 控制并发数，避免冲击上游 DNS

#### 域名列表配置
| 字段 | 说明 | 必填 |
|------|------|------|
| `type` | 列表类型（domain） | ✅ |
| `format` | 文件格式（text） | ✅ |
| `path` | 文件路径 | ✅ |
| `interval` | 重新加载倒计时（秒） | 否 |
| `description` | 列表描述 | 否 |

**行内注释支持（新）**：
- 列表文件中同一行的 `#` 之后内容将被忽略（支持 domain 与 ipcidr 列表）。
- 纯注释行（以 `#` 开头）与空行将被跳过。

**示例**：
```text
# domain 列表
google.com   # 谷歌
www.baidu.com # 百度

# ipcidr 列表
|39.156.0.0/16|CN|  # 国内 IP 段
|8.8.8.0/24|US|     # Google 段
```

#### DNS 解析流程

**解析顺序**（性能优化）：

```
1. 检查 Rule Cache（内存规则缓存）
   ↓ 命中 → 直接使用缓存的 upstream 解析
   ↓ 未命中
   
2. 检查 DNS Cache（DNS 缓存）
   ↓ 命中 → 返回缓存的 DNS 结果
   ↓ 未命中
   
3. 按 Rules 规则进行匹配
   ↓ 匹配成功 → 写入 Rule Cache
   ↓ 使用对应 upstream 查询
   ↓ 将结果写入 DNS Cache
   ↓ 返回查询结果
```

**Rule Cache 说明**：
- 格式：`|rule|upstream|`（内存存储，严格用 `|` 分隔）
- 生命周期：系统 reload 时清空所有 rule.cache 内容
- 优势：高频查询域名跳过规则匹配，大幅提升性能

#### 规则配置
```yaml
rules:
  main:
    - list_name,upstream_name    # 格式: 域名列表 → 上游 DNS
  
  # Final 规则（兜底规则，未分类域名的智能处理）
  final:
    primary_upstream: "dns_name"
    fallback_upstream: "backup_dns"
    ipcidr: "ipcidr_list_name"   # 使用 IP CIDR 列表判定国家代码
    output: "/path/to/output.txt"
```

**Final 规则字段说明**：
| 字段 | 类型 | 说明 |
|------|------|------|
| `primary_upstream` | string | ✅ 主上游 DNS 标签 |
| `fallback_upstream` | string | ✅ 备用上游 DNS 标签 |
| `ipcidr` | string | 否 IP CIDR 列表（用于国家代码判定） |
| `output` | string | 否 输出文件路径（记录未分类域名） |

**规则命中追踪（新功能）**：
- 当某个规则匹配成功后，命中的域名会追加到该规则使用的列表的命中文件“原名.hit.txt”。
- 例：`./lists/china_domains.txt` → 生成 `./lists/china_domains.hit.txt`。- **重要**：如果列表文件路径已包含 `.hit.`（如 `domains.hit.txt`），不会再创建 hit 文件。- 每行一个域名（纯域名），用于后续优化与分析。
- 注意：`servers` 组不记录命中（不产生 .hit.txt）。

---

## 分场景配置

### 场景 1：国内外分流

```yaml
# config-cn-global.yaml

listener:
  main: 5353

upstreams:
  # 国内 DNS
  cn_dns:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
  
  # 国际 DNS
  global_dns:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "main"

lists:
  cn_domains:
    type: "domain"
    format: "text"
    path: "./lists/china_domains.txt"
    interval: 3600

  global_domains:
    type: "domain"
    format: "text"
    path: "./lists/global_domains.txt"
    interval: 3600

rules:
  main:
    - cn_domains,cn_dns                # 国内域名 → 国内 DNS
    - global_domains,global_dns        # 国际域名 → 国际 DNS
    - .,cn_dns                         # 默认 → 国内 DNS

cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400

log:
  enabled: true
  path: "./logs/windspliving.log"
  level: "info"
  max_size: 100MB
  max_backups: 14
```

### 场景 2：广告过滤

```yaml
# config-adblock.yaml

listener:
  main: 5353

upstreams:
  # 正常 DNS
  clean_dns:
    addr: "https://dns.google/dns-query"
    cache: "main"
  
  # 黑洞 DNS（拦截）
  blocked_dns:
    addr: "udp://127.0.0.1:1"
    cache: "main"

lists:
  # 广告域名列表（从 easylist 等来源）
  adblock:
    type: "domain"
    format: "text"
    path: "./lists/adblock.txt"
    interval: 86400                    # 每天更新

rules:
  main:
    - adblock,blocked_dns              # 广告 → 黑洞
    - .,clean_dns                      # 其他 → 正常 DNS

cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400

log:
  enabled: true
  path: "./logs/adblock.log"
  level: "warn"                        # 只记录警告和错误
  max_size: 50MB
  max_backups: 7
```

### 场景 3：开发环境（快速迭代）

```yaml
# config-dev.yaml

listener:
  main: 5353
  local: 5354

upstreams:
  # 本地开发 DNS
  local_dns:
    addr: "udp://192.168.1.1:53"
    cache: "local"
  
  # 公网 DNS
  public_dns:
    addr: "https://dns.google/dns-query"
    cache: "main"

lists:
  # 本地域名（快速迭代，立即重新加载）
  local_hosts:
    type: "domain"
    format: "text"
    path: "./lists/local_hosts.txt"
    interval: 0                        # 立即重新加载！

rules:
  main:
    - local_hosts,local_dns            # 本地域名 → 内网 DNS
    - .,public_dns                     # 其他 → 公网 DNS

cache:
  main:
    size: 1000
    min_ttl: 0                         # 最小缓存时间为 0
    max_ttl: 300                       # 最多缓存 5 分钟
  
  local:
    size: 500
    min_ttl: 0
    max_ttl: 300

log:
  enabled: true
  path: "./logs/dev.log"
  level: "debug"                       # 详细日志
  max_size: 50MB
  max_backups: 3
```

### 场景 4：企业内网（多监听器）

```yaml
# config-enterprise.yaml

listener:
  internal: 53                         # 内网监听
  external: 5353                       # 外网监听
  dmz: 5354                           # DMZ 监听

upstreams:
  # 内网 DNS
  internal_dns:
    addr: "udp://10.0.0.1:53"
    cache: "internal"
  
  # 公网 DNS
  external_dns:
    addr: "https://dns.google/dns-query"
    cache: "external"
  
  # DMZ DNS
  dmz_dns:
    addr: "udp://192.168.1.1:53"
    cache: "dmz"

lists:
  # 内网域名
  internal_domains:
    type: "domain"
    format: "text"
    path: "./lists/internal_domains.txt"
    interval: 1800
  
  # 外网域名
  external_domains:
    type: "domain"
    format: "text"
    path: "./lists/external_domains.txt"
    interval: 3600

rules:
  # 按监听器分流
  servers:
    - internal,internal_dns            # 内网监听 → 内网 DNS
    - external,external_dns            # 外网监听 → 外网 DNS
    - dmz,dmz_dns                      # DMZ 监听 → DMZ DNS
  
  # 按域名分流
  main:
    - internal_domains,internal_dns
    - external_domains,external_dns
    - .,external_dns                   # 默认 → 外网 DNS

cache:
  internal:
    size: 5000
    min_ttl: 300
    max_ttl: 604800                    # 1 周
  
  external:
    size: 10000
    min_ttl: 60
    max_ttl: 86400                     # 1 天
  
  dmz:
    size: 2000
    min_ttl: 60
    max_ttl: 3600                      # 1 小时

log:
  enabled: true
  path: "/var/log/windspliving/app.log"
  level: "info"
  max_time: 7d
  max_size: 100MB
  max_backups: 14
```

### 场景 5：高性能生产环境

```yaml
# config-prod-performance.yaml

listener:
  main: 53                             # 使用标准端口
  backup: 5353

upstreams:
  # 高性能主 DNS
  primary:
    addr: "https://dns.google/dns-query"
    cache: "main"
    timeout: 2000                      # 更短的超时
  
  # 高性能备 DNS
  secondary:
    addr: "https://dns.cloudflare.com/dns-query"
    cache: "main"
    timeout: 2000
  
  # 本地 DNS 缓存（用于快速重定向）
  local_cache:
    addr: "udp://127.0.0.1:53"
    cache: "main"
    timeout: 500

lists:
  # 大型域名列表（百万级）
  china_domains:
    type: "domain"
    format: "text"
    path: "./lists/china_domains_1m.txt"
    interval: 86400                    # 每天更新
  
  global_domains:
    type: "domain"
    format: "text"
    path: "./lists/global_domains_1m.txt"
    interval: 86400

rules:
  main:
    - china_domains,primary            # 国内 → 主 DNS
    - global_domains,secondary         # 国际 → 备 DNS
    - .,primary                        # 默认 → 主 DNS

cache:
  main:
    size: 100000                       # 大缓存
    min_ttl: 60
    max_ttl: 86400

log:
  enabled: true
  path: "/var/log/windspliving/app.log"
  level: "warn"                        # 只记录异常
  max_time: 7d
  max_size: 200MB
  max_backups: 7
```

---

## 高级配置

### 故障转移配置

```yaml
# config-failover.yaml
# 目标: 主 DNS 失败时自动切换到备 DNS

listener:
  main: 5353

upstreams:
  # 主 DNS
  primary:
    addr: "https://dns.google/dns-query"
    cache: "main"
    timeout: 3000

  # 备用 DNS 1
  backup1:
    addr: "https://dns.cloudflare.com/dns-query"
    cache: "main"
    timeout: 3000

  # 备用 DNS 2
  backup2:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"
    timeout: 3000

rules:
  main:
    # 优先级递减：primary → backup1 → backup2
    - .,primary
    # 如果主 DNS 失败，自动尝试备用

log:
  enabled: true
  path: "./logs/failover.log"
  level: "warn"                        # 监控故障
```

### 多地域分流配置

```yaml
# config-geo-routing.yaml
# 目标: 根据地域选择不同的 DNS

listener:
  main: 5353

upstreams:
  # 亚太地区 DNS
  apac:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"

  # 欧美地区 DNS
  americas:
    addr: "https://dns.google/dns-query"
    cache: "main"

  # 其他地区 DNS
  default:
    addr: "https://dns.cloudflare.com/dns-query"
    cache: "main"

lists:
  # 亚太域名列表
  apac_domains:
    type: "domain"
    format: "text"
    path: "./lists/apac_domains.txt"
    interval: 3600

  # 欧美域名列表
  americas_domains:
    type: "domain"
    format: "text"
    path: "./lists/americas_domains.txt"
    interval: 3600

rules:
  main:
    - apac_domains,apac
    - americas_domains,americas
    - .,default

log:
  enabled: true
  path: "./logs/geo_routing.log"
  level: "info"
```

---

## 配置说明

### 支持的协议

| 协议 | 格式 | 示例 |
|------|------|------|
| UDP | `udp://host:port` | `udp://8.8.8.8:53` |
| TCP | `tcp://host:port` | `tcp://8.8.8.8:53` |
| DoH | `https://host/path` | `https://dns.google/dns-query` |
| DoT | `tls://host:port` | `tls://dns.google:853` |
| DoQ | `quic://host:port` | `quic://dns.adguard.com` |
| H3 | `h3://host:port` | `h3://dns.google` |

### 时间单位

| 单位 | 示例 | 说明 |
|------|------|------|
| **秒** | `60` | 数字直接表示秒 |
| **分钟** | `5m` | 5 分钟 = 300 秒 |
| **小时** | `1h` | 1 小时 = 3600 秒 |
| **天** | `7d` | 7 天 = 604800 秒 |

### 大小单位

| 单位 | 示例 | 说明 |
|------|------|------|
| **字节** | `1024` | 1024 字节 |
| **KB** | `10KB` | 10 千字节 |
| **MB** | `100MB` | 100 兆字节 |
| **GB** | `1GB` | 1 吉字节 |

### 路径说明

**相对路径**（相对于应用程序启动目录）：
```yaml
path: "./logs/app.log"
path: "./lists/domains.txt"
```

**绝对路径**：
```yaml
path: "/var/log/windspliving/app.log"      # Linux
path: "C:/logs/windspliving.log"           # Windows
path: "/var/log/dns/domains.txt"          # 列表绝对路径
```

---

## 验证配置

### 方法 1：检查 YAML 语法

```bash
# 使用 YAML 验证工具
python -m yaml config.yaml

# 或使用在线工具
# https://www.yamllint.com/
```

### 方法 2：启动时验证

```bash
# 启动应用，观察日志输出
./creskyDNS config.yaml

# 正常启动的日志输出：
# |2026-01-10|10:00:00,123|INFO|creskyDNS|main|DNS 转发器启动成功|
# |2026-01-10|10:00:00,156|INFO|creskyDNS|listener|监听器 'main' 端口: 5353|
```

### 方法 3：测试 DNS 查询

```bash
# Linux/macOS
nslookup google.com 127.0.0.1:5353
dig @127.0.0.1 -p 5353 google.com

# Windows
nslookup google.com 127.0.0.1
# 需要在命令行工具中修改 DNS 设置

# 使用 DNS 测试工具
# https://mxtoolbox.com/
# https://whatsmydns.net/
```

### 常见验证错误

| 错误 | 原因 | 解决方法 |
|------|------|---------|
| `Port already in use` | 端口被占用 | 更改 listener 端口或关闭占用端口的程序 |
| `File not found` | 列表文件不存在 | 检查 lists.path 路径是否正确 |
| `Invalid YAML` | YAML 格式错误 | 检查缩进和语法 |
| `Connection refused` | DNS 无法连接 | 检查 upstreams.addr 是否正确 |

---

## 常见问题

### Q1: 如何快速切换到生产环境配置？

**A**: 创建不同的配置文件，启动时指定：

```bash
# 开发环境
./creskyDNS config-dev.yaml

# 生产环境
./creskyDNS config-prod.yaml
```

### Q2: 如何实现 A/B 测试？

**A**: 创建两个不同的监听器：

```yaml
listener:
  test_a: 5353       # A 方案
  test_b: 5354       # B 方案

rules:
  servers:
    - test_a,dns_a
    - test_b,dns_b
```

### Q3: 配置修改后如何生效？

**A**: 目前需要重启应用。未来版本支持热重新加载。

### Q4: 如何调试配置问题？

**A**: 设置日志级别为 `debug`：

```yaml
log:
  level: "debug"
```

---

## 相关文档

- [PROJECT_FEATURES.md](PROJECT_FEATURES.md) - 项目功能说明
- [LOG_SYSTEM.md](LOG_SYSTEM.md) - 日志系统说明
- [QUICK_START.md](QUICK_START.md) - 快速开始
- [RULE_MATCHING.md](RULE_MATCHING.md) - 规则匹配详解
- [RULE_MATCHING_ADVANCED.md](RULE_MATCHING_ADVANCED.md) - 高级规则（Final 规则）
- [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) - 域名列表格式
- [IP_CIDR_LIST.md](IP_CIDR_LIST.md) - IP CIDR 列表说明

---

## 总结

本文档提供了：

✅ **最小化配置** - 3 行配置快速开始  
✅ **完整配置** - 包含所有功能的标准配置  
✅ **分场景配置** - 5 个实际场景示例  
✅ **高级配置** - 故障转移、地域分流等  
✅ **配置说明** - 完整的参考文档  

**选择合适的配置模板，快速开始您的 DNS 转发之旅！** 🚀
