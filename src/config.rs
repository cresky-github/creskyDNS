use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use indexmap::IndexMap;
use anyhow::Result;

/// 默认超时时间（秒）
fn default_timeout() -> u64 {
    5
}

/// 域名列表配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainList {
    /// 列表类型：direct 或 proxy
    pub r#type: String,
    /// 域名格式：text 或其他
    pub format: String,
    /// 文件路径（可选）
    pub path: Option<String>,
    /// URL（可选）
    pub url: Option<String>,
    /// 域名集合（可选，从文件加载时可以为空）
    #[serde(default)]
    pub domains: Vec<String>,
    /// 重新加载间隔（秒）：0 表示文件改变立即加载，>0 表示间隔期间无视文件改变
    #[serde(default)]
    pub interval: u64,
    /// 命中记录文件路径（由程序自动生成，不在配置文件中）
    #[serde(skip)]
    pub hit_path: Option<String>,
}

/// 域名列表重新加载状态
#[derive(Clone, Debug)]
pub struct DomainListReloadState {
    /// 最后一次文件修改时间戳
    pub last_modified: u64,
    /// 最后一次加载时间戳
    pub last_loaded: u64,
    /// 是否有待处理的更新
    pub pending_update: bool,
}

/// 上游DNS服务器列表配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpstreamList {
    /// 上游服务器地址列表
    pub addr: Vec<String>,
    /// HTTP/HTTPS 代理地址（可选，用于 DoH）
    /// 格式: http://127.0.0.1:7890 或 socks5://127.0.0.1:7891
    pub proxy: Option<String>,
}

/// 日志配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogConfig {
    /// 是否启用日志
    #[serde(default = "default_log_enabled")]
    pub enabled: bool,
    /// 日志文件路径
    #[serde(default = "default_log_path")]
    pub path: String,
    /// 日志级别: trace/debug/info/warn/error
    #[serde(default = "default_log_level")]
    pub level: String,
    /// 轮转时间（如 7d, 24h, 30m）
    #[serde(default = "default_log_max_time")]
    pub max_time: String,
    /// 单文件最大大小（如 100MB, 1GB）
    #[serde(default = "default_log_max_size")]
    pub max_size: String,
    /// 保留备份数量
    #[serde(default = "default_log_max_backups")]
    pub max_backups: usize,
    /// 日志格式模板
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_enabled() -> bool { true }
fn default_log_path() -> String { "./logs/creskyDNS.log".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_log_max_time() -> String { "7d".to_string() }
fn default_log_max_size() -> String { "100MB".to_string() }
fn default_log_max_backups() -> usize { 10 }
fn default_log_format() -> String { "|{date}|{time}|{level}|{process}|{module}|{message}|".to_string() }

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: default_log_enabled(),
            path: default_log_path(),
            level: default_log_level(),
            max_time: default_log_max_time(),
            max_size: default_log_max_size(),
            max_backups: default_log_max_backups(),
            format: default_log_format(),
        }
    }
}

/// 冷启动配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColdStartConfig {
    /// 是否启用冷启动
    #[serde(default)]
    pub enabled: bool,
    /// 超时时间（毫秒）
    #[serde(default = "default_cold_start_timeout")]
    pub timeout: u64,
    /// 并发查询数
    #[serde(default = "default_cold_start_parallel")]
    pub parallel: usize,
}

fn default_cold_start_timeout() -> u64 { 5000 }
fn default_cold_start_parallel() -> usize { 10 }

impl Default for ColdStartConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout: default_cold_start_timeout(),
            parallel: default_cold_start_parallel(),
        }
    }
}

/// 缓存类型
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheType {
    /// 规则缓存（记录域名匹配到的上游）
    Rule,
    /// 域名缓存（记录DNS解析结果）
    Domain,
    /// 禁用缓存（特殊配置：ID 为 "disable"，type 为 "cache"）
    Cache,
}

fn default_cache_interval() -> String { "5m".to_string() }

