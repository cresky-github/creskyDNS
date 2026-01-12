# DNS 转发器使用指南

## 配置文件说明

### 1. 监听器配置 (listener)
```yaml
listener:
    main: 5353        # 主DNS服务器端口
    backup: 5354      # 备用DNS服务器端口
    ipv6: 5355        # IPv6专用端口
```
- 支持多个DNS服务器实例
- 每个实例监听不同的端口
- 同时支持IPv4和IPv6

### 2. 缓存配置 (cache)
```yaml
cache:
    fast:             # 快速缓存
        size: 5000    # 缓存条目数
        min_ttl: 30   # 最小TTL（秒）
        max_ttl: 1800 # 最大TTL（秒）
        output: "./output/cache/fast.cache.txt"  # 可选，缓存输出文件路径
        cold_start:   # 可选，冷启动配置
          enabled: true
          timeout: 5000
          parallel: 10
```
- `size`: 缓存的最大条目数
- `min_ttl`: 最小缓存时间（上游TTL小于此值时使用此值）
- `max_ttl`: 最大缓存时间（上游TTL大于此值时使用此值）
- `output`: 可选，指定缓存输出文件路径。格式为 `|cache ID|rule ID|domain|ttl|`，用于调试和优化缓存策略
- `cold_start`: 可选，冷启动配置
  - `enabled`: 是否启用冷启动（默认 true）
  - `timeout`: 冷启动超时时间（毫秒，默认 5000）
  - `parallel`: 并发查询数（默认 10）

**冷启动说明**：
应用启动时，自动从 `output` 文件读取上次运行的缓存内容，根据 rule ID 找到对应的 upstream，重新查询这些域名以刷新 IP 和 TTL，然后写入缓存。这样可以快速恢复应用的缓存状态，减少启动时的外网查询。

### 3. 上游DNS服务器配置 (upstreams)
```yaml
upstreams:
  alidns_doh:
    addr: "https://dns.alidns.com/dns-query"  # 服务器地址
    bootstrap: "udp://223.5.5.5:53"          # Bootstrap DNS（DoH需要）
    cache: "standard"                         # 使用的缓存ID
```
- 支持UDP、TCP、DoH、DoT协议
- DoH/DoT需要bootstrap来解析服务器域名
- 可为每个上游指定不同的缓存策略

### 4. 域名列表配置 (lists)
```yaml
lists:
  direct:
    type: "domain"                            # 列表类型
    path: "direct_domains.txt"                # 本地文件路径
    url: "https://cdn.example.com/list.txt"   # 远程URL（可选）
    interval: 86400                           # 更新间隔（秒）
```
- 支持本地文件和远程URL
- 远程列表会定期自动更新
- `interval`: 更新间隔时间

### 5. 规则配置 (rules)
```yaml
rules:
    # 服务器实例规则（可选）
    servers:
        - main,local_ali      # 实例名,上游名
        - backup,google_udp

    # 域名规则
    domestic:
        - direct,alidns_doh   # 域名列表名,上游名
        - adblock,local_ali
```
- `servers`: 服务器实例规则（可选）
- 域名规则: 按域名列表匹配到上游服务器

## 使用方法

### 1. 启动服务
```bash
# 使用默认配置文件
cargo run

# 指定配置文件
cargo run config.yaml

# 使用环境变量
export DNS_FORWARDER_CONFIG=config.yaml
cargo run
```

### 2. 测试DNS解析
```bash
# 测试主服务器
nslookup google.com 127.0.0.1 -port=5353

# 测试备用服务器
nslookup google.com 127.0.0.1 -port=5354

# TCP测试
dig @127.0.0.1 -p 5353 +tcp example.com
```

### 3. 域名列表文件格式
域名列表文件每行一个域名：
```
baidu.com
qq.com
github.com
google.com
```

### 4. 规则匹配逻辑
1. 如果配置了`servers`规则，则按服务器实例匹配
2. 如果没有`servers`规则，则完全按域名规则匹配
3. 域名规则按顺序匹配，第一个匹配的规则生效

## 高级配置

### 广告屏蔽
通过配置adblock域名列表，可以屏蔽广告域名：
```yaml
lists:
  adblock:
    type: "domain"
    path: "adblock_domains.txt"
    url: "https://example.com/adblock.txt"
    interval: 3600

rules:
    main:
        - adblock,local_dns  # 返回NXDOMAIN屏蔽广告
```

### 多地域DNS
为不同地区的域名使用相应的DNS服务器：
```yaml
lists:
  china:
    type: "domain"
    path: "china_domains.txt"

  global:
    type: "domain"
    path: "global_domains.txt"

rules:
    routing:
        - china,ali_dns      # 中国域名用阿里DNS
        - global,google_dns  # 全球域名用Google DNS
```

### 缓存策略优化
为不同类型的查询使用不同的缓存策略：
```yaml
cache:
    short:    # 短期缓存（游戏、即时通讯）
        size: 1000
        min_ttl: 10
        max_ttl: 300
    long:     # 长期缓存（静态资源）
        size: 10000
        min_ttl: 3600
        max_ttl: 86400

upstreams:
  game_dns:
    addr: "udp://8.8.8.8:53"
    cache: "short"

  cdn_dns:
    addr: "udp://1.1.1.1:53"
    cache: "long"
```