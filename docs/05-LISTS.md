# 05 - 列表模块

## 📋 目录

- [概述](#概述)
- [列表类型](#列表类型)
- [域名列表](#域名列表)
- [IP CIDR 列表](#ip-cidr-列表)
- [热重新加载](#热重新加载)
- [列表格式](#列表格式)
- [配置示例](#配置示例)
- [使用场景](#使用场景)
- [验证工具](#验证工具)
- [最佳实践](#最佳实践)

---

## 概述

列表模块提供域名和 IP 地址段的管理功能，支持多种列表类型、热重新加载和灵活的配置方式。

### 核心特性

✅ **多种类型**：domain（域名）/ ipcidr（IP 段）  
✅ **热重新加载**：零停机更新列表  
✅ **灵活 interval**：支持立即或定时重新加载  
✅ **行内注释**：支持 `#` 注释  
✅ **规则命中追踪**：自动生成 `.hit.txt` 文件  
✅ **格式验证**：提供验证工具检查格式

---

## 列表类型

### 支持的列表类型

| 类型 | 说明 | 用途 |
|------|------|------|
| **domain** | 域名列表 | 域名匹配和分流 |
| **ipcidr** | IP CIDR 列表 | IP 段匹配和分流 |

### 基本配置

```yaml
lists:
  列表名称:
    type: "domain"          # 或 "ipcidr"
    format: "text"
    path: "./lists/file.txt"
    interval: 3600
    description: "描述"
```

---

## 域名列表

### 配置说明

```yaml
lists:
  direct:
    type: "domain"                      # 列表类型
    format: "text"                      # 文件格式
    path: "./lists/direct_domains.txt"  # 文件路径
    interval: 0                         # 重新加载间隔（秒）
    description: "国内直连域名"         # 描述（可选）
```

### 配置字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| **type** | string | ✅ | 固定为 `"domain"` |
| **format** | string | ✅ | 固定为 `"text"` |
| **path** | string | ✅ | 域名列表文件路径 |
| **interval** | integer | 否 | 重新加载间隔（秒），0=立即 |
| **description** | string | 否 | 列表描述，用于日志 |

### 文件格式

**基本格式**：每行一个域名，无前后缀

```text
# 注释行（以 # 开头）

# 完整域名
google.com
www.google.com

# 子域名通配（匹配所有子域名）
example.com           # 匹配 example.com 和 *.example.com

# 行内注释支持
baidu.com   # 百度主域名
qq.com      # 腾讯主域名
```

### 域名匹配规则

| 列表中的域名 | 查询域名 | 是否匹配 |
|-------------|---------|---------|
| `google.com` | `google.com` | ✅ 完全匹配 |
| `google.com` | `www.google.com` | ✅ 子域名匹配 |
| `google.com` | `mail.google.com` | ✅ 子域名匹配 |
| `google.com` | `notgoogle.com` | ❌ 不匹配 |
| `google.com` | `google.com.fake` | ❌ 不匹配 |

### 域名深度

**深度定义**：
- `.` = 深度 0（根域名）
- `com` = 深度 1（顶级域名）
- `google.com` = 深度 2
- `www.google.com` = 深度 3

**匹配优先级**：深度越大，匹配越精确，优先级越高

### 禁止的格式

❌ **不要添加前缀**：
```text
*.google.com      ❌ 不需要通配符
^google.com       ❌ 不需要正则表达式
||google.com      ❌ 不需要 adblock 格式
```

❌ **不要添加后缀**：
```text
google.com$       ❌ 不需要结尾标记
google.com/       ❌ 不需要路径符
```

❌ **不要混用格式**：
```text
127.0.0.1 google.com   ❌ 不需要 IP 地址
google.com=127.0.0.1   ❌ 不需要赋值格式
http://google.com      ❌ 不需要协议
google.com:80          ❌ 不需要端口
```

---

## IP CIDR 列表

### 配置说明

```yaml
lists:
  china_ips:
    type: "ipcidr"                   # 列表类型
    format: "text"                   # 文件格式
    path: "./lists/china_ips.txt"    # 文件路径
    interval: 86400                  # 重新加载间隔
    description: "国内 IP 段"        # 描述
```

### 文件格式

**格式**：管道符分隔

```
|CIDR段|国家代码|
```

**示例**：
```text
# IPv4 示例
|8.8.8.0/24|US|
|1.1.1.0/24|AU|
|223.5.5.0/24|CN|
|101.226.0.0/14|CN|

# IPv6 示例
|2001:4860::/32|US|
|2606:2800:220::/48|US|

# 行内注释支持
|39.156.0.0/16|CN|  # 阿里云 IP 段
|8.8.8.0/24|US|     # Google DNS
```

### CIDR 格式说明

**IPv4 CIDR**：
```text
|0.0.0.0/0|XX|                    # 全网
|10.0.0.0/8|XX|                   # 整个 10.0.0.0 段
|192.168.0.0/16|XX|               # 整个 192.168.0.0 段
|203.0.113.0/24|XX|               # 单个 /24 段
|203.0.113.1/32|XX|               # 单个 IP
```

**IPv6 CIDR**：
```text
|::/0|XX|                         # 全 IPv6 网络
|2001:db8::/32|XX|                # IPv6 段
|2606:2800:220::/48|XX|           # IPv6 段
|fe80::/10|XX|                    # link-local 地址
|::1/128|XX|                      # IPv6 本地回环
```

### 使用场景

- 🌍 **地理位置分流**：根据 IP 所属国家分流
- 🏢 **运营商分流**：根据 IP 所属运营商分流
- 🚫 **IP 黑名单**：拦截特定 IP 段
- 🚀 **CDN 优化**：根据 CDN 节点位置优化

---

## 热重新加载

### interval 规则

| interval 值 | 行为 | 使用场景 |
|-----------|------|----------|
| `0` | 文件改变后**立即**重新加载 | 开发环境、频繁修改 |
| `> 0` | 文件改变时启动倒计时，interval 期间**无视后续改变** | 生产环境、避免频繁重新加载 |

### 工作原理

```
应用启动
  ↓
加载初始列表
  ↓
启动后台监视任务（每 5 秒检查一次）
  ↓
检查文件是否改变
  ├─ 未改变：跳过
  └─ 已改变：
      ├─ interval == 0：立即重新加载
      └─ interval > 0：
          ├─ 无待处理更新：标记 pending_update=true，记录改变时间
          ├─ 有待处理更新：检查 (当前时间 - 改变时间) >= interval
          │   ├─ 是：执行 reload，清除 pending_update
          │   └─ 否：跳过（无视新的改变）
          └─ 倒计时期间忽略任何新的文件改变
```

### 关键机制

- **计时起点**：文件改变时刻（不是上次加载时刻）
- **倒计时期间**：忽略任何新的文件改变
- **执行时机**：倒计时归 0 时才执行 reload

### 配置示例

#### 立即更新（interval: 0）

```yaml
lists:
  direct:
    type: "domain"
    path: "./lists/direct_domains.txt"
    interval: 0              # 文件改变立即加载
```

**效果**：
```
11:23:00 → 修改文件
11:23:05 → 应用检测到变化（5秒内）
11:23:05 → 立即重新加载
11:23:05 → 新配置生效
```

#### 延迟更新（interval: 300）

```yaml
lists:
  proxy:
    type: "domain"
    path: "./lists/proxy_domains.txt"
    interval: 300            # 5 分钟倒计时
```

**效果**：
```
10:00:00 → 应用启动，首次加载
10:02:00 → 修改文件 → 触发！标记 pending_update=true
10:04:00 → 再次修改 → 被忽略（倒计时继续）
10:06:00 → 再次修改 → 被忽略（倒计时继续）
10:07:00 → 倒计时结束（距 10:02 已过 300 秒）→ 执行 reload
```

### interval 选择建议

| 列表大小 | 更新频率 | 推荐 interval |
|---------|---------|--------------|
| < 1,000 条 | 频繁 | 0 |
| 1,000-10,000 条 | 中等 | 300-600 |
| > 10,000 条 | 较少 | 1800+ |

### 性能考虑

| 操作 | 耗时 |
|------|------|
| 监视检查 | 每 5 秒执行一次 |
| 文件读取 | < 100ms（取决于文件大小） |
| 内存更新 | 原子操作，无阻塞 |
| DNS 查询影响 | 无（使用 RwLock 读写分离） |

---

## 列表格式

### 行内注释支持

**功能**：支持在同一行中使用 `#` 添加注释

**domain 列表示例**：
```text
google.com   # 谷歌主域
www.baidu.com # 百度子域
# 这是注释行
```

**ipcidr 列表示例**：
```text
|39.156.0.0/16|CN|  # 国内 IP 段
|8.8.8.0/24|US|     # Google 段
```

**规则**：
- 同一行中 `#` 之后的内容将被忽略
- 纯注释行（以 `#` 开头）被跳过
- 空行被跳过

### 规则命中追踪

**功能**：当某个规则匹配成功后，命中的域名会追加到 `.hit.txt` 文件

**文件命名规则**：
```
原文件：./lists/china_domains.txt
命中文件：./lists/china_domains.hit.txt
```

**重要**：
- 如果列表文件路径已包含 `.hit.`（如 `domains.hit.txt`），则不会再创建 hit 文件
- 每行一个域名（纯域名）
- 用于后续优化与分析
- `servers` 组不记录命中文件

**示例**：
```text
# china_domains.hit.txt
example.com
test.cn
www.baidu.com
api.qq.com
```

**用途**：
- 📊 分析高频查询域名
- 🔍 识别应该添加到规则的新域名
- 📈 评估不同规则的生效情况
- ⚙️ 优化规则配置

---

## 配置示例

### 示例 1：国内外分流

```yaml
lists:
  # 国内域名
  china_domains:
    type: "domain"
    format: "text"
    path: "./lists/china_domains.txt"
    interval: 3600                   # 1 小时更新
    description: "国内网站域名"
  
  # 国际域名
  global_domains:
    type: "domain"
    format: "text"
    path: "./lists/global_domains.txt"
    interval: 3600
    description: "国际网站域名"

upstreams:
  cn_dns:
    addr: "https://dns.alidns.com/dns-query"
  global_dns:
    addr: "https://dns.google/dns-query"

rules:
  main:
    - china_domains,cn_dns
    - global_domains,global_dns
```

### 示例 2：广告过滤

```yaml
lists:
  # 广告域名
  adblock:
    type: "domain"
    format: "text"
    path: "./lists/adblock.txt"
    interval: 86400                  # 每天更新
    description: "广告域名列表"

upstreams:
  clean_dns:
    addr: "https://dns.google/dns-query"
  blocked_dns:
    addr: "udp://127.0.0.1:1"        # 黑洞 DNS

rules:
  main:
    - adblock,blocked_dns            # 广告 → 拦截
    - .,clean_dns                    # 其他 → 正常
```

### 示例 3：开发环境（立即更新）

```yaml
lists:
  # 本地开发域名
  local_hosts:
    type: "domain"
    format: "text"
    path: "./lists/local_hosts.txt"
    interval: 0                      # 立即更新！
    description: "本地开发域名"

upstreams:
  local_dns:
    addr: "udp://192.168.1.1:53"

rules:
  main:
    - local_hosts,local_dns
```

### 示例 4：IP CIDR 分流

```yaml
lists:
  # 国内 IP 段
  china_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_ips.txt"
    interval: 86400                  # 每天更新
    description: "国内 IP 地址段"
  
  # 国际 IP 段
  global_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/global_ips.txt"
    interval: 86400
    description: "国际 IP 地址段"

upstreams:
  cn_dns:
    addr: "https://dns.alidns.com/dns-query"
  global_dns:
    addr: "https://dns.google/dns-query"

rules:
  main:
    - china_ips,cn_dns
    - global_ips,global_dns
```

### 示例 5：混合配置（不同更新策略）

```yaml
lists:
  # 高优先级：立即更新
  direct:
    type: "domain"
    path: "./lists/direct.txt"
    interval: 0
  
  # 中优先级：10 分钟更新
  proxy:
    type: "domain"
    path: "./lists/proxy.txt"
    interval: 600
  
  # 低优先级：1 小时更新
  adblock:
    type: "domain"
    path: "./lists/adblock.txt"
    interval: 3600
  
  # IP 列表：每天更新
  china_ips:
    type: "ipcidr"
    path: "./lists/china_ips.txt"
    interval: 86400
```

---

## 使用场景

### 场景 1：开发测试（快速迭代）

**需求**：频繁修改列表，需要实时生效

**配置**：
```yaml
lists:
  test:
    type: "domain"
    path: "./lists/test.txt"
    interval: 0              # 立即更新
```

### 场景 2：生产环境（稳定优先）

**需求**：稳定性优先，减少频繁重新加载

**配置**：
```yaml
lists:
  production:
    type: "domain"
    path: "./lists/prod.txt"
    interval: 1800           # 30 分钟更新
```

### 场景 3：自动化更新

**需求**：定时从远程拉取更新列表

**实现**：
```bash
#!/bin/bash
# update_lists.sh

# 下载最新列表
wget -O /app/lists/adblock.txt https://example.com/adblock.txt

# 配置 interval: 0 会立即重新加载
```

**crontab 配置**：
```
0 * * * * /app/update_lists.sh
```

### 场景 4：分级列表管理

**需求**：不同类型的列表使用不同的更新策略

**配置**：
```yaml
lists:
  # 实时列表（立即更新）
  realtime:
    type: "domain"
    path: "./lists/realtime.txt"
    interval: 0
  
  # 快速列表（5 分钟更新）
  fast:
    type: "domain"
    path: "./lists/fast.txt"
    interval: 300
  
  # 标准列表（30 分钟更新）
  standard:
    type: "domain"
    path: "./lists/standard.txt"
    interval: 1800
  
  # 静态列表（每天更新）
  static:
    type: "domain"
    path: "./lists/static.txt"
    interval: 86400
```

---

## 验证工具

### validate_domain_lists.py

**功能**：验证域名列表文件是否符合规范

**使用方法**：
```bash
# 检查指定文件
python validate_domain_lists.py direct_domains.txt proxy_domains.txt

# 检查所有默认文件
python validate_domain_lists.py
```

**检查项**：
- ✅ 域名格式有效性
- ✅ 禁止的前缀（`*.`, `||`, `^` 等）
- ✅ 禁止的后缀（`$`, `|`, `/*` 等）
- ✅ 协议、路径、端口等非法内容
- ✅ 统计有效域名数量

**输出示例**：
```
检查文件: direct_domains.txt
============================================================

统计信息:
  总行数: 31
  有效域名: 30 ✓
  注释行: 1
  空行: 0
  无效行: 0

✅ 所有域名格式都是有效的!
```

**CI/CD 集成**：
```yaml
# GitHub Actions
- name: 验证域名列表
  run: python validate_domain_lists.py
```

---

## 最佳实践

### 1. 列表组织

✅ **推荐**：
- 按功能分类列表（direct、proxy、adblock）
- 使用有意义的列表名称
- 添加描述信息
- 保持列表文件整洁

❌ **不推荐**：
- 所有域名放在一个列表
- 使用数字命名（list1、list2）
- 列表过于分散

### 2. interval 配置

✅ **推荐**：
- 开发环境：interval: 0
- 测试环境：interval: 300-600
- 生产环境：interval: 1800-3600
- 根据列表大小调整

❌ **不推荐**：
- 生产环境 interval: 0（频繁重新加载）
- 大文件列表 interval: 0（性能影响）

### 3. 文件管理

✅ **推荐**：
- 使用版本控制管理列表文件
- 定期备份列表文件
- 使用验证工具检查格式
- 分析 `.hit.txt` 文件优化规则

❌ **不推荐**：
- 手动编辑未经验证
- 不备份重要列表
- 忽略 `.hit.txt` 文件

### 4. 性能优化

✅ **推荐**：
- 合理设置 interval
- 避免重复域名
- 定期清理无效域名
- 监控重新加载频率

---

## 相关文档

- [01-LOG.md](01-LOG.md) - 日志模块
- [02-LISTENER.md](02-LISTENER.md) - 监听器模块
- [03-CACHE.md](03-CACHE.md) - 缓存模块
- [04-UPSTREAMS.md](04-UPSTREAMS.md) - 上游服务器模块
- [06-RULES.md](06-RULES.md) - 规则模块
