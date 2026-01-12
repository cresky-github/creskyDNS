# 🔄 域名列表热重新加载功能

## 概述

DNS 转发器支持域名列表的热重新加载功能，可以在不重启应用的情况下更新域名列表。

## interval 字段说明

在配置文件中，每个域名列表都可以配置 `interval` 字段：

```yaml
lists:
  direct:
    type: "domain"
    format: "text"
    path: "direct_domains.txt"
    interval: 0              # 重新加载间隔（秒）
    domains: []
```

### interval 规则

| interval 值 | 行为 | 使用场景 |
|-----------|------|----------|
| `0` 或不配置 | 文件改变后**立即**重新加载最新的文件 | 需要实时更新，频繁修改 |
| `> 0` | 文件改变时启动倒计时，interval 期间**无视后续文件改变**，倒计时归 0 时执行 reload | 文件频繁变化，避免频繁重新加载 |

**关键规则**：
- **计时起点**：文件改变时刻（不是上次加载时刻）
- **倒计时期间**：忽略任何新的文件改变，不重新触发计时器
- **执行时机**：倒计时归 0 时才执行 reload

## 工作原理

### 启动流程

```
应用启动
  ↓
加载初始配置和域名列表
  ↓
初始化重新加载状态（记录首次加载时间和文件修改时间）
  ↓
启动后台监视任务
```

### 监视流程（每 5 秒执行一次）

```
检查每个域名列表配置
  ↓
如果没有指定 path，跳过
  ↓
检查文件是否被修改
  ├─ 未修改：跳过
  └─ 已修改：
      ├─ interval == 0：立即重新加载
      └─ interval > 0：
          ├─ 无待处理更新：标记 pending_update=true，记录文件改变时间，启动倒计时
          ├─ 有待处理更新：检查 (当前时间 - 文件改变时间) >= interval
          │   ├─ 是：执行 reload，清除 pending_update
          │   └─ 否：跳过，继续等待（无视新的文件改变）
          └─ 在 interval 倒计时期间，任何新的文件改变都被忽略
  ↓
更新状态（修改时间、加载时间、待处理标记）
```

## 配置示例

### 示例 1：立即更新（interval: 0）

```yaml
lists:
  direct:
    type: "domain"
    format: "text"
    path: "direct_domains.txt"
    interval: 0              # 文件改变立即加载
    domains:
      - baidu.com
      - qq.com
```

**行为**：
- 修改 `direct_domains.txt`
- 应用在下一个检查周期（5秒内）检测到变化
- **立即**重新加载文件，应用新的域名列表

### 示例 2：间隔更新（interval: 300）

```yaml
lists:
  adblock:
    type: "domain"
    format: "text"
    path: "adblock_domains.txt"
    interval: 300            # 每 300 秒（5 分钟）最多更新一次
    domains:
      - ads.example.com
```

**行为**：
- **时间 00:00**：应用启动，首次加载
- **时间 00:02**：修改文件 → 触发！标记 pending_update=true，记录改变时间 00:02，启动 300 秒倒计时
- **时间 00:04**：再次修改文件 → 被忽略（倒计时继续，不重置）
- **时间 01:00**：再次修改文件 → 被忽略（倒计时继续）
- **时间 05:02**：倒计时结束（距 00:02 已过 300 秒）→ 执行 reload，清除 pending_update
- **时间 05:10**：修改文件 → 重新触发，标记 pending_update=true，记录改变时间 05:10

**关键理解**：
- interval 的计时起点是**文件改变时刻**（00:02），不是首次加载时刻（00:00）
- 在倒计时期间（00:02 - 05:02），任何文件改变都被忽略
- 只有倒计时归 0 时才执行 reload

### 示例 3：长间隔更新（interval: 3600）

```yaml
lists:
  custom:
    type: "domain"
    format: "text"
    path: "custom_domains.txt"
    interval: 3600           # 每小时最多更新一次
```

**场景**：
- 此列表对应的是相对稳定的用户自定义域名
- 即使文件经常被编辑，也只有每小时检查一次
- 减少频繁重新加载的开销

## 日志输出

### 成功重新加载

```
2024-01-15 12:34:56 INFO  域名列表 'direct' 已重新加载: 45 个域名
```

### 重新加载失败

```
2024-01-15 12:34:56 ERROR 域名列表 'direct' 重新加载失败: 文件不存在
```

### 总体完成

```
2024-01-15 12:34:56 INFO  域名列表已更新，重新加载完成
```

## 最佳实践

### 1. 快速迭代开发

如果正在调整域名列表配置，使用 `interval: 0`：

```yaml
lists:
  test:
    path: "test_domains.txt"
    interval: 0              # 立即看到变化
```

### 2. 生产环境稳定性