/// 缓存配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 缓存类型
    pub r#type: CacheType,
    /// 缓存条目数量（disable 类型不需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
    /// 最小缓存时间（秒，仅对 domain 类型有效）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_ttl: Option<u64>,
    /// 最大缓存时间（秒，仅对 domain 类型有效）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_ttl: Option<u64>,
    /// 缓存输出文件（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// 冷启动配置（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cold_start: Option<ColdStartConfig>,
    /// 导出间隔（如 5m, 1h），归零时保存到文件
    #[serde(default = "default_cache_interval")]
    pub interval: String,
}

/// Final 规则配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalRule {
    /// 主要上游服务器
    pub primary_upstream: String,
    /// 备用上游服务器
    pub fallback_upstream: String,
    /// IP CIDR 列表名称（用于判定国家代码）
    pub ipcidr: String,
    /// 输出文件路径（记录未分类域名）
    pub output: Option<String>,
}

/// DNS 转发器配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// 日志配置
    #[serde(default)]
    pub log: LogConfig,
    /// 监听器配置 (实例名 -> 端口)
    pub listener: HashMap<String, u16>,
    /// 域名列表配置 (name -> config)
    pub lists: HashMap<String, DomainList>,
    /// 上游列表配置 (name -> config)
    pub upstreams: HashMap<String, UpstreamList>,
    /// 规则配置 (按 YAML 顺序保留，使用 IndexMap 确保顺序)
    /// 注意：rules.final 会在加载后被提取到 final_rule 字段
    pub rules: IndexMap<String, Vec<String>>,
    /// Final 规则配置（可选，从 rules.final 中提取）
    #[serde(skip)]
    pub final_rule: Option<FinalRule>,
    /// 请求超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 缓存配置 (id -> config)
    pub cache: HashMap<String, CacheConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let mut lists = HashMap::new();
        lists.insert("direct".to_string(), DomainList {
            r#type: "direct".to_string(),
            format: "text".to_string(),
            path: None,
            url: None,
            domains: vec!["google.com".to_string(), "baidu.com".to_string()],
            interval: 0,
        });
        lists.insert("proxy".to_string(), DomainList {
            r#type: "proxy".to_string(),
            format: "text".to_string(),
            path: None,
            url: None,
            domains: vec!["twitter.com".to_string(), "facebook.com".to_string()],
            interval: 0,
        });

        let mut upstreams = HashMap::new();
        upstreams.insert("direct_dns".to_string(), UpstreamList {
            addr: vec!["udp://8.8.8.8:53".to_string()],
            proxy: None,
        });
        upstreams.insert("proxy_dns".to_string(), UpstreamList {
            addr: vec!["udp://1.1.1.1:53".to_string()],
            proxy: None,
        });
        upstreams.insert("default_dns".to_string(), UpstreamList {
            addr: vec!["udp://223.5.5.5:53".to_string()],
            proxy: None,
        });

        let mut rules = IndexMap::new();
        rules.insert("main".to_string(), vec![
            "direct,direct_dns".to_string(),
            "proxy,proxy_dns".to_string(),
        ]);

        let mut listener = HashMap::new();
        listener.insert("main".to_string(), 5353);
        listener.insert("backup".to_string(), 5354);

        let mut cache = HashMap::new();
        cache.insert("rule".to_string(), CacheConfig {
            r#type: CacheType::Rule,
            size: Some(10000),
            min_ttl: None,
            max_ttl: None,
            output: Some("./output/cache/rule.cache.txt".to_string()),
            cold_start: None,
            interval: "5m".to_string(),
        });
        cache.insert("domain".to_string(), CacheConfig {
            r#type: CacheType::Domain,
            size: Some(10000),
            min_ttl: Some(60),
            max_ttl: Some(86400),
            output: Some("./output/cache/domain.cache.txt".to_string()),
            cold_start: None,
            interval: "5m".to_string(),
        });

        Self {
            log: LogConfig::default(),
            listener,
            lists,
            upstreams,
            rules,
            final_rule: None,
            timeout_secs: 5,
            cache,
        }
    }
}

