# 域名列表文件格式更新说明

## 更新概述

已经规范了 DNS 转发器中的域名列表文件格式，并添加了从文件加载域名列表的功能。

## 主要变更

### 1. 域名列表文件格式规范

**新格式**：
- 每行一个域名
- 无前后缀（直接写入域名本身）
- 以 `#` 开头的行作为注释（会被忽略）
- 空行会被自动跳过

**示例**：
```text
# 这是注释
com
google.com
www.google.com
api.example.com
```

**禁止的格式**：
```text
*.google.com      ❌ 不需要通配符
^google.com       ❌ 不需要正则前缀
||google.com      ❌ 不需要 adblock 格式
google.com$       ❌ 不需要后缀标记
127.0.0.1 google.com  ❌ 不需要 IP 地址
```

### 2. 代码更改

#### src/config.rs
- 新增 `DomainList::from_text_file()` 方法：从纯文本文件加载域名列表
- 新增 `DomainList::load()` 方法：异步加载域名列表（支持文件或 URL）
- 添加 `use tracing;` 导入用于日志输出

#### src/main.rs
- 修改 `load_config()` 返回值为可变的 `Config`
- 在启动时加载所有指定路径的域名列表文件
- 添加日志输出，显示每个列表的加载状态

### 3. 文件更新

#### 域名列表文件
已清理并规范化以下文件：

1. **direct_domains.txt**
   - 包含国内直连域名
   - 已删除分类注释，保留格式注释
   
2. **proxy_domains.txt**
   - 包含需要代理的国外域名
   - 已删除分类注释，保留格式注释

3. **adblock_domains.txt**
   - 包含要屏蔽的广告域名
   - 已删除分类注释，保留格式注释

4. **custom_domains.txt**
   - 用户自定义域名列表模板
   - 添加清晰的说明注释

#### 配置文件
**config.yaml** 更新：
- `lists` 部分已规范化
- 移除了尚不支持的 `url` 和 `interval` 字段
- 保留 `path` 字段用于指定本地文件

### 4. 新增文档

**DOMAIN_LIST_FORMAT.md**：
详细的域名列表文件格式说明，包括：
- 基本格式规范
- 允许/禁止的内容
- 匹配行为说明
- 配置示例
- 最佳实践
- 故障排除

## 使用方式

### 配置示例

```yaml
lists:
  direct:
    type: "domain"
    format: "text"
    path: "direct_domains.txt"    # 指定文件路径
    domains:                       # 备用的内联域名列表
      - baidu.com
      - qq.com
```

### 工作流程

1. **应用启动**时，配置管理器加载 `config.yaml`
2. **检查 lists 配置**，对于每个有 `path` 字段的列表：
   - 从指定文件加载域名
   - 使用文件内容替换 `domains` 字段
   - 输出日志显示加载结果
3. **域名匹配**时，使用最终加载的域名列表进行查询

## 文件加载逻辑

```rust
// 加载过程
for (name, list) in &mut config.lists {
    if let Some(path) = &list.path {
        match list.load().await {
            Ok(_) => {
                // 成功：list.domains 已被文件内容替换
                info!("域名列表 '{}' 从文件 '{}' 加载成功: {} 个域名",
                      name, path, list.domains.len());
            }
            Err(e) => {
                // 失败：使用配置中的备用域名列表
                error!("域名列表 '{}' 加载失败: {}", name, e);
            }
        }
    }
}
```

## 最佳实践

### 1. 文件组织
```text
project/
├── config.yaml
├── direct_domains.txt      # 国内直连
├── proxy_domains.txt       # 国外代理
├── adblock_domains.txt     # 广告屏蔽
└── custom_domains.txt      # 用户自定义
```

### 2. 性能优化
- 避免重复的域名条目
- 定期清理无用的域名
- 大型列表可以分割成多个文件

### 3. 维护建议
```text
# 在文件开头添加元信息
# 更新时间：2024-01-15
# 来源：示例列表
# 包含域名数：150

google.com
facebook.com
# twitter.com  # 临时禁用
```

## 相关文档

- [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md) - 详细格式说明
- [config.yaml](config.yaml) - 配置文件示例
- [RULE_MATCHING.md](RULE_MATCHING.md) - 规则匹配说明

## 常见问题

### Q: 域名列表文件不存在会怎样？
A: 应用会输出错误日志，但继续使用 `domains` 字段中的备用列表启动。

### Q: 可以不指定 path 字段吗？
A: 可以。如果不指定 `path`，应用会直接使用 `domains` 字段中的内联列表。

### Q: 域名匹配是精确匹配吗？
A: 不是。使用后缀匹配（前缀匹配），例如 `google.com` 会匹配 `www.google.com`。

### Q: 文件编码有要求吗？
A: 建议使用 UTF-8 编码，以支持国际化域名。

## 后续计划

- [ ] 实现远程 URL 加载功能（`url` 字段）
- [ ] 实现定时更新功能（`interval` 字段）
- [ ] 支持更多文件格式（JSON、YAML 等）
- [ ] 增量更新支持（只更新差异）
- [ ] 域名列表缓存

## 更新日期

- 2024-01-15：初始规范化和代码实现
