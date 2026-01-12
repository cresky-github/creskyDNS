# 🎯 高级规则匹配 - Final 规则详解

## 📋 目录

- [概述](#概述)
- [Final 规则原理](#final-规则原理)
- [配置说明](#配置说明)
- [详细工作流程](#详细工作流程)
- [配置示例](#配置示例)
- [使用场景](#使用场景)
- [双层解析机制](#双层解析机制)
- [输出文件管理](#输出文件管理)
- [常见问题](#常见问题)

---

## 概述

**Final 规则**是 creskyDNS 中的高级兜底规则，用于处理**未匹配到任何预定义规则的查询**。

### 核心特性

✅ **智能双层解析** - 根据 IP 所属国家智能选择上游  
✅ **国家代码判定** - 自动检测响应 IP 的地理位置  
✅ **动态输出** - 记录优化候选域名  
✅ **灵活配置** - 支持多个上游组合  
✅ **性能优化** - 减少冗余查询  

---

## Final 规则原理

### 规则流程图

```bash
# 查看输出文件
cat ./output/domains.txt | head -20

# 统计域名数量
wc -l ./output/domains.txt

# 排序去重
sort ./output/domains.txt | uniq > domains_unique.txt

# 按照输出内容更新规则列表
cat ./output/domains.txt >> ./lists/new_domains.txt
```

---

## 规则命中追踪（list 命中文件）

当某个主规则组（main）中的规则匹配成功后，会将命中的域名追加到该规则使用的列表的命中文件，命名规则为“原名.hit.txt”。

**行为说明**：
- 命中文件与列表文件同目录、同前缀，扩展名为 `.hit.txt`。
- 每行一个域名（纯域名），便于后续统计、分析与优化。- **重要**：如果列表文件路径中已经包含 `.hit.`（如 `domains.hit.txt`），则不会再创建 hit 文件，避免循环记录。- 例外：`servers` 组的规则不记录命中文件（不产生 .hit.txt）。

**示例**：
```text
列表文件: ./lists/china_domains.txt
命中文件: ./lists/china_domains.hit.txt

列表文件: ./lists/global_domains.txt
命中文件: ./lists/global_domains.hit.txt
```

    ├─ 国家代码 = CN?
```bash
# 去重并合并命中文件到新的优化列表
sort ./lists/china_domains.hit.txt | uniq >> ./lists/china_domains_optimized.txt

# 将命中域名回写到规则列表（人工审核后）
cat ./lists/global_domains.hit.txt >> ./lists/global_domains.txt
```
    │  ├─ YES → 返回 primary 结果
    │  └─ NO  → 使用 fallback_upstream 再解析
    │
    ├─ 检查 output 配置
    └─ 记录域名（如果有输出文件）
```

### 关键概念

| 概念 | 说明 |
|------|------|
| **primary_upstream** | 主上游服务器标签（首选）|
| **fallback_upstream** | 备用上游服务器标签（备选）|
| **country_code** | IP 地址所属国家代码（ISO 3166-1） |
| **双层解析** | 最多执行两次 DNS 查询 |
| **output 文件** | 记录需要优化的域名 |

---

## 配置说明

### YAML 配置格式

```yaml
rules:
  final:
    primary_upstream: "dns_name"        # 必填：主上游 DNS 标签
    fallback_upstream: "backup_dns"     # 必填：备用上游 DNS 标签
    ipcidr: "ipcidr_list_name"          # 可选：IP CIDR 列表，用于判断国家代码
    output: "/path/to/output.txt"       # 可选：输出文件路径
```

### 配置字段详解

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| **primary_upstream** | string | ✅ | 主上游服务器的标签名称 |
| **fallback_upstream** | string | ✅ | 备用上游服务器的标签名称 |
| **ipcidr** | string | 否 | IP CIDR 列表名称（用于国家代码判定）|
| **output** | string | 否 | 输出文件路径（绝对或相对路径）|

### 配置示例

#### 基础配置

```yaml
lists:
  # IP CIDR 列表（用于国家代码判定）
  cn_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_ips.txt"
    interval: 86400

upstreams:
  default_upstream:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"
  
  backup_upstream:
    addr: "https://dns.google/dns-query"
    cache: "main"

rules:
lists:
  # IP CIDR 列表
  cn_ips:
    type: "ipcidr"
    format: "text"
    path: "./lists/china_ips.txt"
    interval: 86400
  
  # 域名列表
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

upstreams:
  main_dns:
    addr: "https://dns.alidns.com/dns-query"
    cache: "main"
  
  global_dns:
    addr: "https://dns.google/dns-query"
    cache: "main"

rules:
  # 主规则组
  main:
    - cn_domains,main_dns
    - global_domains,global_dns
  
  # Final 规则（兜底，含 IP CIDR 列表）
  final:
    primary_upstream: "main_dns"
    fallback_upstream: "global_dns"
    ipcidr: "cn_ips"          # 使用 cn_ips 判断国家代码
  # 主规则组
  main:
    - cn_domains,main_dns
    - global_domains,global_dns
  
  # Final 规则（兜底）
  final:
    primary_upstream: "main_dns"
    fallback_upstream: "global_dns"
    output对比 IP CIDR 列表判断国家代码
   │  └─ 查询 ipcidr 指定的列表（cn_ips）
   │     ├─ 检查 203.0.113.45 是否在列表中
   │     └─ 国家代码: US（非 CN，不在 cn_ips 中
---

## 详细工作流程

### 查询流程

```
1. 客户端发起 DNS 查询: example.com

2. creskyDNS 尝试匹配规则
   ├─ 检查 main 规则组 → 未匹配
   ├─ 检查 servers 规则组 → 未匹配
   └─ 无其他规则 → 进入 Final 规则

3. Final 规则执行
   ├─ 第一步：使用 primary_upstream 解析
   │  └─ 向 main_dns 查询 example.com
   │     └─ 返回 IP: 203.0.113.45
   │
   ├─ 第二步：检查 IP 的国家代码
   │  └─ 对比 IP CIDR 列表
   │     └─ 国家代码: US（非 CN）
   │
   ├─ 第三步：执行 fallback 解析
   │  └─ 向 global_dns 查询 example.com
   │     └─ 返回 IP: 203.0.113.50
   │
   ├─ 第四步：选择最终结果
   │  └─ 采用 fallback_upstream 的结果
   │     └─ 返回 IP: 203.0.113.50
   │
   └─ 第五步：输出记录
      └─ 将 example.com 写入 output 文件
         └─ 文件内容: example.com

4. 返回查询结果给客户端
   └─ IP: 203.0.113.50
```

### 查询决策树

```
              example.com 查询
                    |
        ____________|____________
       |                        |
   匹配规则?              未匹配规则?
   |                        |
 返回结果                进入 Final
 并缓存                      |
                      使用 primary 解析
                            |
                  国家代码 = CN?
                    /          \
                  YES            NO
                  /              \
           返回 primary      使用 fallback 解析
               结果              |
                          返回 fallback
                              结果
                            并记录域名
```

### 响应流程对比

#### 情景 A：国内 IP（country_code = CN）

```
查询: baidu.com
使用 primary 解析: 返回 IP 39.156.66.10
② 对比 ipcidr 列表(cn_ips): 匹配到 |39.156.0.0/16|CN|
③ 检查国家代码: CN
④ 决策: 采用 primary 结果
⑤ 响应: 39.156.66.10
⑥ 输出: 记录 baidu.com（可选）
```

#### 情景 B：国外 IP（country_code = US）

```
查询: google.com

① 使用 primary 解析: 返回 IP 142.250.185.68
② 对比 ipcidr 列表(cn_ips): 未匹配
③ 检查国家代码: US（非 CN）
④ 决策: 执行 fallback 解析
⑤ fallback 解析: 返回 IP 142.250.185.70
⑥ 响应: 142.250.185.70
⑦ 输出: 记录 google.com（可选）0
⑥ 输出: 记录 google.com
```

---

## 配置示例

### 场景 1：国内外智能分流

```yaml
upstreams:
  cn_dns:
    addr: "https://dns.alidns.com/dns-query"
    bootstrap: "udp://223.5.5.5:53"
    cache: "main"
  
  global_dns:
    addr: "https://dns.google/dns-query"
    bootstrap: "udp://8.8.8.8:53"
    cache: "main"

lists:
  # 已知国内域名
  cn_domains:
    type: "domain"
    path: "./lists/china_domains.txt"
    interval: 3600
  
  # 已知国外域名
  global_domains:
    type: "domain"
    path: "./lists/global_domains.txt"
    interval: 3600

rules:
  main:
    # 明确的分类规则
    - cn_domains,cn_dns
    - global_domains,global_dns
  
  # 未分类域名的智能判定
  final:
    primary_upstream: "cn_dns"
    fallback_upstream: "global_dns"
    output: "./output/uncategorized_domains.txt"

cache:
  main:
    size: 10000
    min_ttl: 60
    max_ttl: 86400

log:
  enabled: true
  path: "./logs/creskyDNS.log"
  level: "info"
```

### 场景 2：运营商优化
lists:
  # IP CIDR 列表（多个运营商）
  mobile_ips:
    type: "ipcidr"
    path: "./lists/china_mobile_ips.txt"
    interval: 86400
  
  unicom_ips:
    type: "ipcidr"
    path: "./lists/china_unicom_ips.txt"
    interval: 86400

upstreams:
  # 移动 DNS
  mobile:
    addr: "https://dns.mobile.com/dns-query"
    cache: "main"
  
  # 联通 DNS
  unicom:
    addr: "https://dns.unicom.com/dns-query"
    cache: "main"
  
  # 备用 DNS
  backup:
    addr: "https://dns.google/dns-query"
    cache: "main"

rules:
  main:
lists:
  # 内网 IP 段
  local_ips:
    type: "ipcidr"
    path: "./lists/local_ips.txt"
    interval: 0               # 立即更新
  
  # 开发域名列表
  dev_domains:
    type: "domain"
    path: "./lists/dev_hosts.txt"
    interval: 0               # 立即更新

upstreams:
  dev_dns:
    addr: "udp://192.168.1.1:53"
    cache: "dev"
  
  backup_dns:
    addr: "https://dns.google/dns-query"
    cache: "main"

rules:
  main:
    - dev_domains,dev_dns
  
  final:
    primary_upstream: "dev_dns"
    fallback_upstream: "backup_dns"
    ipcidr: "local_ips"       # 使用本地 IP 列表判定uery"
    cache: "main"

lists:
  dev_domains:
    type: "domain"
    path: "./lists/dev_hosts.txt"
    interval: 0              # 立即重新加载

rules:
  main:
    - dev_domains,dev_dns
  
  final:
    primary_upstream: "dev_dns"
    fallback_upstream: "backup_dns"
    output: "./output/dev_optimization.txt"
```

---

## 使用场景

### 场景 1：优化域名分类

**问题**：有一些新域名未被分类到任何规则中

**解决方案**：
1. 配置 Final 规则使用国内外 DNS 双解析
2. 根据 IP 国家代码智能决策
3. 记录所有未分类域名到输出文件
4. 定期查看输出文件，优化分类规则

**配置**：
```yaml
rules:
  final:
    primary_upstream: "cn_dns"
    fallback_upstream: "global_dns"
    output: "./output/domains_to_classify.txt"
```

**流程**：
```
1. 新域名查询 → 未匹配到规则
2. Final 规则处理 → 双解析判定
3. 输出文件记录 → domains_to_classify.txt
4. 分析文件内容 → 识别域名特性
5. 更新规则 → 添加到相应列表
```

### 场景 2：CDN 节点优化

**问题**：某些 CDN 域名需要选择最近的节点

**解决方案**：
1. Primary 使用本地运营商 DNS
2. Fallback 使用其他运营商 DNS
3. 根据 IP 归属地选择最优节点
4. 记录需要特殊优化的域名

### 场景 3：灾备和故障恢复

**问题**：某个 DNS 可能不稳定或无法解析某些域名

**解决方案**：
根据 ipcidr 列表判定：
返回的 IP 在列表中?
  │
  ├─ YES → 国家代码匹配列表的代码（如 CN）
  │        只执行一次查询（primary）
  │
  └─ NO  → 国家代码不匹配（如 US、JP 等）
           执行两次查询（primary + fallback）
```

**列表匹配判定**：
```
查询: example.com
① primary 解析: 返回 203.0.113.45
② 检查 ipcidr 列表（cn_ips）:
   - 203.0.113.45 在列表中? → CN（国内 IP）
   - 203.0.113.45 不在列表中? → 非 CN（国外 IP）
③ 决策：是否执行 fallback

**配置**：
```yaml
rules:
  final:
    primary_upstream: "primary_dns"
    fallback_upstream: "secondary_dns"
    output: "./output/problematic_domains.txt"
```

---

## 双层解析机制

### 工作原理

Final 规则的完整工作流程：

```
1️⃣ 使用 primary_upstream 查询
   ↓
2️⃣ 对比返回 IP 在指定的 ipcidr 列表
   ↓
3️⃣ 检查国家代码
   ├─ 匹配到 CN（在列表中）
   │  └─ 采用 primary 结果 → 返回给客户端
   │
   └─ 未匹配到 CN（不在列表中）
      └─ 4️⃣ 使用 fallback_upstream 进行第二次查询
         └─ 5️⃣ 采用 fallback 结果 → 返回给客户端
   
6️⃣ 将域名写入 output 文件（用于优化规则）
```

**列表格式**（|CIDR|country_code|）：
```text
|8.8.8.0/24|US|
|223.5.5.0/24|CN|
|142.250.0.0/15|US|
|39.156.0.0/16|CN|
```

### 何时触发双层解析

根据返回的 IP 是否在指定的 ipcidr 列表中判定：

```
返回 IP 在 ipcidr 列表中?
  │
  ├─ YES（CN）    → 只执行一次查询（primary）
  │
  └─ NO（非CN）   → 执行两次查询（primary + fallback）
```

### 性能影响

| 场景 | 查询次数 | 耗时 | 说明 |
|------|---------|------|------|
| **国内域名** | 1 次 | ~50ms | 只使用 primary |
| **国外域名** | 2 次 | ~100ms | primary + fallback |
| **平均** | 1.5 次 | ~75ms | 取决于国内外比例 |

### 缓存策略

```yaml
cache:
  main:
    size: 10000
    min_ttl: 300      # 较长的缓存时间
    max_ttl: 86400    # 支持长期缓存
```

**缓存益处**：
- ✅ 减少重复查询
- ✅ 降低平均响应时间
- ✅ 减轻上游负载

---

## 输出文件管理

### 输出文件格式

**文件位置**：由 `output` 字段指定

**文件内容**：每行一个域名，纯域名格式

**示例内容**：
```text
example.com
test.example.org
api.service.io
cdn.content.net
```

### 输出内容说明

| 场景 | 是否输出 | 说明 |
|------|---------|------|
| **匹配其他规则** | ❌ 否 | 不触发 Final 规则 |
| **国内 IP (CN)** | ✅ 是 | 使用 primary 结果 |
| **国外 IP (非CN)** | ✅ 是 | 使用 fallback 结果 |

### 文件操作建议

#### 配置相对路径

```yaml
output: "./output/domains.txt"    # 相对于启动目录
```

#### 配置绝对路径（推荐）

```yaml
output: "/var/log/creskyDNS/optimized_domains.txt"  # Linux
output: "C:/logs/creskyDNS/domains.txt"             # Windows
```

#### 文件管理

```bash
# 查看输出文件
cat ./output/domains.txt | head -20

# 统计域名数量
wc -l ./output/domains.txt

# 排序去重
sort ./output/domains.txt | uniq > domains_unique.txt

# 按照输出内容更新规则列表
cat ./output/domains.txt >> ./lists/new_domains.txt
```

### 输出文件监控

```bash
# 实时监控输出文件变化
tail -f ./output/domains.txt

# 定期导出统计
find ./output -name "domains.txt" -mtime -7 -exec wc -l {} \;
```

---

## 常见问题

### Q1: Final 规则何时触发？

**A**: Final 规则在以下情况触发：
1. 查询未匹配主规则组（main）
2. 查询未匹配监听器规则（servers）
3. 没有其他规则可应用

**示例**：
```yaml
rules:
  main:
    - cn_domains,cn_dns
  
  # 以下查询会触发 Final:
  # - example.com (不在 cn_domains 中)

---

## 列表注释规则（适用于所有 list）

**规则**：
- 同一行中 `#` 之后的内容将被忽略（行内注释）。
- 以 `#` 开头的纯注释行与空行会被跳过。
- 适用于所有列表类型：`domain` 与 `ipcidr`。

**示例（domain 列表）**：
```text
google.com    # 谷歌
www.baidu.com # 百度
# 这是注释行
```

**示例（ipcidr 列表）**：
```text
|39.156.0.0/16|CN|  # 百度段
|8.8.8.0/24|US|     # Google 段
```
  # - test.org (不在任何列表中)
```

### Q2: primary_upstream 和 fallback_upstream 的区别？

**A**:

| 特性 | primary | fallback |
|------|---------|----------|
| **使用频率** | 总是首先使用 | 条件使用 |
| **触发条件** | 无条件 | 当 primary 返回非 CN IP |
| **结果选择** | 有条件地采用 | 当触发时采用 |
| **典型角色** | 主 DNS | 备用 DNS |

**示例**：
```yaml
rules:
  final:
    primary_upstream: "cn_dns"      # 国内 DNS，优先使用
    fallback_upstream: "global_dns" # 国际 DNS，备用
```
`ipcidr` 字段指定的 IP CIDR 列表判定。配置中指定列表名称：

```yaml
lists:
  # IP CIDR 列表（包含国家代码）
  cn_ips:
    type: "ipcidr"
    path: "./lists/cn_ips.txt"
    interval: 86400

rules:
  final:
    primary_upstream: "cn_dns"
    fallback_upstream: "global_dns"
    ipcidr: "cn_ips"          # 指定用于判定的列表
```

**判定原理**：
```
返回的 IP 地址 在 ipcidr 列表中?
  ├─ YES → 采用列表中的国家代码（如 CN）→ 只用 primary
  └─ NO  → IP 不在列表中 → 执行 fallback 解析
```

**列表格式**（|CIDR|国家代码|）：
```text
|8.8.8.0/24|US|
|223.5.5.0/24|CN|
|142.250.0.0/15|US|
|39.156.0.0/16|CN
**A**: 每次触发 Final 规则都会追加一条记录。

**文件增长速度**：
- 取决于未分类域名的数量
- 缓存命中率越高，增长越慢
- 建议定期清理或分析

**管理建议**：
```bash
# 定期清理或备份
mv ./output/domains.txt ./output/domains_backup_$(date +%Y%m%d).txt

# 分析后清空
cat ./output/domains.txt >> ./rules/analysis.txt
> ./output/domains.txt  # 清空文件
```

### Q5: 国家代码如何判定？

**A**: 通过 IP CIDR 列表判定。需要配置 IP CIDR 列表：

```yaml
lists:
  # IP CIDR 列表（包含国家代码）
  cn_ips:
    type: "ipcidr"
    path: "./lists/cn_ips.txt"
    interval: 86400
```

**格式示例**：
```text
|8.8.8.0/24|US|
|223.5.5.0/24|CN|
|142.250.0.0/15|US|
```

### Q6: 如何调试 Final 规则？（使用 ipcidr 列表判定）
4. **无匹配** - 返回错误

**示例**：
```yaml
lists:
  cn_ips:
    type: "ipcidr"
    path: "./lists/cn_ips.txt"

rules:
  servers:
    - listener1,dns_a    # 优先级最高
  
  main:
    - domain_list,dns_b  # 优先级次高
  
  final:                 # 优先级最低
    primary_upstream: "dns_c"
    fallback_upstream: "dns_d"
    ipcidr: "cn_ips"     # 使用 cn_ips 判定国家代码
```

**匹配流程**：
```
1. 检查 servers 规则 → 匹配? → 返回结果
2. 检查 main 规则   → 匹配? → 返回结果
3. 都未匹配         → 进入 final
4. final 规则
   ├─ 使用 primary 解析
   ├─ 对比ipcidr 字段是必须的吗？

**A**: **不是必须的**。

| 情况 | 说明 | 结果 |
|------|------|------|
| **指定 ipcidr** | 根据列表判定国家代码 | 灵活判定 |
| **未指定 ipcidr** | 使用默认判定逻辑 | 根据其他方式判定 |

**不指定 ipcidr 的配置**：
```yaml
rules:
  final:
    primary_upstream: "dns_a"
    fallback_upstream: "dns_b"
    # 不指定 ipcidr，使用默认方式
```

**指定 ipcidr 的好处**：
✅ 精确控制国家代码判定  
✅ 支持多个不同的 IP 列表  
✅ 灵活适应不同场景  

**建议**：
- 生产环境推荐指定 ipcidr
- 多个运营商场景必须指定不同的 ipcidr

### Q10: 一个 Final 规则可以使用多个 ipcidr 吗？

**A**: **不支持**。一个 Final 规则只能指定一个 ipcidr 列表。

如果需要多个判定逻辑，可以创建多个 Final 规则（如果架构支持）或使用多个主规则组。

**单 ipcidr 配置**：
```yaml
rules:
  final:
    primary_upstream: "dns_a"
    fallback_upstream: "dns_b"
    ipcidr: "cn_ips"    # 只能指定一个
```

**多场景解决方案**：
使用多个主规则组处理不同的场景：
```yaml
lists:
  # 国内 IP 列表
  cn_ips:
    type: "ipcidr"
    path: "./lists/cn_ips.txt"
  
  # 移动 IP 列表
  mobile_ips:
    type: "ipcidr"
    path: "./lists/mobile_ips.txt"

rules:
  # 主规则 1：处理已分类域名
  main:
    - cn_domains,cn_dns
  
  # Final 规则：处理未分类域名
  final:
    primary_upstream: "cn_dns"
    fallback_upstream: "global_dns"
    ipcidr: "cn_ips"  # 用国内 IP 列表判定
# 使用相对路径（相对于启动目录）
output: "./output/domains.txt"

# 或绝对路径
output: "/var/log/creskyDNS/domains.txt"
```

### Q8: Final 规则与其他规则的优先级关系？

**A**: 优先级顺序（从高到低）：

1. **Server 规则**（servers 组）- 按监听器匹配
2. **Main 规则**（main 组）- 按规则顺序匹配
3. **Final 规则** - 兜底规则
4. **无匹配** - 返回错误

**示例**：
```yaml
rules:
  servers:
    - listener1,dns_a    # 优先级最高
  
  main:
    - domain_list,dns_b  # 优先级次高
  
  final:                 # 优先级最低
    primary_upstream: "dns_c"
    fallback_upstream: "dns_d"
```

### Q9: 双层解析会影响性能吗？

**A**: 有一定影响，但可通过缓存优化：

**性能对比**：
```
无缓存:
  - 国内域名: 50ms × 1 = 50ms
  - 国外域名: 50ms × 2 = 100ms
  - 平均: 75ms

有缓存（缓存命中率 80%）:
  - 缓存命中: 1ms
  - 缓存未命中: 75ms
  - 平均: 1ms × 80% + 75ms × 20% = 16ms
```

**优化建议**：
```yaml
cache:
  main:
    size: 100000        # 增加缓存大小
    min_ttl: 300        # 增加最小 TTL
    max_ttl: 86400      # 增加最大 TTL
```

---

## 相关文档

- [RULE_MATCHING.md](RULE_MATCHING.md) - 基础规则匹配
- [CONFIG_EXAMPLES.md](CONFIG_EXAMPLES.md) - 配置文件示例
- [PROJECT_FEATURES.md](PROJECT_FEATURES.md) - 项目功能说明
- [IP_CIDR_LIST.md](IP_CIDR_LIST.md) - IP CIDR 列表说明

---

## 总结

**Final 规则**提供了强大的兜底和优化能力：

✅ **智能双层解析** - 根据地理位置自动选择上游  
✅ **动态优化** - 记录未分类域名便于后续优化  
✅ **灵活配置** - 支持灵活的上游组合  
✅ **性能平衡** - 通过缓存平衡性能和准确性  
✅ **实战价值** - 适用于多种优化场景  

**通过 Final 规则，可以构建自学习和自优化的 DNS 分流系统！** 🚀