impl Config {
    /// 解析时间间隔字符串（如 "5m", "1h", "30s"）为秒数
    pub fn parse_interval(interval: &str) -> Result<u64> {
        let interval = interval.trim();
        if interval.is_empty() {
            return Ok(300); // 默认 5 分钟
        }
        
        let (num_str, unit) = if let Some(pos) = interval.find(|c: char| c.is_alphabetic()) {
            (&interval[..pos], &interval[pos..])
        } else {
            (interval, "s") // 默认单位为秒
        };
        
        let num: u64 = num_str.parse()
            .map_err(|_| anyhow::anyhow!("无效的时间间隔数字: {}", num_str))?;
        
        let seconds = match unit.to_lowercase().as_str() {
            "s" | "sec" | "second" | "seconds" => num,
            "m" | "min" | "minute" | "minutes" => num * 60,
            "h" | "hour" | "hours" => num * 3600,
            "d" | "day" | "days" => num * 86400,
            _ => return Err(anyhow::anyhow!("无效的时间单位: {}", unit)),
        };
        
        Ok(seconds)
    }
    
    /// 验证监听器端口配置
    pub fn validate_listener_ports(&self) -> Result<()> {
        use tracing::{warn, error};
        
        const RULE_KEY: &str = "rule";
        const SERVERS_GROUP: &str = "servers";
        let mut has_error = false;
        
        for (name, port) in &self.listener {
            // 检查 rule 键的端口范围
            if name == RULE_KEY {
                // rule 端口范围：53 或 1025-65535
                if *port != 53 && (*port < 1025 || *port > 65535) {
                    error!("监听器 '{}' 端口 {} 无效，取值范围：53 或 1025-65535", name, port);
                    has_error = true;
                }
                
                // 检查 rule 是否在 rules.servers 中被使用
                if let Some(servers_rules) = self.rules.get(SERVERS_GROUP) {
                    for rule_str in servers_rules {
                        if let Some((list_name, _)) = rule_str.split_once(',') {
                            let list_name = list_name.trim();
                            if list_name == RULE_KEY {
                                warn!("规则组 '{}' 中引用了监听器名称 '{}' 作为域名列表，这是无效的。'{}' 监听器参与顶层 rules 决策，但不参与 rules.servers 决策", 
                                    SERVERS_GROUP, RULE_KEY, RULE_KEY);
                            }
                        }
                    }
                }
            } else {
                // 其它监听器端口范围：1025-65535，不能是 53
                if *port == 53 {
                    error!("监听器 '{}' 不能使用端口 53，端口 53 只能用于 'rule' 监听器", name);
                    has_error = true;
                } else if *port < 1025 || *port > 65535 {
                    error!("监听器 '{}' 端口 {} 无效，取值范围：1025-65535（0-1024 由操作系统保留，除了 53 只能用于 'rule'）", name, port);
                    has_error = true;
                }
            }
        }
        
        if has_error {
            return Err(anyhow::anyhow!("监听器端口配置验证失败，请检查配置文件"));
        }
        
        Ok(())
    }
    
    /// 根据上游名称获取上游配置
    pub fn get_upstream(&self, name: &str) -> Result<&UpstreamList> {
        self.upstreams.get(name)
            .ok_or_else(|| anyhow::anyhow!("未找到上游配置: {}", name))
    }

    /// 根据监听器名称获取上游配置
    pub fn get_upstream_for_listener(&self, listener_name: &str) -> Result<&UpstreamList> {
        // 遍历所有规则，查找匹配的监听器
        for rule in self.rules.values() {
            for rule_str in rule {
                let parts: Vec<&str> = rule_str.split(',').map(|s| s.trim()).collect();
                if parts.len() == 2 && parts[0] == listener_name {
                    return self.get_upstream(parts[1]);
                }
            }
        }
        // 如果没有找到规则，使用第一个上游
        self.upstreams.values().next()
            .ok_or_else(|| anyhow::anyhow!("没有配置任何上游服务器"))
    }

    /// 获取第一个上游配置（用于默认情况）
    pub fn get_first_upstream(&self) -> Result<&UpstreamList> {
        self.upstreams.values().next()
            .ok_or_else(|| anyhow::anyhow!("没有配置任何上游服务器"))
    }

    /// 从 YAML 文件加载配置
    pub fn from_yaml(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        
        // 先解析为 Value 以便手动处理 rules.final
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content)?;
        
