# 规则匹配说明

## 规则组 (Rule Group) 工作原理

### 基本结构
```yaml
rules:
  group_name:        # 规则组名称（按定义顺序处理）
    - list1,upstream1
    - list2,upstream2
    - list3,upstream3
```

### 匹配流程

#### 第一步：按 YAML 顺序逐一检查规则组
按配置文件中**定义的顺序**，逐个检查每个规则组。

> **关键点：一旦某个规则组有匹配，立即返回结果，不再检查后续规则组！**

```
规则组遍历过程：
group1 → 检查是否有匹配
    ↓
  有匹配? 
    ↓ 是
  【返回结果】← 不再检查 group2, group3, ...
    ↓ 否
  继续到 group2
group2 → 检查是否有匹配
    ↓
  有匹配?
    ↓ 是
  【返回结果】← 不再检查 group3, ...
    ↓ 否
  继续到 group3
...
```

#### 第二步：在规则组内同时匹配所有规则
对于当前规则组，同时对所有规则进行评估，查找匹配的规则。

**匹配方式：Domain Suffix（域名后缀匹配）**
- 支持多级域名匹配
- 例如：配置了 `example.com`，可匹配：
  - `example.com` ✓
  - `www.example.com` ✓
  - `api.service.example.com` ✓
  - `other.example.org` ✗

#### 第三步：规则组内的优先级排序

对于所有匹配的规则，按以下优先级排序：

**1. 域名深度 (Domain Depth)** - 最重要
   - 深度 = 匹配的域名层级数
   - 深度大的优先（更具体的匹配）
   
   例如对于 `api.service.example.com`：
   - `example.com` 的深度 = 1
   - `service.example.com` 的深度 = 2
   - `api.service.example.com` 的深度 = 3
   - **选择深度 3 的规则**

**2. 规则顺序 (Rule Index)** - 次要
   - 当多个规则深度相同时
   - **选择在列表中最后出现的规则**
   
   例如：
   ```yaml
   rules:
     group:
       - list1,upstream1  # index=0, depth=2
       - list2,upstream2  # index=1, depth=2 ← 选中！
   ```

#### 第四步：返回结果
返回匹配规则对应的上游 DNS 服务器配置。

---

## 优先级说明

规则匹配的优先级（由高到低）：

| 优先级 | 规则 | 说明 |
|--------|------|------|
| **1** | **YAML 规则组顺序** | 按配置文件定义顺序逐一检查，**第一个有匹配的规则组被使用，其余规则组被忽略** |
| 2 | 域名深度 | 规则组内，深度大的规则优先（更精确匹配） |
| 3 | 规则顺序 | 深度相同时，选择列表中最后的规则 |

### ⚠️ 关键特性
- **短路逻辑**：一旦某个 group 有匹配，立即返回，不再检查后续 group
- ❌ 规则组名称**不影响**处理顺序（如 `a_alpha`, `z_zebra` 不决定顺序）
- ✓ 规则组**在配置文件中的位置**决定了处理顺序
- ✓ 这样设计的好处：可以设置优先级最高的规则组在最前面，低优先级的在后面

---

## 实例演示

### 配置示例

```yaml
lists:
  direct:
    domains:
      - baidu.com
      - qq.com
      - aliyun.com
  
  proxy:
    domains:
      - google.com
      - github.com
  
  custom:
    domains:
      - internal.company.com

upstreams:
  ali_doh:
    addr: "https://dns.alidns.com/dns-query"
  
  google_doq:
    addr: "quic://dns.google:784"
  
  cloudflare_dot:
    addr: "tls://1.1.1.1:853"

rules:
  # 规则组 1：国内规则（最先检查）
  domestic:
    - direct,ali_doh          # 规则0：直连 → 阿里DoH
    - custom,cloudflare_dot   # 规则1：自定义 → Cloudflare DoT
  
  # 规则组 2：国际规则（后检查）
  international:
    - proxy,google_doq        # 规则0：代理 → Google DoQ
```

### 查询案例分析

#### 案例 1：精确匹配 + 深度优先

**查询**：`api.aliyun.com`

**规则组 `domestic` 的评估**：
- 规则0 `direct,ali_doh`：
  - `baidu.com` ✗ 不匹配
  - `qq.com` ✗ 不匹配  
  - `aliyun.com` ✓ 匹配，深度=1
  
- 规则1 `custom,cloudflare_dot`：
  - `internal.company.com` ✗ 不匹配

**组内结果**：选择规则0 (aliyun.com, 深度=1)

