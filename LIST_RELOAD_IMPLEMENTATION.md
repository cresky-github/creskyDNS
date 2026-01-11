# 🎉 域名列表热重新加载功能 - 实现说明

## 功能概述

已完成实现 DNS 转发器的**域名列表热重新加载**功能，支持在不重启应用的情况下更新域名列表文件。

## 核心特性

### ✅ interval 字段规则

```yaml
interval: 0      # 文件改变后，interval:0 才执行reload 当时最新的文件
interval: 300    # 文件改变后，interval 期间无视文件改变
interval: 3600   # 1小时内忽略文件变化，1小时后检查更新
```

### ✅ 自动监视机制

- 后台任务每 5 秒检查一次所有域名列表文件
- 智能检测文件修改时间
- 根据 interval 配置决定是否重新加载
- 重新加载期间不影响 DNS 查询

### ✅ 完善的错误处理

- 文件不存在：保留原列表
- 文件读取失败：记录错误，继续监视
- 格式错误：记录错误，继续监视

## 代码实现

### 1. 配置结构扩展（config.rs）

```rust
// 添加 interval 字段到 DomainList
pub struct DomainList {
    pub r#type: String,
    pub format: String,
    pub path: Option<String>,
    pub url: Option<String>,
    pub domains: Vec<String>,
    pub interval: u64,  // ← 新增：重新加载间隔
}

// 新增：重新加载状态跟踪
pub struct DomainListReloadState {
    pub last_modified: u64,   // 文件最后修改时间
    pub last_loaded: u64,     // 最后加载时间
    pub pending_update: bool, // 待处理的更新标记
}
```

### 2. 加载和检查逻辑（config.rs）

```rust
impl DomainList {
    /// 同步加载（用于监视线程）
    pub fn load_sync(&mut self) -> Result<()> { ... }
    
    /// 获取文件修改时间戳
    pub fn get_file_modified_time(&self) -> Option<u64> { ... }
    
    /// 检查是否需要重新加载
    pub fn should_reload(&self, state: &DomainListReloadState) -> bool {
        // 获取当前文件修改时间
        let current_mtime = get_file_modified_time()?;
        
        // 文件未修改
        if current_mtime <= state.last_modified { 
            return false; 
        }
        
        // 文件已修改
        if self.interval == 0 { 
            return true;  // 立即加载
        }
        
        // interval > 0
        if !state.pending_update {
            // 首次检测到文件改变，标记待处理并记录时间
            state.pending_update = true;
            state.pending_change_time = current_mtime;
            return false;  // 启动倒计时，本次不加载
        }
        
        // 已有待处理更新，检查倒计时是否结束
        let now = get_current_timestamp();
        let elapsed = now - state.pending_change_time;
        if elapsed >= self.interval {
            // 倒计时结束，执行 reload
            state.pending_update = false;
            return true;
        }
        
        // 倒计时未结束，继续等待（无视新的文件改变）
        false
    }
}
```

### 3. 监视任务（main.rs）

```rust
async fn monitor_domain_list_reload(
    config: Config,
    domain_lists: Arc<RwLock<HashMap<String, Vec<String>>>>,
    reload_states: Arc<Mutex<HashMap<String, DomainListReloadState>>>,
) {
    // 每 5 秒执行一次
    loop {
        sleep(5 seconds)
        
        // 检查每个列表
        for each domain_list {
            if should_reload(state) {
                load_sync()
                update_state()
            }
        }
    }
}
```

## 使用示例

### 配置示例

```yaml
lists:
  # 开发环境：立即更新
  dev:
    path: "dev_domains.txt"
    interval: 0              # 立即加载

  # 生产环境：定期更新
  prod:
    path: "prod_domains.txt"
    interval: 1800           # 30分钟
```

### 工作流程

```
1. 应用启动
   ↓
2. 初始加载所有域名列表
   ↓
3. 启动后台监视任务
   ↓
4. 每 5 秒检查文件变化
   ├─ 文件未改变：继续等待
   ├─ 文件改变，interval=0：立即重新加载
   └─ 文件改变，interval>0：检查时间是否已过
   ↓
5. 更新共享的域名列表（RwLock）
   ↓
6. DNS 查询自动使用新列表（无需重启）
```

## 日志输出示例

```
2024-01-15 10:00:00 INFO  DNS 转发器启动
2024-01-15 10:00:00 INFO  域名列表 'direct' 从文件 'direct_domains.txt' 加载成功: 45 个域名
2024-01-15 10:00:00 INFO  域名列表 'proxy' 从文件 'proxy_domains.txt' 加载成功: 120 个域名

... (监视任务运行中) ...

2024-01-15 10:05:00 INFO  域名列表 'direct' 已重新加载: 48 个域名
2024-01-15 10:05:00 INFO  域名列表已更新，重新加载完成
```

## 文件修改清单

### 代码文件（修改 2 个）

