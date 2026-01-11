# ⏱️ interval 工作机制详解

## 核心规则

**interval 不是两次 reload 之间的最小间隔时间**

**interval 的计时起点是文件改变时刻**，倒计时期间无视后续文件改变。

---

## 时间轴示例

### 场景：interval = 300（5 分钟）

```
时间轴：
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

00:00  应用启动，加载文件
       │
       ├─ state.last_loaded = 00:00
       ├─ state.pending_update = false
       └─ （监视任务启动）

00:02  文件改变！ ← 计时起点
       │
       ├─ 检测到文件改变
       ├─ interval = 300 > 0
       ├─ pending_update = false（首次）
       │
       └─ 触发：
          ├─ state.pending_update = true
          ├─ state.pending_change_time = 00:02
          └─ 启动 300 秒倒计时（00:02 → 05:02）

00:04  文件再次改变 ✗ 忽略
       │
       ├─ 检测到文件改变
       ├─ pending_update = true（已有待处理）
       ├─ elapsed = 00:04 - 00:02 = 2 秒
       ├─ 2 < 300（倒计时未结束）
       │
       └─ 结果：忽略此次改变，倒计时继续

01:00  文件第三次改变 ✗ 忽略
       │
       ├─ 检测到文件改变
       ├─ pending_update = true
       ├─ elapsed = 01:00 - 00:02 = 58 秒
       ├─ 58 < 300
       │
       └─ 结果：忽略此次改变，倒计时继续

03:00  文件第四次改变 ✗ 忽略
       │
       ├─ elapsed = 03:00 - 00:02 = 178 秒
       ├─ 178 < 300
       │
       └─ 结果：继续等待

05:02  倒计时结束 ✓ 执行 reload
       │
       ├─ elapsed = 05:02 - 00:02 = 300 秒
       ├─ 300 >= 300（倒计时结束）
       │
       └─ 执行：
          ├─ 重新加载文件
          ├─ state.pending_update = false
          ├─ state.last_loaded = 05:02
          └─ 完成

05:10  文件再次改变 → 重新触发
       │
       ├─ pending_update = false（已清除）
       ├─ 检测到文件改变
       │
       └─ 触发新的倒计时：
          ├─ state.pending_update = true
          ├─ state.pending_change_time = 05:10
          └─ 新的 300 秒倒计时（05:10 → 10:10）

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## 关键理解

### ✅ 正确理解

| 概念 | 说明 |
|------|------|
| **计时起点** | 文件改变时刻（pending_change_time） |
| **倒计时长度** | interval 秒 |
| **倒计时期间** | 无视任何新的文件改变 |
| **执行时机** | 倒计时归 0 时 |
| **计时器重置** | reload 后清除 pending_update，下次文件改变重新触发 |

### ❌ 错误理解

| 错误观念 | 正确理解 |
|---------|---------|
| ❌ interval 是两次 reload 的最小间隔 | ✅ interval 是文件改变后的倒计时 |
| ❌ 计时起点是上次 reload 时间 | ✅ 计时起点是文件改变时刻 |
| ❌ interval 期间文件改变会重置计时器 | ✅ interval 期间无视任何文件改变 |
| ❌ 每隔 interval 秒检查一次 | ✅ 文件改变触发倒计时，倒计时归 0 时 reload |

---

## 状态机

```
┌─────────────────┐
│   初始状态      │
│ pending=false   │
└────────┬────────┘
         │
         │ 文件改变
         ↓
┌─────────────────┐
│  倒计时状态     │
│ pending=true    │ ← 期间无视文件改变
│ 计时: elapsed   │
└────────┬────────┘
         │
         │ elapsed >= interval
         ↓
┌─────────────────┐
│  执行 reload    │
│ pending=false   │
└────────┬────────┘
         │
         └─→ 回到初始状态
```

---

## interval: 0 的特殊情况

```
时间轴：
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

00:00  应用启动，加载文件

00:02  文件改变！
       │
       ├─ interval = 0（特殊值）
       │
       └─ 立即执行 reload（无倒计时）

00:04  文件再次改变
       │
       └─ 立即执行 reload

00:06  文件第三次改变
       │
       └─ 立即执行 reload

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**特点**：
- 无需等待
- 每次文件改变都触发 reload
- 适合开发环境

---

## 代码实现逻辑

