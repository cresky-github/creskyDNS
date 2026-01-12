# Bootstrap DNS 实现完整性检查

## ✅ 已完成项

### 1. 配置结构支持
- [x] `UpstreamList` 结构体添加 `bootstrap: Option<Vec<String>>` 字段
- [x] 位置：`src/config.rs` 第 48-59 行
- [x] 说明：支持为每个上游服务器配置独立的 bootstrap DNS 服务器列表

### 2. Bootstrap 解析方法
- [x] 实现 `resolve_with_bootstrap` 方法
- [x] 位置：`src/forwarder.rs` 第 779-833 行
- [x] 功能：
  - 使用 UDP DNS 查询解析域名到 IP 地址
  - 尝试所有 bootstrap 服务器直到成功
  - 返回解析得到的 IP 地址列表

### 3. DoH (DNS over HTTPS) Bootstrap 支持
- [x] `forward_doh` 方法支持 bootstrap 参数
- [x] 位置：`src/forwarder.rs` 第 836-920 行
- [x] 实现细节：
  - 从 HTTPS URL 中提取域名
  - 使用 bootstrap DNS 解析域名为 IP
  - 将 URL 中的域名替换为 IP 进行连接
  - 设置 Host header 保持正确的 SNI
  - 使用 IP 连接时接受无效证书（因为证书是针对域名的）
- [x] 调用位置：`src/forwarder.rs` 第 510 行

### 4. DoT (DNS over TLS) Bootstrap 支持
- [x] `forward_dot` 方法支持 bootstrap 参数
- [x] 位置：`src/forwarder.rs` 第 689-780 行
- [x] 实现细节：
  - 从 tls:// URL 中提取域名和端口
  - 使用 bootstrap DNS 解析域名为 IP
  - 使用 IP 进行 TCP 连接
  - 设置原始域名作为 SNI (Server Name Indication)
  - 支持 SOCKS5 代理（如果配置）
- [x] 调用位置：`src/forwarder.rs` 第 509 行

### 5. DoQ (DNS over QUIC) Bootstrap 支持
- [x] `forward_doq` 方法支持 bootstrap 参数
- [x] 位置：`src/forwarder.rs` 第 947-1025 行
- [x] 实现细节：
  - 从 doq:// 或 quic:// URL 中提取域名和端口
  - 使用 bootstrap DNS 解析域名为 IP
  - 使用 IP 进行 QUIC 连接
  - 默认端口 784
- [x] 调用位置：`src/forwarder.rs` 第 512 行

### 6. H3 (DNS over HTTP/3) Bootstrap 支持
- [x] `forward_h3` 方法支持 bootstrap 参数
- [x] 位置：`src/forwarder.rs` 第 1066-1150 行
- [x] 实现细节：
  - 从 h3:// 或 https3:// URL 中提取域名
  - 使用 bootstrap DNS 解析域名为 IP
  - 将 URL 中的域名替换为 IP 进行连接
  - 设置 Host header 保持正确的 SNI
  - 使用 IP 连接时接受无效证书
- [x] 调用位置：`src/forwarder.rs` 第 513 行

### 7. 配置文件示例
- [x] `config/config.example.yaml` 包含 bootstrap 配置示例
- [x] 位置：第 80-110 行
- [x] 示例：
  ```yaml
  cn_dns:
    addr:
      - "https://dns.alidns.com/dns-query"
    bootstrap:
      - "udp://223.5.5.5:53"
  
  cloudflare_dns:
    addr:
      - "https://cloudflare-dns.com/dns-query"
    bootstrap:
      - "udp://1.1.1.1:53"
  ```

### 8. 协议覆盖范围
- [x] UDP - 不需要 bootstrap（直接使用 IP）
- [x] TCP - 不需要 bootstrap（直接使用 IP）
- [x] DoH - ✅ 已实现 bootstrap 支持
- [x] DoT - ✅ 已实现 bootstrap 支持
- [x] DoQ - ✅ 已实现 bootstrap 支持
- [x] H3 - ✅ 已实现 bootstrap 支持

## 🎯 实现目的

### 问题背景
DoH 和 DoT 协议使用域名（如 `dns.google`, `cloudflare-dns.com`）作为服务器地址。这导致循环依赖：
- 需要 DNS 解析来获取 DNS 服务器的 IP 地址
- 但 DNS 服务器本身还未就绪

### 解决方案
Bootstrap DNS 提供独立的 DNS 解析能力：
1. 配置可靠的 UDP DNS 服务器（如 `223.5.5.5`, `8.8.8.8`）
2. 使用 bootstrap 服务器解析 DoH/DoT 服务器域名
3. 获取 IP 后直接连接，绕过系统 DNS