**返回**：`ali_doh` (https://dns.alidns.com/dns-query)

---

#### 案例 2：深度相同 + 后者优先

**查询**：`internal.company.com`

假设配置改为：
```yaml
rules:
  domestic:
    - direct,ali_doh           # 规则0：如果 direct 包含 internal.company.com
    - custom,cloudflare_dot    # 规则1：custom 也包含 internal.company.com
```

**规则组 `domestic` 的评估**：
- 规则0：深度=1，index=0
- 规则1：深度=1，index=1 ← **选中**（深度相同，后者优先）

**返回**：`cloudflare_dot` (tls://1.1.1.1:853)

---

#### 案例 3：多规则组 + 第一组优先

**查询**：`google.com`

**规则处理顺序**（完全按 YAML 定义顺序）：

1. **规则组 `domestic`** (第1个)：
   - 所有规则都不匹配 ✗
   - 继续到下一个规则组

2. **规则组 `international`** (第2个)：
   - 规则0 `proxy,google_doq`：
     - `google.com` ✓ 匹配，深度=1

**返回**：`google_doq` (quic://dns.google:784)

> 即使 `google.com` 在其他规则组中有匹配，也不会使用，因为按 YAML 顺序，`domestic` 被优先检查（如果有匹配就返回）。

---

## YAML 顺序示例

### 示例 1：规则组名称不决定顺序

```yaml
rules:
  z_zebra:          # ← 第1个检查（尽管名字排在最后）
    - direct,upstream1
  
  a_alpha:          # ← 第2个检查（尽管名字排在最前）
    - proxy,upstream2
  
  m_middle:         # ← 第3个检查
    - special,upstream3
```

**处理顺序**：`z_zebra` → `a_alpha` → `m_middle`（**不是**字母顺序！）

### 示例 2：按优先级排列

```yaml
rules:
  # 第1优先：公司内部（最特殊）
  internal_trust:
    - internal,internal_dns
  
  # 第2优先：安全防护
  security:
    - ads,blocker
    - malware,blocker
  
  # 第3优先：国内加速
  domestic:
    - cn_fast,local_dns
  
  # 第4优先：国际转发
  international:
    - global,overseas_dns
```

**处理顺序**：`internal_trust` → `security` → `domestic` → `international`

---

## 最佳实践

### ✓ 推荐

---

## 短路逻辑示例 - 关键特性

### 场景：多个 group 都有匹配，但只用第一个

```yaml
rules:
  # Group 1
  security:
    - ads,blocker
    - malware,blocker
  
  # Group 2
  domestic:
    - direct,local_dns
  
  # Group 3
  international:
    - proxy,overseas_dns
```

### 查询示例：`ads.example.com`

**执行过程：**

```
1. 检查 security group
   ├─ ads list 中有 ads.example.com? ✓ 是！
   ├─ 返回 blocker upstream
   └─ 【停止】← 不再检查 domestic 和 international group
   
2. domestic group 被跳过 ✗
3. international group 被跳过 ✗
```

**结果**：使用 `blocker` 上游，尽管 `direct` 和 `proxy` 列表中也可能有匹配

---

### 查询示例：`baidu.com`（假设只在 domestic 有匹配）

**执行过程：**

```
1. 检查 security group
   ├─ ads 中有 baidu.com? ✗
   ├─ malware 中有 baidu.com? ✗
   └─ 继续到下一个 group
   
2. 检查 domestic group
   ├─ direct 中有 baidu.com? ✓ 是！
   ├─ 返回 local_dns upstream
   └─ 【停止】← 不再检查 international group
   
3. international group 被跳过 ✗
```

**结果**：使用 `local_dns` 上游，忽略 international group 中的配置

---

### 查询示例：`google.com`（假设只在 international 有匹配）

**执行过程：**

```
1. 检查 security group
   ├─ ads 中有 google.com? ✗
   ├─ malware 中有 google.com? ✗
   └─ 继续到下一个 group
   
2. 检查 domestic group
   ├─ direct 中有 google.com? ✗
   └─ 继续到下一个 group
   
3. 检查 international group
   ├─ proxy 中有 google.com? ✓ 是！
   ├─ 返回 overseas_dns upstream
   └─ 【完成】
```

**结果**：使用 `overseas_dns` 上游

---

1. **按优先级排列规则组**（重要的在前面）
   ```yaml
   rules:
     # 第1层：最高优先级（精确控制）
     exceptions:
       - special_case,special_upstream
     
     # 第2层：高优先级（安全）
     security:
       - ads,blocker
     
     # 第3层：中优先级（分类）
     domestic:
       - direct,local_dns
     
     # 第4层：低优先级（默认）
     international:
       - proxy,overseas_dns
   ```

2. **在规则组内按特殊性排序**
   ```yaml
   rules:
     domain_rules:
       - special_list,special_upstream    # 最特殊
       - general_list,general_upstream    # 通用
   ```

3. **避免冲突**
   - 同一规则组内避免多个列表包含相同域名
   - 或故意利用深度和顺序差异来控制优先级

### ✗ 需要注意

1. **第一匹配原则**
   - 一旦某个规则组有匹配，就不再检查后续规则组
   - 确保规则组顺序符合预期

2. **深度限制**
   - 最深支持的域名层级由列表定义决定
   - 过多的子域名不会增加深度

3. **无匹配时的行为**
   - 如果查询的域名在所有规则组都不匹配，会返回错误
   - 可添加 wildcard 规则处理默认情况

---

## 配置建议

### 基础配置
```yaml
rules:
  default:
    - all_domains,preferred_upstream
```

### 分类配置
```yaml
rules:
  security:
    - ads,blocker_upstream
    - malware,blocker_upstream
  
  domestic:
    - cn_domains,local_upstream
  
  international:
    - us_domains,us_upstream
    - eu_domains,eu_upstream
```

### 高级配置
```yaml
rules:
  custom:
    - company_internal,internal_dns
    - company_trusted,trusted_upstream
  
  secure:
    - sensitive_domains,encrypted_upstream
  
  fast:
    - frequently_used,fast_upstream
  
  standard:
    - general_domains,standard_upstream
```

---

## 工作流程图

```
输入：域名查询

↓

遍历规则组（按顺序）
├─ 规则组1：domestic
│  ├─ 同时评估所有规则
│  ├─ 筛选匹配规则
│  ├─ 按深度排序
│  └─ 若有匹配 → 返回结果 ✓
│
├─ 规则组2：international
│  ├─ 同时评估所有规则
│  └─ ...
│
└─ 其他规则组
   └─ ...

↓

返回上游配置 / 错误
```