### should_reload() 方法

```rust
fn should_reload(&self, state: &mut DomainListReloadState) -> bool {
    // 1. 检查文件是否改变
    let current_mtime = get_file_modified_time()?;
    if current_mtime <= state.last_modified {
        return false;  // 文件未改变
    }
    
    // 2. 文件已改变
    if self.interval == 0 {
        return true;  // interval:0 立即 reload
    }
    
    // 3. interval > 0
    if !state.pending_update {
        // 首次检测到改变，启动倒计时
        state.pending_update = true;
        state.pending_change_time = current_mtime;
        return false;  // 本次不 reload，等待倒计时
    }
    
    // 4. 已有倒计时，检查是否结束
    let now = get_current_timestamp();
    let elapsed = now - state.pending_change_time;
    
    if elapsed >= self.interval {
        // 倒计时结束，执行 reload
        state.pending_update = false;
        return true;
    }
    
    // 5. 倒计时未结束，继续等待（无视新的改变）
    false
}
```

### 关键变量

```rust
pub struct DomainListReloadState {
    pub last_modified: u64,        // 文件最后修改时间
    pub last_loaded: u64,          // 最后加载时间
    pub pending_update: bool,      // 是否有待处理的更新
    pub pending_change_time: u64,  // 文件改变时间（倒计时起点）
}
```

---

## 常见场景

### 场景 1：频繁编辑文件

```
用户行为：
- 00:00 编辑文件（保存）
- 00:10 再次编辑（保存）
- 00:20 再次编辑（保存）
- 00:30 再次编辑（保存）

interval: 300

结果：
- 00:00 触发倒计时
- 00:10-00:30 所有改变被忽略
- 05:00 执行 reload（只 reload 一次）

优势：避免频繁 reload
```

### 场景 2：自动化脚本更新

```
脚本行为：
- 每小时更新一次域名文件
- 00:00, 01:00, 02:00, ... 更新

interval: 300

结果：
- 00:00 触发倒计时
- 01:00 改变被忽略（倒计时继续）
- 05:00 执行 reload
- 05:00 之后的 01:00 触发新的倒计时
- ...

说明：interval 确保不会过于频繁 reload
```

### 场景 3：开发环境快速迭代

```
interval: 0

结果：
- 每次保存文件立即 reload
- 快速看到效果
- 无需等待
```

---

## 配置建议

| 使用场景 | 推荐 interval | 理由 |
|---------|--------------|------|
| **开发调试** | 0 | 立即看到效果 |
| **测试环境** | 60-300 | 平衡更新速度和稳定性 |
| **生产环境（频繁更新）** | 300-600 | 避免过度 reload |
| **生产环境（稳定）** | 1800-3600 | 减少不必要的 reload |
| **很少变化的列表** | 3600+ | 最大稳定性 |

---

## 常见问题

### Q1: 为什么设置 interval:300，但修改文件后立即生效了？

**A**: 不可能。interval:300 意味着文件改变后需要等待 300 秒。如果立即生效，说明：
- interval 配置为 0
- 或者文件在 300 秒前已经改变（倒计时已结束）

### Q2: interval 期间我修改了多次文件，会 reload 多次吗？

**A**: 不会。interval 期间的所有改变都被忽略，只在倒计时结束时 reload 一次。

### Q3: 我想每 5 分钟自动检查一次，应该设置什么？

**A**: 这是错误的理解。interval 不是定时检查，而是文件改变后的倒计时。应用每 5 秒检查文件变化，但 reload 时机由 interval 决定。

### Q4: 倒计时期间我修改文件，会重置倒计时吗？

**A**: **不会**。倒计时不会重置，期间的所有改变都被忽略。

---

## 总结

```
╔═══════════════════════════════════════════════════════════╗
║  interval 规则精髓                                        ║
╠═══════════════════════════════════════════════════════════╣
║  1. 计时起点 = 文件改变时刻                               ║
║  2. 倒计时长度 = interval 秒                              ║
║  3. 倒计时期间 = 无视任何新的文件改变                    ║
║  4. 执行时机 = 倒计时归 0 时                              ║
║  5. interval:0 = 立即 reload（无倒计时）                 ║
╚═══════════════════════════════════════════════════════════╝
```

---

**关键**：interval 是**防抖机制**，避免文件频繁变化时的过度 reload。