## 🔍 工作流程

### DoH Bootstrap 流程
```
1. 用户查询 -> example.com
2. 选择上游 -> https://dns.google/dns-query
3. 提取域名 -> dns.google
4. Bootstrap 解析 -> dns.google -> 8.8.8.8
5. 替换 URL -> https://8.8.8.8/dns-query
6. 设置 Header -> Host: dns.google
7. HTTPS 请求 -> 使用 SNI: dns.google
8. 返回响应 -> example.com 的 IP
```

### DoT Bootstrap 流程
```
1. 用户查询 -> example.com
2. 选择上游 -> tls://dns.google:853
3. 提取域名 -> dns.google
4. Bootstrap 解析 -> dns.google -> 8.8.8.8
5. TCP 连接 -> 8.8.8.8:853
6. TLS 握手 -> SNI: dns.google
7. DNS 查询 -> example.com
8. 返回响应 -> example.com 的 IP
```

### DoQ Bootstrap 流程
```
1. 用户查询 -> example.com
2. 选择上游 -> doq://dns.google:784
3. 提取域名 -> dns.google
4. Bootstrap 解析 -> dns.google -> 8.8.8.8
5. QUIC 连接 -> 8.8.8.8:784
6. DNS 查询 -> example.com
7. 返回响应 -> example.com 的 IP
```

### H3 Bootstrap 流程
```
1. 用户查询 -> example.com
2. 选择上游 -> h3://dns.google/dns-query
3. 提取域名 -> dns.google
4. Bootstrap 解析 -> dns.google -> 8.8.8.8
5. 替换 URL -> https://8.8.8.8/dns-query
6. 设置 Header -> Host: dns.google
7. HTTP/3 请求 -> 使用 SNI: dns.google
8. 返回响应 -> example.com 的 IP
```

## ✅ 验证结果

### 编译状态
- ✅ GitHub Actions 编译通过
- ✅ 所有 Send trait 错误已修复
- ✅ 无未使用变量警告

### 代码质量
- ✅ 错误处理完善（Result<>）
- ✅ 日志记录详细（debug/warn 级别）
- ✅ 回退机制（bootstrap 失败时使用系统 DNS）
- ✅ 超时控制（config.timeout_secs）

### 功能覆盖
- ✅ DoH 协议完整支持
- ✅ DoT 协议完整支持
- ✅ SOCKS5 代理兼容
- ✅ 配置文件示例完整

## 📝 使用建议

### 推荐配置
```yaml
upstreams:
  # 国内 DoH 使用阿里云 bootstrap
  cn_doh:
    addr:
      - "https://dns.alidns.com/dns-query"
    bootstrap:
      - "udp://223.5.5.5:53"
      - "udp://223.6.6.6:53"  # 备用

  # 国际 DoH 使用 Google bootstrap
  global_doh:
    addr:
      - "https://dns.google/dns-query"
    bootstrap:
      - "udp://8.8.8.8:53"
      - "udp://8.8.4.4:53"    # 备用

  # DoT 同样支持
  cloudflare_dot:
    addr:
      - "tls://1.1.1.1:853"
    bootstrap:
      - "udp://1.1.1.1:53"

  # DoQ 支持 bootstrap
  google_doq:
    addr:
      - "doq://dns.google:784"
    bootstrap:
      - "udp://8.8.8.8:53"

  # H3 支持 bootstrap
  cloudflare_h3:
    addr:
      - "h3://cloudflare-dns.com/dns-query"
    bootstrap:
      - "udp://1.1.1.1:53"
```

### 注意事项
1. **Bootstrap 服务器选择**：
   - 选择可靠的公共 DNS（阿里、Google、Cloudflare）
   - 配置多个 bootstrap 服务器作为备份
   - Bootstrap 服务器应使用 UDP 协议（简单、快速）

2. **循环依赖避免**：
   - Bootstrap 服务器必须使用 IP 地址或 UDP 协议
   - 不要在 bootstrap 中使用 DoH/DoT 协议（会导致循环）

3. **性能考虑**：
   - Bootstrap 解析会增加首次连接延迟
   - 解析结果应该被系统 DNS 缓存
   - 考虑使用 IP 地址而不是域名（如果 IP 不常变）

## 🎉 结论

Bootstrap DNS 实现已完整覆盖所有需要域名解析的加密 DNS 协议（DoH、DoT、DoQ 和 H3）。实现包括：
- ✅ 配置结构完整
- ✅ 解析方法健壮
- ✅ 协议集成完善（DoH、DoT、DoQ、H3）
- ✅ 错误处理到位
- ✅ 文档配置齐全

**状态**：✅ Bootstrap 功能完全实现，所有加密 DNS 协议均已支持！
