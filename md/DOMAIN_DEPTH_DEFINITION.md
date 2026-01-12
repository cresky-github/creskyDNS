# 域名深度定义规范

## 定义

域名深度（Domain Depth）是指域名层级的数量，从根域名开始计数。

### 计数规则

- **起点**：根域名 `.` 为深度 0
- **增长**：每增加一级域名，深度 +1
- **格式**：域名不包含前导和后导的点 `.`（除了根域名本身）

## 标准示例

| 域名 | 深度 | 说明 |
|------|------|------|
| `.` | 0 | 根域名 |
| `com` | 1 | 顶级域名 (TLD) |
| `google.com` | 2 | 二级域名 |
| `www.google.com` | 3 | 三级域名 |
| `api.www.google.com` | 4 | 四级域名 |
| `v1.api.www.google.com` | 5 | 五级域名 |

## 计算方法

对于域名字符串，深度的计算方法：

```
深度 = 点号数量 + 1
```

特殊情况：
- `.` (根域名) → 深度 = 0
- 空字符串 → 无效域名

## 实现示例

```rust
fn calculate_depth(domain: &str) -> usize {
    if domain == "." {
        return 0;
    }
    
    let parts: Vec<&str> = domain
        .split('.')
        .filter(|s| !s.is_empty())
        .collect();
    
    parts.len()
}
```

### 测试用例

```rust
assert_eq!(calculate_depth("."), 0);
assert_eq!(calculate_depth("com"), 1);
assert_eq!(calculate_depth("google.com"), 2);
assert_eq!(calculate_depth("www.google.com"), 3);
assert_eq!(calculate_depth("api.service.example.com"), 4);
```

## 在规则匹配中的应用

在 DNS 规则匹配系统中，深度用于确定匹配的优先级：

1. **深度越大，匹配越精确**
   - `www.google.com` (深度3) 比 `google.com` (深度2) 更精确
   
2. **优先选择深度大的规则**
   ```
   查询: www.google.com
   规则A: google.com (深度2)
   规则B: www.google.com (深度3)
   → 选择规则B（深度更大）
   ```

3. **深度相同时，使用其他规则排序**
   - 通常按规则定义顺序，选择最后一个匹配的规则

## 注意事项

### ✓ 正确格式
- `com` (不含点)
- `google.com`
- `www.google.com`

### ✗ 错误格式
- `.com` (前导点)
- `google.com.` (后导点)
- `.google.com.` (前后都有点)

### 特殊说明

根域名 `.` 是唯一允许的单点格式，用于匹配所有域名（通配符）。

## 相关文档

- [RULE_MATCHING.md](RULE_MATCHING.md) - 规则匹配说明
- [README.md](README.md) - 域名深度匹配流程
- [PROJECT_FEATURES.md](PROJECT_FEATURES.md) - 智能规则分流

## 版本历史

- **2026-01-12**: 创建文档，明确域名深度定义规范