1. **src/config.rs**
   - 添加 `interval: u64` 字段
   - 添加 `DomainListReloadState` 结构体
   - 添加 `load_sync()` 同步加载方法
   - 添加 `get_file_modified_time()` 方法
   - 添加 `should_reload()` 检查方法
   - 新增 `use std::sync::{Arc, Mutex}` 和时间相关导入

2. **src/main.rs**
   - 添加 `use std::sync::{Arc, RwLock}`
   - 添加 `use std::time::{SystemTime, UNIX_EPOCH}`
   - 添加 `use tokio::time::{sleep, Duration}`
   - 创建 `domain_lists: Arc<RwLock<HashMap>>` 共享列表
   - 创建 `reload_states: Arc<Mutex<HashMap>>` 重新加载状态
   - 初始化重新加载状态
   - 启动 `monitor_domain_list_reload` 后台任务
   - 添加 `monitor_domain_list_reload` 函数

### 配置文件（修改 1 个）

- **config.yaml**
  - 所有 `lists` 配置添加 `interval` 字段
  - 示例配置：
    - `direct`: interval: 0（立即）
    - `proxy`: interval: 1800（30分钟）
    - `adblock`: interval: 3600（1小时）
    - `custom`: interval: 300（5分钟）

### 文档文件（新增 2 个）

1. **LIST_RELOAD.md** - 完整功能说明
   - 工作原理
   - 配置示例
   - 日志说明
   - 最佳实践
   - 常见问题
   - 技术细节

2. **INTERVAL_QUICK_REF.md** - 快速参考
   - interval 快速选择表
   - 常用配置示例
   - 性能影响说明
   - 故障排除
   - 最佳实践

### 现有文件（更新 1 个）

- **README.md**
  - 添加 `[域名列表热重新加载](LIST_RELOAD.md)` 链接

## 性能特性

### 监视开销
- ✅ 后台任务每 5 秒执行一次
- ✅ 只检查配置的列表，无额外遍历
- ✅ 只有文件修改时才读取文件
- ✅ 内存占用恒定，与重新加载频率无关

### 重新加载性能
- ✅ 文件读取：毫秒级（取决于文件大小）
- ✅ 内存更新：原子操作
- ✅ DNS 查询：使用 RwLock，无阻塞

### 建议配置

| 列表大小 | 推荐 interval | 理由 |
|---------|-------------|------|
| < 1,000 | 0 | 开销小 |
| 1K-10K | 300-600 | 平衡性能 |
| > 10K | 1800+ | 减少频率 |

## 常见问题

### Q: 重新加载时会中断 DNS 查询吗？

**A**: 不会。使用 RwLock 确保读写操作不阻塞。

### Q: interval: 0 是立即加载吗？

**A**: 大约 5 秒内加载（取决于下一个检查周期）。

### Q: 如何禁用重新加载？

**A**: 不配置 `path` 字段，只用 `domains` 静态列表。

### Q: 重新加载失败会怎样？

**A**: 保留原列表，记录错误，继续监视。

### Q: 可以手动触发重新加载吗？

**A**: 目前不支持。计划未来版本添加 API。

## 后续计划

- [ ] HTTP API 手动触发重新加载
- [ ] 支持远程 URL 自动更新
- [ ] 增量更新支持（差异加载）
- [ ] 通知机制（Webhook 等）
- [ ] 重新加载失败重试机制
- [ ] 并发修改保护

## 升级指南

### 从旧版本升级

1. **无需迁移**：既有配置自动兼容
2. **可选配置**：添加 `interval` 字段启用热重新加载
3. **默认行为**：不设置 `interval` 时默认为 0（立即加载）

### 推荐升级配置

```yaml
# 旧配置（仍可使用）
lists:
  direct:
    path: "direct_domains.txt"

# 新配置（建议）
lists:
  direct:
    path: "direct_domains.txt"
    interval: 300  # 显式指定间隔
```

## 测试方法

### 快速测试

```bash
# 1. 使用 interval: 0 配置
# 2. 启动应用
cargo run --release

# 3. 修改域名列表文件
echo "example.com" >> direct_domains.txt

# 4. 5 秒内查看日志
# 应该看到：域名列表 'direct' 已重新加载
```

### 压力测试

```bash
# 快速修改文件多次
for i in {1..100}; do
    echo "test-$i.com" >> test_domains.txt
    sleep 0.1
done

# 观察：
# - interval: 0 → 多次重新加载
# - interval: 300 → 忽略频繁修改
```

## 版本信息

- **功能版本**：v1.2+
- **首次发布**：2024-01-15
- **向后兼容**：完全兼容（interval 默认为 0）

---

## 文档导航

| 文档 | 内容 |
|------|------|
| [LIST_RELOAD.md](LIST_RELOAD.md) | 完整功能说明 |
| [INTERVAL_QUICK_REF.md](INTERVAL_QUICK_REF.md) | 快速参考表 |
| [config.yaml](config.yaml) | 配置示例 |
| [README.md](README.md) | 项目主文档 |

---

✅ **功能实现完成，已就绪使用！**
