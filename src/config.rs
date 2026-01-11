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
    /// 域名集合
    pub domains: Vec<String>,
    /// 重新加载间隔（秒）：0 表示文件改变立即加载，>0 表示间隔期间无视文件改变
    #[serde(default)]
    pub interval: u64,
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
    pub addresses: Vec<String>,
}

/// 缓存配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 缓存条目数量
    pub size: usize,
    /// 最小缓存时间（秒，可选）
    pub min_ttl: Option<u64>,
    /// 最大缓存时间（秒，可选）
    pub max_ttl: Option<u64>,
}

/// DNS 转发器配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// 监听器配置 (实例名 -> 端口)
    pub listener: HashMap<String, u16>,
    /// 域名列表配置 (name -> config)
    pub lists: HashMap<String, DomainList>,
    /// 上游列表配置 (name -> config)
    pub upstreams: HashMap<String, UpstreamList>,
    /// 规则配置 (按 YAML 顺序保留，使用 IndexMap 确保顺序)
    pub rules: IndexMap<String, Vec<String>>,
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
            addresses: vec!["udp://8.8.8.8:53".to_string()],
        });
        upstreams.insert("proxy_dns".to_string(), UpstreamList {
            addresses: vec!["udp://1.1.1.1:53".to_string()],
        });
        upstreams.insert("default_dns".to_string(), UpstreamList {
            addresses: vec!["udp://223.5.5.5:53".to_string()],
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
        cache.insert("default".to_string(), CacheConfig {
            size: 1000,
            min_ttl: None,
            max_ttl: None,
        });

        Self {
            listener,
            lists,
            upstreams,
            rules,
            timeout_secs: 5,
            cache,
        }
    }
}

impl Config {
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
        let config = serde_yaml::from_str(&content)?;
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
