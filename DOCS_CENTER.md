# 📖 域名列表文档中心

## 🎯 快速导航

### 根据你的需求选择：

**我刚开始，5分钟快速上手** 👉 [QUICK_START.md](QUICK_START.md)

**我想快速查看 dos 和 don'ts** 👉 [DOMAIN_LIST_QUICK_REF.md](DOMAIN_LIST_QUICK_REF.md)

**我需要详细的规范说明** 👉 [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md)

**我想了解所有的变更** 👉 [DOMAIN_LIST_UPDATE.md](DOMAIN_LIST_UPDATE.md)

**我需要验证我的文件格式** 👉 [VALIDATION_TOOL.md](VALIDATION_TOOL.md)

**我想看完整的总结** 👉 [DOMAIN_LIST_SPEC_COMPLETE.md](DOMAIN_LIST_SPEC_COMPLETE.md)

---

## 📚 完整文档列表

### 快速参考
| 文档 | 描述 | 用途 |
|------|------|------|
| [QUICK_START.md](QUICK_START.md) | 5分钟快速开始指南 | 新用户入门 |
| [DOMAIN_LIST_QUICK_REF.md](DOMAIN_LIST_QUICK_REF.md) | dos/don'ts 速查表 | 快速查询 |

### 详细文档
| 文档 | 描述 | 用途 |
|------|------|------|
| [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) | 详细格式规范 | 完整了解 |
| [DOMAIN_LIST_UPDATE.md](DOMAIN_LIST_UPDATE.md) | 更新说明和变更列表 | 了解变更 |
| [VALIDATION_TOOL.md](VALIDATION_TOOL.md) | 验证工具使用指南 | 验证文件 |
| [DOMAIN_LIST_SPEC_COMPLETE.md](DOMAIN_LIST_SPEC_COMPLETE.md) | 规范化完整总结 | 全面了解 |

### 相关文档
| 文档 | 描述 |
|------|------|
| [README.md](README.md) | 项目主文档 |
| [RULE_MATCHING.md](RULE_MATCHING.md) | 规则匹配说明 |

---

## 🗂️ 文件组织

### 配置文件
- `config.yaml` - 应用配置（包含列表配置）
- `config.example.yaml` - 配置示例
- `config.example.json` - JSON 格式示例

### 域名列表文件
- `direct_domains.txt` - 直连域名列表
- `proxy_domains.txt` - 代理域名列表
- `adblock_domains.txt` - 广告屏蔽列表
- `custom_domains.txt` - 自定义列表模板

### 工具
- `validate_domain_lists.py` - 格式验证脚本

### 文档
- 本文件及以上所有 `.md` 文件

---

## 🚀 常见场景

### 场景 1：我是新用户
```
1. 阅读 QUICK_START.md (5分钟)
2. 浏览 DOMAIN_LIST_QUICK_REF.md (2分钟)
3. 准备好开始了！
```

### 场景 2：我需要详细了解
```
1. 阅读 DOMAIN_LIST_FORMAT.md (全面了解)
2. 参考相关示例
3. 查看 RULE_MATCHING.md 了解匹配行为
```

### 场景 3：我遇到了问题
```
1. 检查文件格式：使用 validate_domain_lists.py
2. 查看应用日志
3. 参考 DOMAIN_LIST_FORMAT.md 的故障排除
4. 查看 QUICK_START.md 的故障排除
```

### 场景 4：我想了解所有变更
```
1. 阅读 DOMAIN_LIST_UPDATE.md
2. 查看 DOMAIN_LIST_SPEC_COMPLETE.md
3. 浏览 README.md 的文档链接
```

---

## 💡 关键概念速记

### 格式规则
```
✅ 正确：
com
google.com
www.google.com

❌ 错误：
*.google.com
||google.com
google.com:80
```

### 加载流程
```
配置文件 config.yaml
    ↓
读取 lists 部分
    ↓
如果有 path 字段
    ↓
加载文件（覆盖 domains）
    ↓
使用最终的域名列表
```

### 匹配行为
```
列表中有：google.com

查询 google.com      → ✅ 匹配
查询 www.google.com  → ✅ 匹配 (子域名)
查询 notgoogle.com   → ❌ 不匹配
```

---

## 🔗 文档导览树

```
文档中心 (本文件)
├── 快速开始
│   ├── QUICK_START.md           (5分钟入门)
│   ├── DOMAIN_LIST_QUICK_REF.md (速查表)
│   └── VALIDATION_TOOL.md       (工具使用)
│
├── 深入学习
│   ├── DOMAIN_LIST_FORMAT.md    (详细规范)
│   ├── DOMAIN_LIST_UPDATE.md    (变更说明)
│   ├── DOMAIN_LIST_SPEC_COMPLETE.md (完整总结)
│   └── RULE_MATCHING.md         (规则匹配)
│
├── 示例文件
│   ├── config.yaml              (配置示例)
│   ├── direct_domains.txt       (示例列表)
│   ├── proxy_domains.txt        (示例列表)
│   └── ...
│
└── 工具
    └── validate_domain_lists.py (验证脚本)
```

---

## ❓ 常见问题

### Q: 从哪里开始？
**A:** 如果你是新用户，从 [QUICK_START.md](QUICK_START.md) 开始。

### Q: 如何验证我的文件？
**A:** 使用 [validate_domain_lists.py](validate_domain_lists.py) 或参考 [VALIDATION_TOOL.md](VALIDATION_TOOL.md)。

### Q: 我的文件格式有问题，怎么办？
**A:** 
1. 运行验证脚本查看具体错误
2. 参考 [DOMAIN_LIST_QUICK_REF.md](DOMAIN_LIST_QUICK_REF.md) 的对比示例
3. 查看 [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) 的故障排除

### Q: 规则匹配如何工作？
**A:** 参考 [RULE_MATCHING.md](RULE_MATCHING.md)。

### Q: 有哪些变更？
**A:** 查看 [DOMAIN_LIST_UPDATE.md](DOMAIN_LIST_UPDATE.md) 或 [DOMAIN_LIST_SPEC_COMPLETE.md](DOMAIN_LIST_SPEC_COMPLETE.md)。

---

## 📊 文档统计

| 类型 | 数量 | 说明 |
|------|------|------|
| 快速参考 | 2 | 快速上手和速查表 |
| 详细文档 | 4 | 规范、更新、验证、总结 |
| 配置示例 | 4 | YAML/JSON 配置和列表文件 |
| 工具脚本 | 1 | Python 验证脚本 |
| 总计 | 11+ | 完整的文档生态 |

---

## 🎓 学习路径建议

### 初级用户（30分钟）
1. QUICK_START.md (5分钟)
2. DOMAIN_LIST_QUICK_REF.md (5分钟)
3. 实际操作：创建文件、配置、运行 (20分钟)

### 中级用户（1小时）
1. 上面的初级路径 (30分钟)
2. DOMAIN_LIST_FORMAT.md (20分钟)
3. 查看示例文件和配置 (10分钟)

### 高级用户（2小时）
1. 所有文档 (1.5小时)
2. 深入研究 RULE_MATCHING.md (30分钟)
3. 自定义配置和优化 (配套)

---

## 🔄 文档维护

**最后更新**：2024-01-15
**版本**：1.0

---

## 🆘 需要帮助？

1. **查看快速参考** - [DOMAIN_LIST_QUICK_REF.md](DOMAIN_LIST_QUICK_REF.md)
2. **运行验证工具** - `python validate_domain_lists.py`
3. **查看详细文档** - [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md)
4. **查看故障排除** - [QUICK_START.md](QUICK_START.md#故障排除)

---

**祝你使用愉快！** 🎉
