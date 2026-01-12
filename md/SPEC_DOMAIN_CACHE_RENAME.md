# domain.cache 重命名与缓存规范（取代原 cache）

本规范将原配置项与文档中的 `cache` 全面重命名为 `domain.cache`，名称变化但功能保持一致，并补充新的记录格式与解析流程，明确与 `rule.cache` 的协同关系。

## 名称变更
- 原名称：`cache`
- 新名称：`domain.cache`
- 影响范围：配置文件键名、文档描述与输出文件命名建议（例如 main.cache.txt 可保留，但文档推荐以 domain.cache 语义对齐）。

## 记录格式

### domain.cache 输出文件行格式
- 规范：`|cache ID|rule|domain|ttl|IP(上游返回的其它内容)|`
- 要求：严格使用 `|` 作为分隔符；每行一条记录。
- 字段含义：
  - `cache ID`：缓存实例标识（例如 `main`）。
  - `rule`：命中的规则（为 lists 中的域名/模式，例：`ads.example.com` 或 `*.example.com`）。
  - `domain`：实际被解析的完整域名（例：`img.ads.example.com`）。
  - `ttl`：单位秒；当 TTL 归 0 时，应当从文件中删除该条目。
  - `IP(上游返回的其它内容)`：解析结果与上游的其它原始信息，可按实现需要写入（如 `A/AAAA` 记录组合、附加段等）。内容整体作为一个字段，仍由外层 `|` 分隔。

示例：
```
|main|ads.example.com|img.ads.example.com|292|203.0.113.5,203.0.113.6(A)|
|main|*.example.net|a.b.example.net|360|2001:db8::5(AAAA)|
```

### rule.cache（内存）记录格式
- 规范：`|rule|upstream|`
- 要求：严格使用 `|` 分隔，驻留内存；一旦触发 reload，需清空所有 `rule.cache` 内容。
- 字段含义：
  - `rule`：lists 文件中的域名/模式（与 `domain.cache` 中的 `rule` 吻合）。
  - `upstream`：该 rule 上次匹配成功所选择的上游标签名。

示例：
```
|ads.example.com|default_upstream|
|*.example.net|intl_upstream|
```

## 解析流程（rule.cache → domain.cache → rules）
为兼顾一致性与性能，解析域名按以下顺序进行：

1) 检查 rule.cache：
   - 基于传入域名，先进行“轻量规则匹配”（仅定位会命中的 lists 规则，不进行上游解析），得到 `rule`；
   - 若该 `rule` 存在于 `rule.cache`，获取对应的 `upstream` 作为“上游选择提示（hint）”。

2) 检查 domain.cache：
   - 携带上一步得到的 `rule` 与当前请求的 `domain`，在 `domain.cache` 中查找记录；
   - 命中且 TTL 有效则直接返回缓存的 `IP(其它内容)`；若 TTL=0 或未命中，进入下一步。

3) 按 rules 正常解析：
   - 依既有规则（含 `final` 逻辑、primary/fallback、ipcidr 判断等）进行上游查询；
   - 成功后：
     - 写入/更新 `rule.cache`（`|rule|upstream|`）。
     - 写入/更新 `domain.cache`（`|cache ID|rule|domain|ttl|IP(...)|`）。

说明与假设：
- “轻量规则匹配”仅用于定位 `rule`（lists 中的域名/模式），避免重复执行完整解析；若无法定位到任何 `rule`，直接进入步骤 3。
- 若实现层希望进一步加速，可通过 Trie/基数树/前缀树等结构对 lists 域名/模式做快速匹配。

## 冷启动（cold start）
- 原“按 cache.output 文件记录内容进行恢复”的描述替换为：
  - “冷启动时按 `domain.cache.output` 文件记录的内容，按既定并发与超时策略，对相关 `domain` 进行上游查询以刷新缓存，并重新写回 `domain.cache` 输出文件。”
- 并发控制、超时参数与原有含义一致，配置键名保持在 `domain.cache` 下（见示例配置）。

## 生命周期与清理
- `rule.cache`：完全驻留内存，reload 时清空。
- `domain.cache`：遵守 TTL；TTL 归 0 时从输出文件移除对应条目。

## 迁移指引（从 cache → domain.cache）
- 配置键：将顶层/对应位置的 `cache:` 改为 `domain.cache:`。
- 文档措辞：凡描述“cache.output”、“cache 文件”等，统一替换为 “domain.cache.output”、“domain.cache 文件”。
- 输出格式：从 `|cache ID|rule ID|domain|ttl|` 升级为 `|cache ID|rule|domain|ttl|IP(...)|`。
- 解析顺序描述：以 “rule.cache → domain.cache → rules” 为准。

## 附：边界与建议
- 上游返回多记录时，`IP(其它内容)` 字段建议以逗号分隔或原样串联，保持整个字段不包含外层分隔符 `|`。
- 如需更强通用性，可考虑将 `IP(其它内容)` 序列化为 JSON 再写入（实现可选，规范不强制）。
- 为避免过期膨胀，建议定期扫描 `domain.cache` 输出文件，清理 TTL 过期或 0 的条目。
