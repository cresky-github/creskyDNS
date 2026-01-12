# Bootstrap DNS 协议支持总结

## 📊 支持状态一览表

| 协议 | Bootstrap 支持 | 实现位置 | 需要原因 |
|------|---------------|----------|----------|
| UDP | ❌ 不需要 | - | 直接使用 IP 地址 |
| TCP | ❌ 不需要 | - | 直接使用 IP 地址 |
| **DoH** | ✅ 已支持 | forwarder.rs:836-920 | HTTPS URL 使用域名 |
| **DoT** | ✅ 已支持 | forwarder.rs:689-780 | TLS 连接使用域名 |
| **DoQ** | ✅ 已支持 | forwarder.rs:947-1025 | QUIC 连接使用域名 |
| **H3** | ✅ 已支持 | forwarder.rs:1066-1150 | HTTP/3 URL 使用域名 |

## 🔍 各协议实现对比

### DoH (DNS over HTTPS)
- **协议**: HTTPS (TCP + TLS)
- **默认端口**: 443
- **URL 格式**: `https://dns.google/dns-query`
- **Bootstrap 实现**:
  1. 解析域名 `dns.google` → IP `8.8.8.8`
  2. 替换 URL → `https://8.8.8.8/dns-query`
  3. 设置 `Host: dns.google` header
  4. 接受无效证书（因为使用 IP）
- **关键特性**: 需要处理 HTTP header 和证书验证

### DoT (DNS over TLS)
- **协议**: TLS over TCP
- **默认端口**: 853
- **URL 格式**: `tls://dns.google:853`
- **Bootstrap 实现**:
  1. 解析域名 `dns.google` → IP `8.8.8.8`
  2. TCP 连接到 `8.8.8.8:853`
  3. TLS 握手时使用 SNI: `dns.google`
- **关键特性**: 需要正确设置 SNI
- **额外支持**: SOCKS5 代理

### DoQ (DNS over QUIC)
- **协议**: QUIC (基于 UDP)
- **默认端口**: 784
- **URL 格式**: `doq://dns.google:784` 或 `quic://dns.google:784`
- **Bootstrap 实现**:
  1. 解析域名 `dns.google` → IP `8.8.8.8`
  2. QUIC 连接到 `8.8.8.8:784`
  3. 使用原始域名作为 SNI
- **关键特性**: 基于 UDP，不支持 SOCKS5 代理

### H3 (DNS over HTTP/3)
- **协议**: HTTP/3 (QUIC + HTTP)
- **默认端口**: 443
- **URL 格式**: `h3://dns.google/dns-query` 或 `https3://dns.google/dns-query`
- **Bootstrap 实现**:
  1. 解析域名 `dns.google` → IP `8.8.8.8`
  2. 替换 URL → `https://8.8.8.8/dns-query`
  3. 设置 `Host: dns.google` header
  4. 接受无效证书（因为使用 IP）
- **关键特性**: 类似 DoH，但使用 HTTP/3 协议

## 🎯 为什么需要 Bootstrap

### 循环依赖问题
```
┌─────────────────────────────────────────┐
│ 用户想要解析 example.com                 │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│ DNS 服务器地址: dns.google              │
│ 问题：需要 DNS 来解析这个域名！          │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│ 使用 Bootstrap DNS (8.8.8.8)            │
│ 解析 dns.google → 获得 IP               │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│ 使用 IP 连接到 DNS 服务器                │
│ 查询 example.com                        │
└─────────────────────────────────────────┘
```

### 使用场景
1. **首次启动**: 系统 DNS 可能不可用或不可信
2. **DNS 污染**: 避免系统 DNS 返回错误的 DNS 服务器 IP
3. **隐私保护**: 不让 ISP 知道你在使用哪个 DNS 服务器
4. **可靠性**: Bootstrap 使用简单的 UDP DNS，更可靠

## 📝 配置示例

### 基础配置
```yaml
upstreams:
  google_doh:
    addr:
      - "https://dns.google/dns-query"
    bootstrap:
      - "udp://8.8.8.8:53"    # Google 公共 DNS
```

### 完整配置（多协议）
```yaml
upstreams:
  # DoH - DNS over HTTPS
  cloudflare_doh:
    addr:
      - "https://cloudflare-dns.com/dns-query"
    bootstrap:
      - "udp://1.1.1.1:53"
      - "udp://1.0.0.1:53"
    cache: "domain"

  # DoT - DNS over TLS
  google_dot:
    addr:
      - "tls://dns.google:853"
    bootstrap:
      - "udp://8.8.8.8:53"
      - "udp://8.8.4.4:53"
    proxy: "socks5://127.0.0.1:1080"  # 可选代理

  # DoQ - DNS over QUIC
  adguard_doq:
    addr:
      - "doq://dns.adguard.com:784"
    bootstrap:
      - "udp://94.140.14.14:53"

  # H3 - DNS over HTTP/3
  cloudflare_h3:
    addr:
      - "h3://cloudflare-dns.com/dns-query"
    bootstrap:
      - "udp://1.1.1.1:53"
```

