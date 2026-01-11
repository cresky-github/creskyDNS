# 问题修复：域名未匹配规则时的默认行为

## 问题描述

**日志错误**：
```
ERROR creskyDNS: UDP 监听器 'main-udp' 处理查询失败 [127.0.0.1:58482]: 域名 jd.com. 未匹配到任何规则
```

**问题分析**：
- 用户查询 `jd.com` 域名
- 系统有两个域名列表：`direct` (2个域名) 和 `proxy` (2个域名)
- `jd.com` 不在这两个列表中
- 代码在未匹配到规则时直接报错，而不是使用默认上游

## 修复方案

### 修改文件
[src/forwarder.rs](src/forwarder.rs#L183-L225)

### 修改内容

**原逻辑**：
```rust
fn match_domain_rules(&self, domain: &str) -> Result<(&UpstreamList, String)> {
    // 遍历所有规则组
    for (group_name, rules) in &self.config.rules {
        if let Some(upstream_list) = self.find_best_match_in_group(domain, rules) {
            // 找到匹配，返回
            return Ok((upstream, rule_name));
        }
    }

    // ❌ 没有匹配时直接报错
    anyhow::bail!("域名 {} 未匹配到任何规则", domain)
}
```

**新逻辑**：
```rust
fn match_domain_rules(&self, domain: &str) -> Result<(&UpstreamList, String)> {
    // 遍历所有规则组（跳过 'final' 规则组）
    for (group_name, rules) in &self.config.rules {
        if group_name == "final" {
            continue;  // final 规则组预留给未来实现
        }
        
        if let Some(upstream_list) = self.find_best_match_in_group(domain, rules) {
            // 找到匹配，返回
            return Ok((upstream, rule_name));
        }
    }

    // ✅ 没有匹配时，使用默认上游（多级降级）
    // 1. 优先使用 default_dns
    // 2. 然后尝试 cn_dns、direct_dns、global_dns
    // 3. 最后使用任何可用的第一个上游
    let default_upstream_names = vec!["default_dns", "cn_dns", "direct_dns", "global_dns"];
    
    for upstream_name in default_upstream_names {
        if let Some(upstream) = self.config.upstreams.get(upstream_name) {
            debug!("域名 {} 未匹配任何规则，使用默认上游 '{}'", domain, upstream_name);
            let rule_name = format!("default:{}", upstream_name);
            return Ok((upstream, rule_name));
        }
    }
    
    // 使用第一个可用的上游
    if let Some((name, upstream)) = self.config.upstreams.iter().next() {
        debug!("域名 {} 未匹配任何规则，使用第一个可用上游 '{}'", domain, name);
        let rule_name = format!("fallback:{}", name);
        return Ok((upstream, rule_name));
    }

    // 只有在完全没有上游时才报错
    anyhow::bail!("域名 {} 未匹配到任何规则，且没有可用的默认上游", domain)
}
```

## 修复效果

### 修复前
```
查询 jd.com
  ↓
检查 direct 列表 → 不匹配
  ↓
检查 proxy 列表 → 不匹配
  ↓
❌ 报错：域名 jd.com. 未匹配到任何规则
```

### 修复后
```
查询 jd.com
  ↓
检查 direct 列表 → 不匹配
  ↓
检查 proxy 列表 → 不匹配
  ↓
✅ 使用默认上游 default_dns (223.5.5.5)
  ↓
正常返回 DNS 查询结果
```

## 默认上游优先级

系统会按以下顺序寻找默认上游：

1. **default_dns** (如果配置) - 最高优先级
2. **cn_dns** (如果配置) - 国内 DNS
3. **direct_dns** (如果配置) - 直连 DNS
4. **global_dns** (如果配置) - 国际 DNS
5. **第一个可用上游** - 兜底方案
6. **报错** - 只有完全没有上游时才会报错

## 配置建议

### 推荐配置
```yaml
upstreams:
  # 推荐：明确定义 default_dns
  default_dns:
    addresses:
      - "udp://223.5.5.5:53"     # 阿里云 DNS
      - "udp://119.29.29.29:53"  # 腾讯 DNS
  
  cn_dns:
    addresses:
      - "https://dns.alidns.com/dns-query"
  
  global_dns:
    addresses:
      - "https://dns.google/dns-query"

rules:
  main:
    - china_domains,cn_dns
    - global_domains,global_dns
  
  # 未匹配的域名会自动使用 default_dns
```

### 简化配置（自动降级）
```yaml
upstreams:
  cn_dns:
    addresses:
      - "udp://223.5.5.5:53"
  
  proxy_dns:
    addresses:
      - "udp://1.1.1.1:53"

rules:
  main:
    - direct,cn_dns
    - proxy,proxy_dns

# 未匹配的域名会自动使用 cn_dns（因为它排在前面）
```

## 日志示例

### 匹配到规则
```
DEBUG creskyDNS: 域名 google.com 在规则组 'main' 中匹配到上游 'direct_dns'
```

### 使用默认上游
```
DEBUG creskyDNS: 域名 jd.com 未匹配任何规则，使用默认上游 'default_dns'
```

### 使用降级上游
```
DEBUG creskyDNS: 域名 taobao.com 未匹配任何规则，使用默认上游 'cn_dns'
```

### 使用兜底上游
```
DEBUG creskyDNS: 域名 example.com 未匹配任何规则，使用第一个可用上游 'direct_dns'
```

## 测试验证

修复后，使用以下命令测试：

```bash
# 启动 DNS 服务器
./creskyDNS

# 测试未在列表中的域名
nslookup jd.com 127.0.0.1 -port=5353
nslookup taobao.com 127.0.0.1 -port=5353
nslookup qq.com 127.0.0.1 -port=5353

# 预期结果：所有查询都能正常返回，不会报错
```

查看日志确认使用了默认上游：
```bash
grep "未匹配任何规则" logs/creskyDNS.log
```

## 兼容性

- ✅ 向后兼容：现有配置无需修改
- ✅ 智能降级：自动选择最合适的默认上游
- ✅ 灵活配置：支持显式指定 default_dns
- ✅ 保留扩展：为 'final' 规则组预留了实现空间

---

**修复日期**: 2026年1月11日  
**相关问题**: 域名未匹配到任何规则时报错  
**影响范围**: 所有未在域名列表中的查询
