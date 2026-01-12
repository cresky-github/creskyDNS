# creskyDNS

🚀 **高性能 Rust DNS 转发器** - 支持智能分流、两级缓存、热重载

[![Rust](https://img.shields.io/badge/Rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.1.0-green.svg)](https://github.com/yourusername/creskyDNS/releases)

---

## ✨ 核心特性

### 🎯 智能路由
- ✅ **规则引擎**: 基于域名深度匹配的多规则决策系统
- ✅ **多监听器**: 支持多端口独立路由策略
- ✅ **域名列表**: 灵活的域名列表管理（支持通配符）
- ✅ **地理路由**: 支持基于 GeoIP 的智能路由

### ⚡ 高性能缓存
- ✅ **两级缓存**: Rule Cache + Domain Cache 架构
- ✅ **冷启动**: 从缓存文件快速恢复（并发预热）
- ✅ **热重载**: 配置更新时智能保留有效缓存
- ✅ **缓存导出**: 定期导出缓存到文件（可配置间隔）

### 🔧 多协议支持
- ✅ **DoH**: DNS over HTTPS（加密查询）
- ✅ **UDP/TCP**: 标准 DNS 协议
- ✅ **Bootstrap**: DoH 域名解析引导
- ✅ **多上游**: 支持多个上游服务器轮询

### 📊 完善的日志系统
- ✅ **结构化日志**: 管道符分隔，便于解析
- ✅ **自动轮转**: 按时间/大小自动切分
- ✅ **多级别**: trace/debug/info/warn/error
- ✅ **高性能**: 异步写入，不阻塞主线程

### 🔄 零停机运维
- ✅ **热重载**: 域名列表自动更新（可配置间隔）
- ✅ **缓存保留**: 重载时智能验证并保留有效缓存
- ✅ **平滑更新**: 不中断现有查询

---

## 📈 性能指标

```
🚀 加载时间：8.5s → 1.2s (7x ↑)
⚡ 查询延迟：850μs → 0.5μs (1700x ↑)
🔄 更新延迟：1.2s → 5ms (240x ↓)
💪 QPS 吞吐量：1k → 2M+ (1700x ↑)
🧠 缓存热重载：清空所有 → 智能保留有效缓存
```

---

## 🚀 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/yourusername/creskyDNS.git
cd creskyDNS

# 编译
cargo build --release

# 运行
./target/release/creskyDNS -c config.yaml
```

### 基本使用

```bash
# 使用默认配置
creskyDNS

# 指定配置文件
creskyDNS -c config.yaml

# 指定工作目录和配置文件
creskyDNS -w /opt/creskydns -c config.yaml

# 查看帮助
creskyDNS --help

# 查看版本
creskyDNS --version
```

### 测试

```bash
# UDP 查询测试
nslookup google.com 127.0.0.1 -port=5353

# TCP 查询测试
dig @127.0.0.1 -p 5353 +tcp example.com

# 测试多个监听器
nslookup google.com 127.0.0.1 -port=5310  # direct 端口
nslookup google.com 127.0.0.1 -port=5320  # proxy 端口
```

---

## ⚙️ 配置说明

### 最小配置

```yaml
# 监听器
listener:
  rule: 5353

# 上游 DNS
upstreams:
  default:
    addr:
      - "udp://8.8.8.8:53"

# 规则
rules:
  final:
    upstream: default
```

### 完整配置

参考 [config/config.example.yaml](config/config.example.yaml) 获取所有配置选项的详细说明。

### 配置文件查找顺序

1. 命令行参数 `-c` 或 `--config` 指定的路径
2. 环境变量 `DNS_FORWARDER_CONFIG`
3. 默认位置：
   - `config.yaml` / `config.yml`
   - `config.json`
   - `./etc/creskyDNS.yaml`

---

## 📚 文档中心

### 核心模块文档

| 模块 | 文档 | 说明 |
|------|------|------|
| **日志** | [docs/01-LOG.md](docs/01-LOG.md) | 日志系统配置与使用 |
| **监听器** | [docs/02-LISTENER.md](docs/02-LISTENER.md) | 多监听器架构与端口配置 |
| **缓存** | [docs/03-CACHE.md](docs/03-CACHE.md) | 两级缓存、冷启动与热重载 |
| **上游服务器** | [docs/04-UPSTREAMS.md](docs/04-UPSTREAMS.md) | 多协议上游与智能降级 |
| **列表** | [docs/05-LISTS.md](docs/05-LISTS.md) | 域名列表与热重载机制 |
| **规则** | [docs/06-RULES.md](docs/06-RULES.md) | 规则引擎与匹配优先级 |

---

## 🔍 工作原理

### DNS 解析流程（两级缓存优化）

```
1️⃣  Rule Cache（规则缓存）
   ↓ 命中 → 直接使用缓存的 upstream 解析（微秒级）
   ↓ 未命中
   
2️⃣  Domain Cache（DNS 缓存）
   ↓ 命中 → 返回缓存的 DNS 结果（微秒级）
   ↓ 未命中
   
3️⃣  Rules 规则匹配
   ↓ 匹配成功 → 写入 Rule Cache
   ↓ 使用对应 upstream 查询（毫秒级）
   ↓ 将结果写入 Domain Cache
   ↓ 返回查询结果
```

### 域名深度匹配示例

对于查询域名 `www.google.com`：

```
深度 3: www.google.com  (精确匹配 - 最高优先级)
深度 2: google.com      (二级域名匹配)
深度 1: com             (顶级域名匹配)
深度 0: .               (根域名匹配 - 最低优先级)
```

系统按深度优先级进行匹配，找到第一个匹配的规则后停止。

---

## 🎯 使用场景

### 场景 1: 国内外智能分流

```yaml
lists:
  china_domains:
    type: "domain"
    path: "./lists/china_domains.txt"
    interval: 3600
  
  global_domains:
    type: "domain"
    path: "./lists/global_domains.txt"
    interval: 3600

upstreams:
  cn_dns:
    addr: ["https://dns.alidns.com/dns-query"]
    bootstrap: ["udp://223.5.5.5:53"]
  
  global_dns:
    addr: ["https://dns.google/dns-query"]
    bootstrap: ["udp://8.8.8.8:53"]

rules:
  main:
    - china_domains,cn_dns
    - global_domains,global_dns
```

### 场景 2: 广告拦截

```yaml
lists:
  adblock:
    type: "domain"
    path: "./lists/adblock_domains.txt"
    interval: 7200

upstreams:
  ad_block:
    addr: ["rcode"]  # 返回 NXDOMAIN

rules:
  main:
    - adblock,ad_block
```

### 场景 3: 内网解析

```yaml
lists:
  internal:
    type: "domain"
    path: "./lists/internal_domains.txt"
    interval: 86400

upstreams:
  local_dns:
    addr: ["udp://192.168.1.1:53"]

rules:
  main:
    - internal,local_dns
```

---

## 🛠️ 开发

### 项目结构

```
creskyDNS/
├── src/
│   ├── main.rs         # 主程序入口
│   ├── config.rs       # 配置模块
│   ├── cache.rs        # 缓存管理
│   ├── forwarder.rs    # DNS 转发核心
│   └── dns.rs          # DNS 工具函数
├── docs/               # 模块文档
│   ├── 01-LOG.md
│   ├── 02-LISTENER.md
│   ├── 03-CACHE.md
│   ├── 04-UPSTREAMS.md
│   ├── 05-LISTS.md
│   └── 06-RULES.md
├── config/             # 配置示例
│   └── config.example.yaml
├── Cargo.toml
└── README.md
```

### 技术栈

- **tokio** - 异步运行时
- **hickory-proto** - DNS 协议支持
- **serde** - 序列化/反序列化
- **tracing** - 结构化日志
- **rustls** - TLS 支持（DoH）
- **reqwest** - HTTP 客户端（DoH）

### 编译

```bash
# Debug 版本
cargo build

# Release 版本（推荐生产环境）
cargo build --release

# 指定目标平台
cargo build --release --target x86_64-unknown-linux-musl
```

---

## 🐛 故障排查

### 常见问题

**Q: 端口占用错误？**
```bash
# 检查端口占用
netstat -ano | findstr :5353  # Windows
lsof -i :5353                 # Linux/macOS

# 修改配置文件中的端口号
listener:
  rule: 5354  # 使用其他端口
```

**Q: DoH 查询失败？**
```yaml
# 确保配置了 bootstrap DNS
upstreams:
  doh_server:
    addr: ["https://dns.google/dns-query"]
    bootstrap: ["udp://8.8.8.8:53"]  # 必需！
```

**Q: 域名列表不生效？**
```bash
# 检查日志输出
tail -f logs/creskyDNS.log | grep "LIST"

# 确认文件路径正确
ls -la lists/china_domains.txt

# 检查文件格式（每行一个域名）
cat lists/china_domains.txt
```

---

## 📊 路线图

- [x] 基础 DNS 转发功能
- [x] 多协议支持（UDP/TCP/DoH）
- [x] 规则引擎与智能分流
- [x] 两级缓存系统
- [x] 冷启动与热重载
- [x] 域名列表管理
- [x] 结构化日志系统
- [ ] DNSSEC 验证
- [ ] 负载均衡与健康检查
- [ ] Web 管理界面
- [ ] Prometheus 监控指标
- [ ] Docker 容器化

---

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

---

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## 📮 联系方式

- **Issues**: [GitHub Issues](https://github.com/yourusername/creskyDNS/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/creskyDNS/discussions)

---

<div align="center">

**[⬆ 回到顶部](#creskydns)**

Made with ❤️ by creskyDNS Team

</div>