### 中国区推荐配置
```yaml
upstreams:
  # 阿里云 DoH
  ali_doh:
    addr:
      - "https://dns.alidns.com/dns-query"
    bootstrap:
      - "udp://223.5.5.5:53"
      - "udp://223.6.6.6:53"

  # 腾讯云 DoH
  dnspod_doh:
    addr:
      - "https://doh.pub/dns-query"
    bootstrap:
      - "udp://119.29.29.29:53"

  # 360 安全 DNS DoH
  360_doh:
    addr:
      - "https://doh.360.cn/dns-query"
    bootstrap:
      - "udp://101.226.4.6:53"
```

## ⚠️ 注意事项

### 1. Bootstrap 服务器选择
- ✅ **使用 UDP 协议**: `udp://8.8.8.8:53`
- ✅ **使用 IP 地址**: 不要用域名
- ❌ **避免加密协议**: 不要使用 DoH/DoT 作为 bootstrap（会循环依赖）
- ✅ **配置多个备份**: 提高可靠性

### 2. 性能考虑
- **首次延迟**: Bootstrap 解析会增加约 50-200ms 延迟
- **缓存优化**: 系统会缓存 DNS 服务器 IP，后续查询不受影响
- **建议**: 如果 DNS 服务器 IP 固定，可直接使用 IP 地址

### 3. 推荐的 Bootstrap 服务器

#### 国际
- Google: `8.8.8.8`, `8.8.4.4`
- Cloudflare: `1.1.1.1`, `1.0.0.1`
- Quad9: `9.9.9.9`, `149.112.112.112`

#### 中国
- 阿里云: `223.5.5.5`, `223.6.6.6`
- 腾讯云: `119.29.29.29`, `182.254.116.116`
- 百度: `180.76.76.76`
- 114 DNS: `114.114.114.114`, `114.114.115.115`

## 🔧 实现细节

### 核心方法
```rust
// src/forwarder.rs
async fn resolve_with_bootstrap(
    &self, 
    domain: &str, 
    bootstrap_servers: &[String]
) -> Result<Vec<String>>
```

### 调用流程
```
forward_to_upstream_list()
    ↓
检测协议类型
    ↓
match protocol:
    - DoH → forward_doh(bootstrap)
    - DoT → forward_dot(bootstrap)
    - DoQ → forward_doq(bootstrap)
    - H3  → forward_h3(bootstrap)
    ↓
resolve_with_bootstrap()
    ↓
使用 UDP DNS 查询
    ↓
返回 IP 列表
```

### 错误处理
1. **Bootstrap 解析失败**: 回退到系统 DNS
2. **超时处理**: 配置的 `timeout_secs` 应用于所有操作
3. **日志记录**: 
   - `debug!()`: 正常流程
   - `warn!()`: Bootstrap 失败但有回退

## ✅ 测试验证

### 手动测试命令
```bash
# 测试 DoH with bootstrap
dig @127.0.0.1 -p 5353 example.com

# 查看日志确认 bootstrap 被使用
# 应该看到类似：
# DEBUG DoH 使用 bootstrap 解析: dns.google -> 8.8.8.8
```

### 验证清单
- [x] DoH 协议使用 bootstrap 解析
- [x] DoT 协议使用 bootstrap 解析
- [x] DoQ 协议使用 bootstrap 解析
- [x] H3 协议使用 bootstrap 解析
- [x] Bootstrap 失败时正确回退
- [x] 日志正确记录 bootstrap 使用
- [x] 配置文件示例完整
- [x] GitHub Actions 编译通过

## 🎉 总结

**Bootstrap DNS 功能已完整实现**，覆盖所有需要域名解析的加密 DNS 协议：

- ✅ **DoH (DNS over HTTPS)**: 完整支持
- ✅ **DoT (DNS over TLS)**: 完整支持，含 SOCKS5 代理
- ✅ **DoQ (DNS over QUIC)**: 完整支持
- ✅ **H3 (DNS over HTTP/3)**: 完整支持

**实现质量**:
- ✅ 健壮的错误处理
- ✅ 完善的日志记录
- ✅ 自动回退机制
- ✅ 详细的配置文档

**状态**: 🚀 生产就绪，可以正常使用！