使用较大的 `interval` 值，避免频繁重新加载导致的性能抖动：

```yaml
lists:
  production:
    path: "production_domains.txt"
    interval: 1800           # 每 30 分钟检查一次
```

### 3. 自动化更新脚本

配合定时任务和 `interval: 0`，实现自动更新：

```bash
# crontab 配置：每小时更新一次
0 * * * * /path/to/update_domains.sh

# update_domains.sh
wget -O /app/direct_domains.txt https://example.com/domains.txt
# 配置 interval: 0 会立即加载新列表
```

## 性能考虑

### 监视开销

- 后台监视任务每 5 秒执行一次
- 每次检查所有配置的列表
- 只有文件修改时才进行文件读取
- 内存占用固定，与重新加载频率无关

### 重新加载开销

- 文件读取时间：取决于文件大小（通常 < 100ms）
- 内存更新：原子操作，无阻塞
- 对 DNS 查询没有影响（使用 RwLock 读写分离）

### 优化建议

| 列表大小 | 建议 interval | 理由 |
|---------|-------------|------|
| < 1,000 条 | 0 | 重新加载开销小 |
| 1,000-10,000 条 | 300-600 | 平衡更新和性能 |
| > 10,000 条 | 1800+ | 减少重新加载频率 |

## 常见问题

### Q: 重新加载时 DNS 查询会中断吗？

**A**: 不会。重新加载使用 RwLock，DNS 查询是读操作，可以并行执行。只有在进行重新加载（写操作）时才会略微增加延迟。

### Q: interval: 0 和没有 interval 字段有区别吗？

**A**: 没有。默认值就是 0，表示立即重新加载。

### Q: 如何禁用自动重新加载？

**A**: 不配置 `path` 字段，只使用 `domains` 的静态列表：

```yaml
lists:
  static:
    type: "domain"
    format: "text"
    domains:
      - example.com
    # 没有 path 字段，不会尝试加载文件
```

### Q: 可以手动触发重新加载吗？

**A**: 目前没有手动触发的 API。计划在后续版本添加 HTTP 接口用于手动触发。

### Q: 重新加载失败会怎样？

**A**: 
- 记录错误日志
- 保留原有的域名列表（不丢失）
- 继续监视，等待下一个 interval 周期重试

## 相关配置

### config.yaml 完整示例

```yaml
# 域名列表配置 (name -> config)
lists:
  # 直连域名列表（立即更新）
  direct:
    type: "domain"
    format: "text"
    path: "direct_domains.txt"
    interval: 0              # 文件改变立即加载
    domains:
      - baidu.com
      - qq.com

  # 代理域名列表（30分钟更新一次）
  proxy:
    type: "domain"
    format: "text"
    path: "proxy_domains.txt"
    interval: 1800           # 30 分钟
    domains:
      - google.com
      - facebook.com

  # 广告屏蔽列表（每小时更新一次）
  adblock:
    type: "domain"
    format: "text"
    path: "adblock_domains.txt"
    interval: 3600           # 1 小时
    domains:
      - ads.example.com

  # 自定义列表（5分钟更新一次）
  custom:
    type: "domain"
    format: "text"
    path: "custom_domains.txt"
    interval: 300            # 5 分钟
```

## 故障排除

### 问题：列表没有更新

**检查清单**：
1. 文件路径是否正确？
2. 文件是否存在？
3. 应用是否有读权限？
4. 查看日志是否有错误信息？

### 问题：频繁重新加载

**解决方案**：
- 增加 `interval` 值
- 检查是否有外部进程频繁修改文件
- 使用文件锁防止并发修改

### 问题：性能下降

**优化方案**：
- 增加 `interval` 值减少重新加载频率
- 使用多个较小的列表文件而不是一个大文件
- 定期清理不使用的域名

## 技术细节

### 重新加载状态结构

```rust
pub struct DomainListReloadState {
    /// 最后一次文件修改时间戳
    pub last_modified: u64,
    /// 最后一次加载时间戳
    pub last_loaded: u64,
    /// 是否有待处理的更新
    pub pending_update: bool,
}
```

### 监视任务执行流程

```rust
loop {
    sleep(5 seconds)
    
    for each domain_list in config.lists {
        if list.path.is_none() continue
        
        current_mtime = get_file_modified_time()
        
        if current_mtime > last_modified {
            if interval == 0 {
                reload()
            } else if (now - last_loaded) >= interval {
                reload()
            }
        }
    }
}
```

## 版本信息

- **添加版本**: v1.1+
- **稳定版本**: v1.1.0+
- **向后兼容**: 是（interval 默认为 0）

---

**相关文档**:
- [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) - 域名列表格式说明
- [README.md](README.md) - 项目主文档