        // 提取 rules.final 如果存在
        let final_rule = if let Some(rules) = value.get_mut("rules").and_then(|r| r.as_mapping_mut()) {
            if let Some(final_value) = rules.remove(&serde_yaml::Value::String("final".to_string())) {
                // 解析为 FinalRule
                Some(serde_yaml::from_value::<FinalRule>(final_value)?)
            } else {
                None
            }
        } else {
            None
        };
        
        // 解析剩余的配置
        let mut config: Config = serde_yaml::from_value(value)?;
        
        // 设置 final_rule
        config.final_rule = final_rule;
        
        Ok(config)
    }

    /// 从 JSON 文件加载配置
    pub fn from_json(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 从文件加载配置（自动判断格式）
    pub fn from_file(path: &str) -> Result<Self> {
        let config = if path.ends_with(".yaml") || path.ends_with(".yml") {
            Self::from_yaml(path)?
        } else if path.ends_with(".json") {
            Self::from_json(path)?
        } else {
            anyhow::bail!("不支持的文件格式，请使用 .yaml, .yml 或 .json 文件");
        };
        Ok(config)
    }

    /// 保存配置到 YAML 文件
    pub fn save_yaml(&self, path: &str) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 保存配置到 JSON 文件
    pub fn save_json(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

impl DomainList {
    /// 从纯文本文件加载域名列表
    /// 文件格式：每行一个域名，不需要前后缀
    /// 示例：
    /// ```text
    /// com
    /// google.com
    /// www.google.com
    /// ```
    pub fn from_text_file(path: &str) -> Result<Vec<String>> {
        let content = fs::read_to_string(path)?;
        let domains = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#')) // 跳过空行和注释
            .map(|line| line.to_string())
            .collect();
        Ok(domains)
    }

    /// 加载域名列表（支持文件或URL）
    pub async fn load(&mut self) -> Result<()> {
        if let Some(path) = &self.path {
            match self.format.as_str() {
                "text" => {
                    self.domains = Self::from_text_file(path)?;
                }
                _ => {
                    anyhow::bail!("不支持的域名列表格式: {}", self.format);
                }
            }
        } else if let Some(url) = &self.url {
            // TODO: 实现从 URL 加载域名列表
            tracing::warn!("从 URL 加载域名列表功能尚未实现: {}", url);
        }
        Ok(())
    }

    /// 同步加载域名列表（用于监视线程）
    pub fn load_sync(&mut self) -> Result<()> {
        if let Some(path) = &self.path {
            match self.format.as_str() {
                "text" => {
                    self.domains = Self::from_text_file(path)?;
                }
                _ => {
                    anyhow::bail!("不支持的域名列表格式: {}", self.format);
                }
            }
        } else if let Some(url) = &self.url {
            // TODO: 实现从 URL 加载域名列表
            tracing::warn!("从 URL 加载域名列表功能尚未实现: {}", url);
        }
        Ok(())
    }

    /// 获取文件的最后修改时间戳（秒）
    pub fn get_file_modified_time(&self) -> Option<u64> {
        if let Some(path) = &self.path {
            match fs::metadata(path) {
                Ok(metadata) => {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            return Some(duration.as_secs());
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("无法获取文件 {} 的修改时间: {}", path, e);
                }
            }
        }
        None
    }

    /// 检查是否需要重新加载
    /// 返回 true 表示需要重新加载
    pub fn should_reload(&self, state: &DomainListReloadState) -> bool {
        // 获取当前文件修改时间
        match self.get_file_modified_time() {
            Some(current_modified) => {
                // 如果文件从未被加载过，返回 false（应该在启动时加载）
                if state.last_loaded == 0 {
                    return false;
                }

                // 如果文件未被修改，不需要重新加载
                if current_modified <= state.last_modified {
                    return false;
                }

                // 文件已被修改
                // 如果 interval == 0，立即加载
                if self.interval == 0 {
                    return true;
                }

                // 如果 interval > 0，检查时间间隔
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let time_since_last_load = now.saturating_sub(state.last_loaded);
                time_since_last_load >= self.interval
            }
            None => false,
        }
    }
}
