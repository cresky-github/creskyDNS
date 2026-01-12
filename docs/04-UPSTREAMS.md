# 04 - 上游服务器模块

## 📋 目录

- [概述](#概述)
- [配置说明](#配置说明)
- [协议支持](#协议支持)
- [上游配置详解](#上游配置详解)
- [默认上游](#默认上游)
- [DoH 支持](#doh-支持)
- [配置示例](#配置示例)
- [故障排查](#故障排查)
- [最佳实践](#最佳实践)

---

## 概述

上游 DNS 服务器是 creskyDNS 转发 DNS 查询的目标服务器。系统支持多种协议和多个上游配置，可以根据不同的规则选择不同的上游。

### 核心特性

✅ **多协议支持**：UDP / TCP / DoH / DoT / DoQ / H3  
✅ **多上游配置**：同时配置多个上游服务器  
✅ **智能降级**：未匹配规则时自动使用默认上游  
✅ **Bootstrap DNS**：DoH 初始化时使用 bootstrap  
✅ **超时控制**：可配置超时和重试  
✅ **缓存绑定**：每个上游可指定缓存配置

---

## 配置说明

### 基本配置格式

```yaml
upstreams:
  上游名称:
    addr: "协议://地址:端口"
    bootstrap: "udp://IP:53"          # DoH 需要
    cache: "缓存名称"
    timeout: 5000                      # 毫秒
    retry: 2
```

### 配置字段详解

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| **addr** | string | ✅ | 无 | DNS 服务器地址（含协议） |
| **addresses** | array | ✅ | 无 | DNS 服务器地址列表（多个地址） |
| **bootstrap** | string | 否 | 无 | DoH 初始化用的 bootstrap DNS |
| **cache** | string | 否 | 无 | 使用的缓存配置名称 |
| **timeout** | integer | 否 | 5000 | 请求超时时间（毫秒） |
| **retry** | integer | 否 | 2 | 重试次数 |

**注意**：`addr` 和 `addresses` 二选一，不能同时使用。

---

## 协议支持

### 支持的协议

| 协议 | 说明 | 地址格式 | 示例 |
|------|------|----------|------|
| **UDP** | 标准 DNS 协议 | `udp://IP:PORT` | `udp://8.8.8.8:53` |
| **TCP** | TCP 传输 | `tcp://IP:PORT` | `tcp://8.8.8.8:53` |
| **DoH** | DNS over HTTPS | `https://URL/path` | `https://dns.google/dns-query` |
| **DoT** | DNS over TLS | `tls://HOST:PORT` | `tls://dns.google:853` |
| **DoQ** | DNS over QUIC | `quic://HOST:PORT` | `quic://dns.adguard.com:784` |
| **H3** | HTTP/3 | `h3://HOST:PORT` | `h3://dns.google:443` |

### 协议特点对比

| 协议 | 加密 | 性能 | 延迟 | 防劫持 | 适用场景 |
|------|------|------|------|--------|----------|
| **UDP** | ❌ | 🟢 最快 | 最低 | ❌ | 局域网、可信网络 |
| **TCP** | ❌ | 🟡 较快 | 较低 | ❌ | 大响应、可信网络 |
| **DoH** | ✅ | 🟡 较快 | 中等 | ✅ | 公网、隐私保护 |
| **DoT** | ✅ | 🟢 快 | 较低 | ✅ | 公网、高性能需求 |
| **DoQ** | ✅ | 🟢 快 | 低 | ✅ | 公网、低延迟需求 |
| **H3** | ✅ | 🟢 快 | 低 | ✅ | 现代网络环境 |

---

## 上游配置详解

### 单地址配置

```yaml
upstreams:
  google:
    addr: "https://dns.google/dns-query"
    cache: "main"
    timeout: 5000
```

### 多地址配置

```yaml
upstreams:
  ali:
    addresses:
      - "https://dns.alidns.com/dns-query"
      - "udp://223.5.5.5:53"
      - "udp://223.6.6.6:53"
    cache: "main"
    timeout: 5000
```

**多地址说明**：
- 系统会按顺序尝试每个地址
- 如果第一个失败，自动尝试下一个
- 提高可用性和容错性

### DoH 配置（需要 bootstrap）

```yaml
upstreams:
  google_doh:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"     # 用于解析 dns.google
    cache: "main"
    timeout: 5000
```

**bootstrap 说明**：
- DoH 需要先解析域名（如 `dns.google`）
- bootstrap 提供初始 DNS 解析能力
- 避免循环依赖问题

### 完整配置示例

```yaml
upstreams:
  # Google DNS（DoH）
  google:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "main"
    timeout: 5000
    retry: 2
  
  # Cloudflare DNS（DoH）
  cloudflare:
    addr: "https://cloudflare-dns.com/dns-query"
    bootstrap: "udp://1.1.1.1:53"
    cache: "main"
    timeout: 5000
    retry: 2
  
  # 阿里 DNS（DoH + UDP 备用）
  ali:
    addresses:
      - "https://dns.alidns.com/dns-query"
      - "udp://223.5.5.5:53"
      - "udp://223.6.6.6:53"
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
    timeout: 5000
  
  # 114 DNS（UDP）
  dns114:
    addr: "udp://114.114.114.114:53"
    cache: "main"
    timeout: 3000
  
  # 本地 DNS（UDP）
  local:
    addr: "udp://192.168.1.1:53"
    cache: "local"
    timeout: 2000
  
  # 黑洞 DNS（拦截广告）
  blocked:
    addr: "udp://127.0.0.1:1"
    cache: "main"
    timeout: 100
```

---

## 默认上游

### 自动降级机制

当域名未匹配到任何规则时，系统会自动使用默认上游。

### 优先级顺序

系统按以下顺序查找默认上游：

1. **default_dns** (如果配置) - 最高优先级
2. **cn_dns** (如果配置) - 国内 DNS
3. **direct_dns** (如果配置) - 直连 DNS
4. **global_dns** (如果配置) - 国际 DNS
5. **第一个可用上游** - 兜底方案
6. **报错** - 只有完全没有上游时才报错

### 配置示例

#### 推荐配置（明确定义 default_dns）

```yaml
upstreams:
  # 明确定义默认上游
  default_dns:
    addresses:
      - "udp://223.5.5.5:53"
      - "udp://119.29.29.29:53"
    cache: "main"
  
  cn_dns:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"
  
  global_dns:
    addr: "https://dns.google/dns-query"
    cache: "main"

rules:
  main:
    - china_domains,cn_dns
    - global_domains,global_dns
  
  # 未匹配的域名会自动使用 default_dns
```

#### 简化配置（自动降级）

```yaml
upstreams:
  cn_dns:
    addr: "udp://223.5.5.5:53"
    cache: "main"
  
  proxy_dns:
    addr: "udp://1.1.1.1:53"
    cache: "main"

rules:
  main:
    - direct,cn_dns
    - proxy,proxy_dns

# 未匹配的域名会自动使用 cn_dns（第一个上游）
```

### 日志示例

#### 匹配到规则
```log
|2026-01-12|10:30:45,123|DEBUG|creskyDNS|dns_resolver|域名 google.com 在规则组 'main' 中匹配到上游 'global_dns'|
```

#### 使用默认上游
```log
|2026-01-12|10:30:45,234|DEBUG|creskyDNS|dns_resolver|域名 jd.com 未匹配任何规则，使用默认上游 'default_dns'|
```

#### 使用降级上游
```log
|2026-01-12|10:30:45,345|DEBUG|creskyDNS|dns_resolver|域名 taobao.com 未匹配任何规则，使用默认上游 'cn_dns'|
```

#### 使用兜底上游
```log
|2026-01-12|10:30:45,456|DEBUG|creskyDNS|dns_resolver|域名 example.com 未匹配任何规则，使用第一个可用上游 'cn_dns'|
```

---

## DoH 支持

### DoH 功能特性

✅ **RFC 8484 标准**：完全符合 DNS over HTTPS 标准  
✅ **HTTPS 加密**：使用 rustls-tls 加密传输  
✅ **隐私保护**：防止 ISP 监听 DNS 查询  
✅ **防劫持**：直连权威 DoH 服务器  
✅ **高性能**：异步非阻塞实现

### DoH 工作流程

```
客户端 DNS 请求 (google.com A 记录)
         ↓
creskyDNS 接收 (UDP/TCP 53端口)
         ↓
规则匹配 → 确定使用 google_doh 上游
         ↓
parse_protocol("https://dns.google/dns-query")
         ↓ 
识别为 Protocol::Doh
         ↓
调用 forward_doh()
         ↓
┌─────────────────────────────────┐
│ 1. DNS Message → 二进制编码      │
│ 2. Base64 URL-safe 编码          │
│ 3. 构建 HTTP GET 请求            │
│    URL: https://dns.google/dns-query?dns=<base64> │
│ 4. 添加 Header:                  │
│    Accept: application/dns-message │
│ 5. 通过 HTTPS 发送请求           │
└─────────────────────────────────┘
         ↓
Google DoH 服务器处理
         ↓
┌─────────────────────────────────┐
│ HTTP 200 OK                     │
│ Content-Type: application/dns-message │
│ Body: <二进制 DNS 响应>          │
└─────────────────────────────────┘
         ↓
解析响应
         ↓
存入缓存 (TTL 管理)
         ↓
返回给客户端
```

### 主流 DoH 服务商

| 提供商 | URL | 特点 |
|--------|-----|------|
| **Google Public DNS** | `https://dns.google/dns-query` | 全球可用、高性能 |
| **Cloudflare** | `https://cloudflare-dns.com/dns-query` | 隐私优先、快速 |
| **阿里云 DNS** | `https://dns.alidns.com/dns-query` | 国内优化 |
| **Quad9** | `https://dns.quad9.net/dns-query` | 安全过滤 |
| **AdGuard** | `https://dns.adguard.com/dns-query` | 广告拦截 |

### DoH 配置示例

```yaml
upstreams:
  # Google DoH
  google_doh:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "main"
    timeout: 5000
  
  # Cloudflare DoH
  cloudflare_doh:
    addr: "https://cloudflare-dns.com/dns-query"
    bootstrap: "udp://1.1.1.1:53"
    cache: "main"
    timeout: 5000
  
  # 阿里云 DoH
  ali_doh:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
    timeout: 5000

rules:
  main:
    - china_domains,ali_doh
    - global_domains,google_doh
    - .,cloudflare_doh
```

### DoH 性能优化

**预期性能**：
- 首次查询（无缓存）: 50-200ms
- 缓存命中: < 1ms
- 规则缓存命中 + DNS 缓存未命中: 50-200ms

**优化建议**：
1. 启用缓存（cache: "main"）
2. 选择地理位置近的 DoH 服务器
3. 配置合理的超时时间（5000ms）
4. 配置多个地址作为备用

---

## 配置示例

### 示例 1：国内外分流

```yaml
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

rules:
  main:
    - cn_domains,cn_dns
    - global_domains,global_dns
```

### 示例 2：多级降级

```yaml
upstreams:
  # 主上游（DoH）
  primary:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "main"
    timeout: 5000
  
  # 备用上游 1（UDP）
  backup1:
    addr: "udp://8.8.8.8:53"
    cache: "main"
    timeout: 3000
  
  # 备用上游 2（UDP）
  backup2:
    addr: "udp://1.1.1.1:53"
    cache: "main"
    timeout: 3000

rules:
  main:
    - .,primary
  
  # 如果 primary 失败，系统会尝试其他上游
```

### 示例 3：多地址容错

```yaml
upstreams:
  ali:
    addresses:
      - "https://dns.alidns.com/dns-query"  # 优先使用 DoH
      - "udp://223.5.5.5:53"                # 备用 UDP
      - "udp://223.6.6.6:53"                # 备用 UDP
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
    timeout: 5000
    retry: 2
```

### 示例 4：企业内网

```yaml
upstreams:
  # 内网 DNS
  internal_dns:
    addr: "udp://192.168.1.1:53"
    cache: "local"
    timeout: 2000
  
  # 外网 DNS（DoH）
  external_dns:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "main"
    timeout: 5000

lists:
  internal_domains:
    type: "domain"
    domains:
      - company.com
      - internal.local

rules:
  main:
    - internal_domains,internal_dns
    - .,external_dns
```

---

## 故障排查

### 问题 1：上游连接超时

**错误信息**：
```
ERROR: 上游 DNS 连接失败: timeout after 5s
```

**原因**：
- 网络不通
- 防火墙阻止
- 上游服务器不可用

**解决方案**：
1. 测试网络连通性：
   ```bash
   # 测试 UDP
   nslookup google.com 8.8.8.8
   
   # 测试 DoH
   curl -H "Accept: application/dns-message" \
        "https://dns.google/dns-query?dns=AAABAAABAAAAAAAAA3d3dwZnb29nbGUDY29tAAABAAE"
   ```

2. 检查防火墙：
   ```bash
   # 允许 DNS (UDP 53)
   sudo iptables -A OUTPUT -p udp --dport 53 -j ACCEPT
   
   # 允许 HTTPS (TCP 443)
   sudo iptables -A OUTPUT -p tcp --dport 443 -j ACCEPT
   ```

3. 更换上游或增加超时：
   ```yaml
   upstreams:
     google:
       addr: "https://dns.google/dns-query"
       timeout: 10000  # 增加超时到 10 秒
   ```

### 问题 2：DoH 解析失败

**错误信息**：
```
ERROR: DoH 请求失败: HTTP 400 Bad Request
```

**原因**：
- Bootstrap DNS 不可用
- 域名解析失败
- 请求格式错误

**解决方案**：
1. 检查 bootstrap 配置：
   ```yaml
   upstreams:
     google:
       addr: "https://dns.google/dns-query"
       bootstrap: "udp://8.8.8.8:53"  # 确保 bootstrap 可用
   ```

2. 测试 bootstrap DNS：
   ```bash
   nslookup dns.google 8.8.8.8
   ```

3. 手动测试 DoH：
   ```bash
   curl -v -H "Accept: application/dns-message" \
        "https://dns.google/dns-query?dns=AAABAAABAAAAAAAAA3d3dwZnb29nbGUDY29tAAABAAE"
   ```

### 问题 3：证书验证失败

**错误信息**：
```
ERROR: TLS 错误: certificate verify failed
```

**原因**：
- 系统时间不正确
- 证书过期
- 缺少根证书

**解决方案**：
1. 检查系统时间：
   ```bash
   date
   # 如果时间不对，同步时间
   sudo ntpdate -s time.nist.gov
   ```

2. 更新证书库（Linux）：
   ```bash
   sudo update-ca-certificates
   ```

3. 更新证书库（Windows）：
   ```powershell
   certutil -generateSSTFromWU roots.sst
   ```

### 问题 4：上游不响应

**检查步骤**：

1. 查看日志：
   ```bash
   grep "上游" logs/creskyDNS.log
   ```

2. 测试上游可用性：
   ```bash
   # UDP 测试
   dig @8.8.8.8 google.com
   
   # DoH 测试
   curl https://dns.google/dns-query?dns=AAABAAABAAAAAAAAA3d3dwZnb29nbGUDY29tAAABAAE
   ```

3. 检查配置：
   ```yaml
   upstreams:
     google:
       addr: "https://dns.google/dns-query"  # 确认地址正确
       timeout: 5000                          # 确认超时合理
   ```

---

## 最佳实践

### 1. 协议选择

✅ **推荐**：
- 公网环境：优先使用 DoH / DoT
- 内网环境：使用 UDP / TCP
- 隐私敏感：使用 DoH
- 低延迟需求：使用 DoQ / H3

❌ **不推荐**：
- 公网使用明文 UDP（易被劫持）
- 内网使用 DoH（增加延迟）

### 2. 上游配置

✅ **推荐**：
- 配置多个地址作为备用
- 设置合理的超时时间（3-5秒）
- 为每个上游指定缓存
- 配置明确的 default_dns

❌ **不推荐**：
- 只配置单一上游（无容错）
- 超时过长（> 10秒）
- 所有上游使用同一缓存

### 3. 地理位置优化

✅ **推荐**：
- 国内用户：阿里云 DNS、腾讯 DNS
- 国际用户：Google DNS、Cloudflare
- 企业用户：内网 DNS + 公网 DNS

### 4. 安全配置

✅ **推荐**：
- 使用加密协议（DoH/DoT）
- 验证 bootstrap DNS 可靠性
- 定期更新上游配置
- 监控上游可用性

---

## 相关文档

- [01-LOG.md](01-LOG.md) - 日志模块
- [02-LISTENER.md](02-LISTENER.md) - 监听器模块
- [03-CACHE.md](03-CACHE.md) - 缓存模块
- [05-LISTS.md](05-LISTS.md) - 列表模块
- [06-RULES.md](06-RULES.md) - 规则模块
