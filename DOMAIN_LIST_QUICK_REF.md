# 域名列表格式快速参考

## ✅ 正确格式

```text
# 注释行
com
google.com
www.google.com
api.example.com
subdomain.another.example.com
```

**特点**：
- 每行一个域名 ✓
- 不需要任何前缀 ✓
- 不需要任何后缀 ✓
- 注释行以 # 开头 ✓
- 支持空行 ✓

---

## ❌ 错误格式

### 不要添加通配符
```text
*.google.com       ❌ 错误
*.example.*        ❌ 错误
```

### 不要添加正则表达式
```text
^google\.com$      ❌ 错误
google\.com        ❌ 错误
(google|bing).com  ❌ 错误
```

### 不要添加协议或 IP
```text
https://google.com         ❌ 错误
http://google.com          ❌ 错误
127.0.0.1 google.com       ❌ 错误
google.com=127.0.0.1       ❌ 错误
```

### 不要添加路径或端口
```text
google.com:80              ❌ 错误
google.com/search          ❌ 错误
google.com:443/api         ❌ 错误
```

### 不要添加 Adblock 格式
```text
||google.com               ❌ 错误
google.com|                ❌ 错误
@@||google.com             ❌ 错误
```

---

## 匹配行为示例

### 如果列表包含 `google.com`

| 查询 | 匹配 | 原因 |
|------|------|------|
| `google.com` | ✅ | 完全匹配 |
| `www.google.com` | ✅ | 子域名前缀匹配 |
| `mail.google.com` | ✅ | 子域名前缀匹配 |
| `sub.www.google.com` | ✅ | 子域名前缀匹配 |
| `notgoogle.com` | ❌ | 不是前缀 |
| `google.com.cn` | ❌ | 不是后缀 |
| `g.com` | ❌ | 不是完整匹配 |

---

## 配置示例

```yaml
lists:
  direct:
    type: "domain"
    format: "text"
    path: "direct_domains.txt"
    domains:
      - baidu.com
      - qq.com
```

**说明**：
- `path`: 指定要加载的文件路径
- `domains`: 备用列表，当文件不存在时使用

---

## 常用域名示例

### 中国国内
```text
baidu.com
qq.com
taobao.com
weibo.com
aliyun.com
```

### 国外常用
```text
google.com
facebook.com
twitter.com
github.com
stackoverflow.com
```

### 广告屏蔽
```text
ads.google.com
ads.facebook.com
doubleclick.net
googlesyndication.com
```

---

## 速记

**记住这一点**：

> 域名列表文件就是一个**纯文本文件**，
> 每行一个域名，
> **不需要加任何符号或前缀**。

### 三句话
1. ✓ 每行一个域名
2. ✓ 注释用 # 开头  
3. ✓ 仅此而已

---

## 更多信息

详见 [DOMAIN_LIST_FORMAT.md](DOMAIN_LIST_FORMAT.md)
