# ⚡ interval 配置快速参考

## interval 的含义

```
interval: 0      → 文件改变立即重新加载
interval: 300    → 每 5 分钟检查一次，忽略期间的文件变化
interval: 3600   → 每小时检查一次
```

## 快速选择表

| 使用场景 | 推荐 interval | 理由 |
|---------|-------------|------|
| **开发测试** | 0 | 实时看到更改 |
| **快速迭代** | 0 | 频繁修改配置 |
| **频繁更新** | 300 | 5分钟一次，避免过频 |
| **生产环境** | 1800-3600 | 30分钟到1小时，稳定性优先 |
| **静态列表** | 不设置 path | 禁用重新加载 |

## 配置示例

### 例子 1：开发模式（立即更新）

```yaml
lists:
  test:
    path: "test_domains.txt"
    interval: 0              # 立即加载
```

**效果**：
```
11:23 → 编辑文件
11:23 → 自动重新加载（< 5 秒内）
11:23 → 新配置生效
```

### 例子 2：生产模式（稳定更新）

```yaml
lists:
  production:
    path: "production_domains.txt"
    interval: 1800           # 30 分钟
```

**效果**：
```
10:00 → 首次加载
10:05 → 编辑文件（被忽略）
10:10 → 再次编辑（仍被忽略）
10:30 → 30 分钟到期，自动重新加载
```

### 例子 3：混合配置

```yaml
lists:
  # 高优先级：快速更新
  direct:
    path: "direct_domains.txt"
    interval: 0              # 立即更新
  
  # 中优先级：定期更新
  proxy:
    path: "proxy_domains.txt"
    interval: 600            # 10 分钟
  
  # 低优先级：长期稳定
  adblock:
    path: "adblock_domains.txt"
    interval: 3600           # 1 小时
```

## 工作流程图

### interval: 0（立即模式）

```
t=0:00 → 加载
         ↓
t=0:05 → 文件改变？
         yes ↓
         立即重新加载
         ↓
t=0:10 → 文件改变？
         yes ↓
         立即重新加载
```

### interval: 300（延迟模式）

```
t=0:00 → 加载（记录时间）
         ↓
t=0:05 → 文件改变？文件改变 > 5 min？
         yes, no → 忽略
         ↓
t=0:10 → 文件改变？文件改变 > 5 min？
         yes, no → 忽略
         ↓
t=5:00 → 文件改变？ 300 秒已过？
         yes, yes → 重新加载
         ↓
```

## 常见配置

### 全部立即更新
```yaml
lists:
  direct:    { interval: 0 }
  proxy:     { interval: 0 }
  adblock:   { interval: 0 }
```

### 全部延迟更新
```yaml
lists:
  direct:    { interval: 600 }    # 10 分钟
  proxy:     { interval: 1800 }   # 30 分钟
  adblock:   { interval: 3600 }   # 1 小时
```

### 推荐生产配置
```yaml
lists:
  direct:    { interval: 300 }    # 5 分钟
  proxy:     { interval: 600 }    # 10 分钟
  adblock:   { interval: 1800 }   # 30 分钟
  custom:    { interval: 0 }      # 立即
```

## 计时器重置规则

| 情况 | 结果 |
|------|------|
| 文件改变 → 启动倒计时 | pending_update=true，记录文件改变时间 |
| interval 期间文件再次改变 | **忽略**，倒计时继续（不重新启动） |
| interval 倒计时归 0 | 执行 reload，清除 pending_update |
| interval 期间无文件改变 | 倒计时正常进行 |

**关键规则**：
- 计时起点 = 文件首次改变时刻
- interval 期间不重新触发计时器
- 只有倒计时归 0 才 reload

## 监视间隔说明

⚠️ **重要**：应用每 **5 秒** 检查一次文件变化

```
t=0:00  → 检查
t=0:05  → 检查
t=0:10  → 检查
t=0:15  → 检查
...
```

这意味着：
- 文件改变到被检测到：最多延迟 5 秒
- 即使 `interval: 0`，也不会立即（会在 5 秒内）重新加载

## 性能影响

### 低开销（interval >= 600）
- 每10分钟最多一次重新加载
- 适合生产环境
- 推荐大文件列表

### 中等开销（interval: 300）
- 每5分钟最多一次重新加载
- 平衡更新和性能
- 推荐中等列表

### 高开销（interval: 0）
- 文件改变立即重新加载
- 适合开发环境
- 不推荐大文件列表

## 故障排除

### Q: 为什么改变没有立即生效？

**A**: 即使配置 `interval: 0`，也需要：
1. 应用检测到文件改变（5秒内）
2. 读取文件（毫秒级）
3. 更新内存（毫秒级）

总延迟：0-5 秒

### Q: interval 应该设置多大？

**A**: 看你的文件更新频率：
- 频繁更新（> 1次/分钟）→ `interval: 0`
- 定期更新（1-5次/小时）→ `interval: 300-600`
- 稀少更新（< 1次/小时）→ `interval: 1800+`

### Q: 可以动态改变 interval 吗？

**A**: 目前不支持（需要重启应用）。计划未来版本添加热配置更新。

## 最佳实践 3 点

### 1️⃣ 开发时用 0，生产用更大的值

```yaml
# 开发配置
lists:
  test: { interval: 0 }

# 生产配置
lists:
  test: { interval: 3600 }
```

### 2️⃣ 根据文件大小调整

```yaml
# 小文件（< 1KB）
lists:
  small: { interval: 0 }

# 中等文件（1-100KB）
lists:
  medium: { interval: 300 }

# 大文件（> 100KB）
lists:
  large: { interval: 1800 }
```

### 3️⃣ 根据重要性调整

```yaml
# 关键列表
lists:
  critical: { interval: 0 }

# 普通列表
lists:
  normal: { interval: 600 }

# 备用列表
lists:
  backup: { interval: 3600 }
```

---

**完整文档**: [LIST_RELOAD.md](LIST_RELOAD.md)
