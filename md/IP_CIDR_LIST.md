# 🌐 IP CIDR 列表 - 完整说明

## 📋 目录

- [概述](#概述)
- [列表格式](#列表格式)
- [配置说明](#配置说明)
- [使用场景](#使用场景)
- [规则匹配](#规则匹配)
- [配置示例](#配置示例)
- [高级用法](#高级用法)
- [性能优化](#性能优化)
- [常见问题](#常见问题)

---

## 概述

DNS 转发器现支持 **IP CIDR 列表**，可根据响应的 IP 地址进行分流和过滤。

### 主要特性

✅ **CIDR 段匹配** - 支持标准 IPv4/IPv6 CIDR 表示法  
✅ **国家代码标记** - 每条 IP 段附带地理位置信息  
✅ **灵活规则** - 支持基于 IP 的规则分流  
✅ **高性能** - 优化的 IP 段查询算法  
✅ **热重新加载** - 支持动态更新 IP 列表  

---

## 列表格式

### IP CIDR 文本格式

**格式**：每行一条记录，使用管道符（`|`）分隔

```
|CIDR段|国家代码|
```

### 字段说明

| 字段 | 说明 | 示例 |
|------|------|------|
| **CIDR** | IP 地址段（CIDR 表示法） | `8.8.8.0/24` |
| **国家代码** | ISO 3166-1 两字母国家代码 | `US`、`CN`、`JP` |

### 格式示例

```text
|8.8.8.0/24|US|
|1.1.1.0/24|AU|
|142.250.0.0/15|US|
|172.217.0.0/16|US|
|223.5.5.0/24|CN|
|101.226.0.0/14|CN|
|119.29.0.0/16|CN|
|2001:4860::/32|US|
|2606:2800:220::/48|US|
```

### 注意事项

⚠️ **严格格式要求**：
 ✅ 必须用 `|` 包起来
 ✅ CIDR 段必须是有效的 IPv4 或 IPv6 CIDR
 ✅ 国家代码必须是 ISO 3166-1 标准（2 字母）
 ✅ 每行一条记录；空行与以 `#` 开头的注释行会被忽略
 ✅ 支持行内注释：同一行中 `#` 之后内容将被忽略

### 有效的 CIDR 格式

#### IPv4

```
|0.0.0.0/0|XX|                    # 全网
|10.0.0.0/8|XX|                   # 整个 10.0.0.0 段
|192.168.0.0/16|XX|               # 整个 192.168.0.0 段
|203.0.113.0/24|XX|               # 单个 /24 段
|203.0.113.1/32|XX|               # 单个 IP（主机）
```

#### IPv6

```
|::/0|XX|                         # 全 IPv6 网络
|2001:db8::/32|XX|                # IPv6 段
|2606:2800:220::/48|XX|           # IPv6 段
|fe80::/10|XX|                    # link-local 地址
|::1/128|XX|                      # IPv6 本地回环
```

---

## 配置说明

### YAML 配置

```yaml
lists:
  ipcidr:
    type: "ipcidr"                # 列表类型：ipcidr
    format: "text"                # 文件格式：text
    path: "./lists/ipcidr.txt"    # 文件路径
    interval: 86400               # 重新加载倒计时（秒）
    description: "IP 地址段列表"  # 描述（可选）
```

### 配置字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| **type** | string | ✅ | 列表类型，固定为 `"ipcidr"` |
| **format** | string | ✅ | 文件格式，固定为 `"text"` |
| **path** | string | ✅ | IP CIDR 列表文件路径 |
| **interval** | integer | 否 | 重新加载倒计时（秒），0 = 立即 |
| **description** | string | 否 | 列表描述，用于日志和管理 |

### 完整配置示例

```yaml
# 多个 IP CIDR 列表
lists:
  # Google 地址段
  google_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/google_ips.txt"
    interval: 86400
    description: "Google 服务 IP 段"
  
  # 国内 CDN 地址段
  cn_cdn:
    type: "ipcidr"
    format: "text"
    path: "./lists/cn_cdn_ips.txt"
    interval: 3600
    description: "国内 CDN IP 段"
  
  # 国内运营商 IP 段
  cn_isp:
    type: "ipcidr"
    format: "text"
    path: "./lists/cn_isp_ips.txt"
    interval: 86400
    description: "国内运营商 IP 段"
  
  # 代理服务 IP 段（黑名单）
  proxy_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/proxy_ips.txt"
    interval: 3600
    description: "代理和 VPN IP 段"
```

---

## 使用场景

### 场景 1：地理位置分流

根据 DNS 响应的 IP 地址所属国家进行分流：

```yaml
lists:
  # 国内 IP 段
  cn_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_ips.txt"
    interval: 86400
  
  # 国际 IP 段
  global_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/global_ips.txt"
    interval: 86400

upstreams:
  cn_dns:
    addr: "https://dns.alidns.com/dns-query"
  
  global_dns:
    addr: "https://dns.google/dns-query"

rules:
  main:
    # 国内 IP → 国内 DNS
    - cn_ips,cn_dns
    # 国际 IP → 国际 DNS
    - global_ips,global_dns
```

### 场景 2：运营商分流

根据响应 IP 所属运营商选择不同的 DNS：

```yaml
lists:
  # 三大运营商 IP 段
  chinamobile:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_mobile_ips.txt"
    interval: 86400
  
  chinaunicom:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_unicom_ips.txt"
    interval: 86400
  
  chinatelecom:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_telecom_ips.txt"
    interval: 86400

upstreams:
  mobile_dns:
    addr: "https://dns.mobile.com/dns-query"
  
  unicom_dns:
    addr: "https://dns.unicom.com/dns-query"
  
  telecom_dns:
    addr: "https://dns.telecom.com/dns-query"

rules:
  main:
    - chinamobile,mobile_dns
    - chinaunicom,unicom_dns
    - chinatelecom,telecom_dns
```

### 场景 3：IP 黑名单过滤

拦截来自特定 IP 段的响应：

```yaml
lists:
  # 恶意 IP 段
  malicious_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/malicious_ips.txt"
    interval: 3600
  
  # 代理 IP 段
  proxy_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/proxy_ips.txt"
    interval: 3600

upstreams:
  clean_dns:
    addr: "https://dns.google/dns-query"
  
  blocked_dns:
    addr: "udp://127.0.0.1:1"      # 黑洞 DNS

rules:
  main:
    # 恶意 IP → 拦截
    - malicious_ips,blocked_dns
    # 代理 IP → 拦截
    - proxy_ips,blocked_dns
    # 其他 → 正常 DNS
    - .,clean_dns
```

### 场景 4：CDN 分流优化

根据 CDN 节点位置选择最优 DNS：

```yaml
lists:
  # Akamai CDN IP 段
  akamai_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/akamai_ips.txt"
    interval: 86400
  
  # Cloudflare CDN IP 段
  cloudflare_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/cloudflare_ips.txt"
    interval: 86400
  
  # 国内 CDN IP 段
  cn_cdn_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/cn_cdn_ips.txt"
    interval: 86400

upstreams:
  akamai_dns:
    addr: "https://dns.akamai.com"
  
  cloudflare_dns:
    addr: "https://dns.cloudflare.com"
  
  cn_dns:
    addr: "https://dns.alidns.com"

rules:
  main:
    - akamai_ips,akamai_dns
    - cloudflare_ips,cloudflare_dns
    - cn_cdn_ips,cn_dns
    - .,cn_dns
```

---

## 规则匹配

### 匹配原理

**IP CIDR 规则匹配流程**：

1. **获取 DNS 响应的 IP 地址**
   ```
   查询: www.google.com (A)
   响应: 142.250.185.68
   ```

2. **检查 IP 所属的 CIDR 段**
   ```
   142.250.185.68 ∈ 142.250.0.0/15 ?
   ✅ 匹配
   ```

3. **返回匹配的规则**
   ```
   应用该规则的上游 DNS 配置
   ```

### 匹配优先级

**多个 IP CIDR 列表时的匹配顺序**：

1. **规则组顺序**：按 rules 中定义的顺序
2. **组内深度优先**：网络掩码更长（更具体）的优先
3. **第一个匹配**：返回第一个匹配的规则

**示例**：
```yaml
lists:
  # 网络 1
  net1:
    path: "./lists/list1.txt"
    # |142.250.0.0/15|US|
  
  # 网络 2（子网）
  net2:
    path: "./lists/list2.txt"
    # |142.250.180.0/24|US|

rules:
  main:
    - net1,dns_a
    - net2,dns_b

# 查询响应: 142.250.185.68
# 匹配结果: net2 (因为 /24 比 /15 更具体)
# 使用: dns_b
```

---

## 配置示例

### 完整示例：国内外分流 + IP 分流

```yaml
# config-ip-routing.yaml

listener:
  main: 5353

upstreams:
  # 国内 DNS
  cn_dns:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"
  
  # 国际 DNS
  global_dns:
    addr: "https://dns.google/dns-query"
    cache: "main"
  
  # 黑洞 DNS（拦截）
  blocked_dns:
    addr: "udp://127.0.0.1:1"
    cache: "main"

lists:
  # 域名列表：国内域名
  cn_domains:
    type: "domain"
    format: "text"
    path: "./lists/china_domains.txt"
    interval: 3600
    description: "国内域名"
  
  # 域名列表：国际域名
  global_domains:
    type: "domain"
    format: "text"
    path: "./lists/global_domains.txt"
    interval: 3600
    description: "国际域名"
  
  # IP 列表：国内 IP
  cn_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_ips.txt"
    interval: 86400
    description: "国内 IP 地址段"
  
  # IP 列表：国际 IP
  global_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/global_ips.txt"
    interval: 86400
    description: "国际 IP 地址段"
  
  # IP 列表：代理 IP（黑名单）
  proxy_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/proxy_ips.txt"
    interval: 3600
    description: "代理和 VPN IP"

rules:
  main:
    # 域名级别的分流
    - cn_domains,cn_dns
    - global_domains,global_dns
    
    # IP 级别的分流
    - cn_ips,cn_dns
    - global_ips,global_dns
    
    # 黑名单过滤
    - proxy_ips,blocked_dns
    
    # 默认规则
    - .,cn_dns

cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400

log:
  enabled: true
  path: "./logs/ip_routing.log"
  level: "info"
  max_size: 100MB
  max_backups: 14
```

---

## 高级用法

### IP CIDR 文件生成

#### 方法 1：使用 Python 生成

```python
#!/usr/bin/env python3
# 从 IP 列表生成 CIDR 文件

import ipaddress

def generate_cidr_file(ip_list, country_code, output_file):
    """
    从 IP 地址列表生成 CIDR 文件
    
    Args:
        ip_list: IP 地址列表
        country_code: 国家代码
        output_file: 输出文件路径
    """
    ips = [ipaddress.ip_address(ip) for ip in ip_list]
    ips.sort()
    
    # 合并相邻的 IP 为 CIDR
    subnets = list(ipaddress.collapse_addresses(
        [ipaddress.ip_network(f"{ip}/32", strict=False) for ip in ips]
    ))
    
    with open(output_file, 'w') as f:
        for subnet in subnets:
            f.write(f"|{subnet}|{country_code}|\n")
    
    print(f"生成 {output_file}，共 {len(subnets)} 个 CIDR 段")

# 使用示例
google_ips = [
    "8.8.8.0",
    "8.8.9.0",
    "142.250.0.0",
    "142.250.1.0",
]

generate_cidr_file(google_ips, "US", "./lists/google_ips.txt")
```
|8.8.8.0/24|US|        # Google 公网段
|223.5.5.0/24|CN|      # 阿里公网段
# 这是注释行，将被忽略
|39.156.0.0/16|CN|     # 百度大段

#### 方法 2：从公开数据源

**获取国内 IP 地址段**：
- MaxMind GeoIP2
- IP2Location
- 国内 CDN 提供商的 IP 列表

**获取国家 IP 地址段**：
- APNIC RIR（亚太地区）
- RIPE NCC（欧洲）
- ARIN（北美）

### IPv4 和 IPv6 混合

```text
# 同一文件可以同时包含 IPv4 和 IPv6
|8.8.8.0/24|US|
|8.8.9.0/24|US|
|2001:4860:4860::/48|US|
|2001:4860:8888::/48|US|
```

### 大规模 IP 列表（百万级）

**优化建议**：

1. **分割列表**
   ```yaml
   lists:
     cn_ips_part1:
       type: "ipcidr"
       path: "./lists/cn_ips_part1.txt"
     cn_ips_part2:
       type: "ipcidr"
       path: "./lists/cn_ips_part2.txt"
   ```

2. **压缩存储**
   ```bash
   # 压缩列表文件
   gzip china_ips.txt
   # 应用可以直接读取压缩文件（如果支持）
   ```

3. **定期更新**
   ```yaml
   lists:
     cn_ips:
       type: "ipcidr"
       path: "./lists/cn_ips.txt"
       interval: 86400  # 每天更新
   ```

---

## 性能优化

### IP 查询性能

| 数据结构 | 查询时间 | 空间占用 | 说明 |
|---------|---------|---------|------|
| 链表 | O(n) | 低 | 不推荐 |
| **哈希表** | **O(1)** | 中 | 推荐 |
| **前缀树** | **O(k)** | 高 | 推荐（k=地址长度） |
| **二叉查找树** | O(log n) | 中 | 可选 |

### 推荐实现

使用 **前缀树（Trie）** 或 **哈希表** 存储 IP CIDR：

```rust
// Rust 伪代码示例
struct IPCIDRList {
    // 使用前缀树结构
    ipv4_trie: IpTrieNode,
    ipv6_trie: IpTrieNode,
}

impl IPCIDRList {
    fn contains(&self, ip: IpAddr) -> Option<String> {
        // 查询时间: O(32) for IPv4, O(128) for IPv6
        match ip {
            IpAddr::V4(v4) => self.ipv4_trie.search(v4),
            IpAddr::V6(v6) => self.ipv6_trie.search(v6),
        }
    }
}
```

### 缓存建议

```yaml
cache:
  main:
    size: 100000           # 增加缓存大小
    min_ttl: 300           # 较长的最小 TTL
    max_ttl: 86400         # 较长的最大 TTL
```

---

## 常见问题

### Q1: IP CIDR 列表和域名列表的区别？

**A**:

| 特性 | 域名列表 | IP CIDR 列表 |
|------|---------|-----------|
| 匹配对象 | 查询的域名 | DNS 响应的 IP |
| 文件格式 | 文本域名 | `\|CIDR\|国家代码\|` |
| 应用时机 | 查询前 | 获得响应后 |
| 典型用途 | 域名分流、过滤 | IP 分流、地理定位 |

**示例**：
```yaml
# 域名列表：根据域名决定使用哪个 DNS
- domain_list,dns_a

# IP CIDR 列表：根据 DNS 响应的 IP 决定后续处理
- ip_list,dns_b
```

### Q2: 如何验证 CIDR 格式是否正确？

**A**: 使用 Python 验证：

```python
import ipaddress

def validate_cidr(cidr_str):
    try:
        ipaddress.ip_network(cidr_str, strict=False)
        return True
    except ValueError:
        return False

# 测试
print(validate_cidr("8.8.8.0/24"))      # True
print(validate_cidr("8.8.8.0/33"))      # False (掩码无效)
print(validate_cidr("999.999.999.999/24"))  # False (IP 无效)
```

### Q3: 如何在 IP CIDR 列表中添加注释？

**A**: 目前 IP CIDR 列表 **不支持注释**。如需添加说明，使用单独的文档：

```yaml
# config.yaml
lists:
  # 此列表用于地理位置分流
  # 数据来源: MaxMind GeoIP2 (更新于 2026-01-10)
  # 包含约 10000 个 CIDR 段
  cn_ips:
    type: "ipcidr"
    path: "./lists/cn_ips.txt"
    description: "国内 IP 地址段（地理位置分流）"
```

### Q4: IPv4 和 IPv6 如何同时支持？

**A**: 在同一个列表文件中混合使用：

```text
|8.8.8.0/24|US|
|2001:4860:4860::/48|US|
|1.1.1.0/24|AU|
|2606:4700:4700::/48|AU|
```

应用程序会自动识别 IPv4 和 IPv6。

### Q5: IP CIDR 列表如何实现热重新加载？

**A**: 和域名列表相同，使用 `interval` 字段：

```yaml
lists:
  cn_ips:
    type: "ipcidr"
    path: "./lists/cn_ips.txt"
    interval: 0           # 立即重新加载
    # 或
    interval: 86400       # 每天倒计时重新加载
```

### Q6: 如何处理重叠的 CIDR 段？

**A**: 使用更具体的（掩码更长）的 CIDR 段：

```yaml
lists:
  net_large:
    path: "./lists/large_net.txt"
    # |10.0.0.0/8|XX|

  net_small:
    path: "./lists/small_net.txt"
    # |10.1.0.0/16|YY|  (更具体，掩码更长)

rules:
  main:
    - net_large,dns_a
    - net_small,dns_b    # 优先匹配

# 查询响应: 10.1.1.1
# 匹配结果: net_small (因为 /16 比 /8 更具体)
# 使用: dns_b
```

### Q7: 如何性能监控 IP 查询速度？

**A**: 启用调试日志：

```yaml
log:
  level: "debug"          # 详细日志
```

日志示例：
```log
|2026-01-10|10:00:00,123|DEBUG|creskyDNS|ip_matcher|IP 查询: 142.250.185.68 → 匹配 cn_ips (耗时: 0.05ms)|
```

---

## 相关文档

- [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) - 域名列表格式
- [RULE_MATCHING.md](RULE_MATCHING.md) - 规则匹配详解
- [CONFIG_EXAMPLES.md](CONFIG_EXAMPLES.md) - 配置文件示例
- [PROJECT_FEATURES.md](PROJECT_FEATURES.md) - 项目功能说明

---

## 总结

IP CIDR 列表提供了：

✅ **IP 层级的分流** - 根据 DNS 响应的 IP 地址分流  
✅ **地理位置识别** - 支持国家代码标记  
✅ **灵活的规则配置** - 支持复杂的 IP 分流策略  
✅ **高性能匹配** - 优化的 CIDR 查询算法  
✅ **热重新加载** - 动态更新 IP 列表  

**结合域名列表和 IP CIDR 列表，可以实现强大的多维度 DNS 分流能力！** 🚀
