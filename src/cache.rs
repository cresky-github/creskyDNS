use hickory_proto::op::Message;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tracing::{debug, info, warn, error};
use anyhow::Result;
use indexmap::IndexMap;

use crate::config::{CacheConfig, CacheType, Config};

/// DNS 缓存记录
#[derive(Clone, Debug)]
pub struct CachedDnsRecord {
    /// 缓存 ID（从 rule.cache 复制）
    pub cache_id: String,
    /// 匹配到的域名（用于链接到 rule.cache）
    pub matched_domain: String,
    /// 查询的域名
    pub domain: String,
    /// 上游服务器名称（从 rule.cache 复制）
    pub upstream: String,
    /// 原始 TTL
    pub original_ttl: u64,
    /// 过期时间点
    pub expire_at: Instant,
    /// 记录时间戳（用于导出）
    pub timestamp: u64,
    /// 缓存的 DNS 响应消息
    pub message: Message,
}

impl CachedDnsRecord {
    /// 检查缓存是否已过期
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expire_at
    }

    /// 获取剩余 TTL（秒）
    pub fn remaining_ttl(&self) -> u64 {
        let now = Instant::now();
        if now >= self.expire_at {
            0
        } else {
            (self.expire_at - now).as_secs()
        }
    }
}

/// Domain Cache（DNS 缓存）
#[derive(Clone)]
pub struct DomainCache {
    /// 缓存数据（domain -> record）
    cache: Arc<RwLock<HashMap<String, CachedDnsRecord>>>,
    /// 缓存 ID
    cache_id: String,
    /// 最大缓存条目数
    max_size: usize,
    /// 最小 TTL（秒）
    min_ttl: Option<u64>,
    /// 最大 TTL（秒）
    max_ttl: Option<u64>,
    /// 缓存输出文件路径
    output_path: Option<String>,
}

impl DomainCache {
    /// 创建新的 Domain Cache
    pub fn new(cache_id: String, max_size: usize, min_ttl: Option<u64>, max_ttl: Option<u64>) -> Self {
        info!(
            "创建 Domain Cache '{}': size={}, min_ttl={:?}, max_ttl={:?}",
            cache_id, max_size, min_ttl, max_ttl
        );
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_id,
            max_size,
            min_ttl,
            max_ttl,
            output_path: None,
        }
    }
    
    /// 从配置创建 Domain Cache
    pub fn from_config(config: &CacheConfig, cache_id: String) -> Self {
        let cache = Arc::new(RwLock::new(HashMap::new()));
        
        // 如果配置了输出文件且启用了冷启动，尝试加载
        if let Some(ref output_path) = config.output {
            if config.cold_start.as_ref().map_or(false, |cs| cs.enabled) {
                if let Err(e) = Self::load_from_file(output_path, &cache, &cache_id) {
                    warn!("加载域名缓存文件 {} 失败: {}, 将从空缓存开始", output_path, e);
                }
            }
        }
        
        info!(
            "创建 Domain Cache '{}': size={}, min_ttl={:?}, max_ttl={:?}, output={:?}",
            cache_id, config.size, config.min_ttl, config.max_ttl, config.output
        );
        
        Self {
            cache,
            cache_id,
            max_size: config.size,
            min_ttl: config.min_ttl,
            max_ttl: config.max_ttl,
            output_path: config.output.clone(),
        }
    }
    
    /// 从文件加载缓存
    fn load_from_file(path: &str, cache: &Arc<RwLock<HashMap<String, CachedDnsRecord>>>, _cache_id: &str) -> Result<()> {
        if !Path::new(path).exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(path)?;
        let mut loaded = 0;
        let now = Instant::now();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            // 格式: |cache ID|match domain|upstream|qname|ttl|IP(及其它信息)|
            let parts: Vec<&str> = line.split('|').filter(|s| !s.is_empty()).collect();
            if parts.len() != 6 {
                continue;
            }
            
            let qname = parts[3].to_string();
            let ttl: u64 = parts[4].parse().unwrap_or(0);
            
            // 创建简单的 DNS 消息（冷启动时只保存 IP 信息，不完整重建 Message）
            // 实际查询时会重新获取完整记录
            let message = Message::new();
            // TODO: 解析 IP 信息并重建 DNS 响应
            
            let record = CachedDnsRecord {
                cache_id: parts[0].to_string(),
                matched_domain: parts[1].to_string(),
                domain: qname.clone(),
                upstream: parts[2].to_string(),
                original_ttl: ttl,
                expire_at: now + Duration::from_secs(ttl),
                timestamp,
                message,
            };
            
            cache.write().unwrap().insert(qname, record);
            loaded += 1;
        }
        
        info!("从文件 {} 加载了 {} 条域名缓存", path, loaded);
        Ok(())
    }

    /// 查询缓存
    pub fn get(&self, domain: &str) -> Option<Message> {
        let cache = self.cache.read().unwrap();
        if let Some(record) = cache.get(domain) {
            if record.is_expired() {
                debug!("Domain Cache '{}': 域名 {} 缓存已过期", self.cache_id, domain);
                drop(cache);
                // 删除过期记录
                self.remove(domain);
                return None;
            }
            debug!(
                "Domain Cache '{}': 命中域名 {} (剩余 TTL: {}s)",
                self.cache_id,
                domain,
                record.remaining_ttl()
            );
            return Some(record.message.clone());
        }
        debug!("Domain Cache '{}': 未命中域名 {}", self.cache_id, domain);
        None
    }
    
    /// 按复合KEY查询缓存（cache_id + match_domain + upstream + qname）
    pub fn get_by_key(&self, cache_id: &str, match_domain: &str, upstream: &str, qname: &str) -> Option<Message> {
        let cache = self.cache.read().unwrap();
        
        // 遍历缓存，找到匹配的条目
        for (cached_qname, record) in cache.iter() {
            if cached_qname == qname 
                && record.cache_id == cache_id 
                && record.matched_domain == match_domain 
                && record.upstream == upstream {
                
                if record.is_expired() {
                    debug!("Domain Cache '{}': KEY匹配但已过期: {}|{}|{}|{}", 
                        self.cache_id, cache_id, match_domain, upstream, qname);
                    return None;
                }
                
                debug!(
                    "Domain Cache '{}': KEY命中: {}|{}|{}|{} (剩余 TTL: {}s)",
                    self.cache_id, cache_id, match_domain, upstream, qname, record.remaining_ttl()
                );
                return Some(record.message.clone());
            }
        }
        
        debug!("Domain Cache '{}': KEY未命中: {}|{}|{}|{}", 
            self.cache_id, cache_id, match_domain, upstream, qname);
        None
    }

    /// 插入缓存
    pub fn insert(&self, domain: String, cache_id: String, matched_domain: String, upstream: String, message: Message, ttl: u64) {
        let mut cache = self.cache.write().unwrap();

        // 检查缓存大小限制，使用 LRU 淘汰策略
        if cache.len() >= self.max_size && !cache.contains_key(&domain) {
            // 简单 LRU：删除最早过期的条目
            if let Some(oldest_key) = self.find_earliest_expiry(&cache) {
                debug!(
                    "Domain Cache '{}': 缓存已满，淘汰域名 {}",
                    self.cache_id, oldest_key
                );
                cache.remove(&oldest_key);
            }
        }

        // 应用 min_ttl 和 max_ttl 限制
        let adjusted_ttl = self.adjust_ttl(ttl);

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let expire_at = Instant::now() + Duration::from_secs(adjusted_ttl);
        let record = CachedDnsRecord {
            cache_id,
            matched_domain: matched_domain.clone(),
            domain: domain.clone(),
            upstream,
            original_ttl: ttl,
            expire_at,
            timestamp,
            message,
        };

        cache.insert(domain.clone(), record);
        debug!(
            "Domain Cache '{}': 写入域名 {} (匹配域名: {}, TTL: {}s)",
            self.cache_id, domain, matched_domain, adjusted_ttl
        );
    }

    /// 删除缓存记录
    pub fn remove(&self, domain: &str) {
        let mut cache = self.cache.write().unwrap();
        cache.remove(domain);
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        let count = cache.len();
        cache.clear();
        info!("Domain Cache '{}': 已清空 {} 条记录", self.cache_id, count);
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read().unwrap();
        let total = cache.len();
        let expired = cache.values().filter(|r| r.is_expired()).count();
        CacheStats {
            total,
            valid: total - expired,
            expired,
        }
    }

    /// 清理过期缓存（定期调用）
    pub fn cleanup_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        let before_count = cache.len();
        cache.retain(|_, record| !record.is_expired());
        let after_count = cache.len();
        let removed = before_count - after_count;
        if removed > 0 {
            info!(
                "Domain Cache '{}': 清理了 {} 条过期记录",
                self.cache_id, removed
            );
        }
    }
    
    /// 验证 domain.cache 条目是否有对应的有效 rule.cache 条目
    /// 返回: (valid_records, invalid_count, warm_up_list)
    pub fn validate_against_rule_cache(
        &self,
        valid_rule_entries: &[(String, String, String)],
    ) -> (Vec<CachedDnsRecord>, usize, Vec<(String, String, String, String)>) {
        let cache = self.cache.read().unwrap();
        let mut valid_records = Vec::new();
        let mut invalid_count = 0;
        let mut warm_up_list = Vec::new();
        
        // 构建有效的 rule.cache 键集合
        let mut valid_keys = std::collections::HashSet::new();
        for (match_domain, upstream, _cache_id) in valid_rule_entries {
            valid_keys.insert((match_domain.clone(), upstream.clone()));
        }
        
        for (domain, record) in cache.iter() {
            let key = (record.matched_domain.clone(), record.upstream.clone());
            
            if valid_keys.contains(&key) {
                // 记录有效，但需要预热（重新查询）
                valid_records.push(record.clone());
                warm_up_list.push((
                    domain.clone(),
                    record.matched_domain.clone(),
                    record.upstream.clone(),
                    record.cache_id.clone(),
                ));
            } else {
                invalid_count += 1;
                debug!("Domain Cache '{}' 冷启动验证: 移除无效条目 {} (规则不存在)", self.cache_id, domain);
            }
        }
        
        (valid_records, invalid_count, warm_up_list)
    }
    
    /// 导出缓存到文件
    pub fn export_to_file(&self) -> Result<()> {
        if let Some(ref output_path) = self.output_path {
            // 创建输出目录
            if let Some(parent) = Path::new(output_path).parent() {
                fs::create_dir_all(parent)?;
            }
            
            let cache = self.cache.read().unwrap();
            let mut file = File::create(output_path)?;
            
            // 只导出未过期的条目，按过期时间排序
            let mut entries: Vec<_> = cache.values()
                .filter(|e| !e.is_expired())
                .collect();
            entries.sort_by_key(|e| e.timestamp);
            
            for entry in &entries {
                // 提取 IP 信息
                let ip_info = Self::extract_ip_info(&entry.message);
                
                // 格式: |cache ID|match domain|upstream|qname|ttl|IP(及其它信息)|
                writeln!(file, "|{}|{}|{}|{}|{}|{}|", 
                    entry.cache_id, 
                    entry.matched_domain, 
                    entry.upstream,
                    entry.domain,
                    entry.remaining_ttl(),
                    ip_info)?;
            }
            
            info!("Domain Cache '{}': 已导出 {} 条缓存到 {}", self.cache_id, entries.len(), output_path);
        }
        Ok(())
    }
    
    /// 从 DNS 消息提取 IP 信息
    fn extract_ip_info(message: &Message) -> String {
        let mut ips = Vec::new();
        for answer in message.answers() {
            if let Some(rdata) = answer.data() {
                ips.push(format!("{}", rdata));
            }
        }
        if ips.is_empty() {
            "NODATA".to_string()
        } else {
            ips.join(",")
        }
    }

    /// 查找最早过期的条目（用于 LRU 淘汰）
    fn find_earliest_expiry(&self, cache: &HashMap<String, CachedDnsRecord>) -> Option<String> {
        cache
            .iter()
            .min_by_key(|(_, record)| record.expire_at)
            .map(|(key, _)| key.clone())
    }

    /// 调整 TTL（应用 min_ttl 和 max_ttl 限制）
    fn adjust_ttl(&self, ttl: u64) -> u64 {
        let mut adjusted = ttl;
        if let Some(min) = self.min_ttl {
            if adjusted < min {
                adjusted = min;
            }
        }
        if let Some(max) = self.max_ttl {
            if adjusted > max {
                adjusted = max;
            }
        }
        adjusted
    }
}

/// 缓存统计信息
#[derive(Debug)]
pub struct CacheStats {
    /// 总记录数
    pub total: usize,
    /// 有效记录数
    pub valid: usize,
    /// 过期记录数
    pub expired: usize,
}

/// Rule Cache（规则缓存）
/// 格式：|cache ID|domain|upstream| (domain -> upstream_name)
/// 用于加速 DNS 解析，避免重复的规则匹配
#[derive(Clone)]
pub struct RuleCache {
    /// 缓存数据（domain -> (upstream_name, cache_id)）
    cache: Arc<RwLock<HashMap<String, (String, String)>>>,
    /// 缓存输出文件路径
    output_path: Option<String>,
    /// 默认上游服务器（YAML 顺序最后一个）
    default_upstream: String,
}

impl RuleCache {
    /// 创建新的 Rule Cache
    pub fn new() -> Self {
        info!("创建 Rule Cache (内存规则缓存)");
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            output_path: None,
            default_upstream: String::new(),
        }
    }
    
    /// 从配置创建 Rule Cache
    pub fn from_config(config: &CacheConfig, default_upstream: String) -> Self {
        let cache = Arc::new(RwLock::new(HashMap::new()));
        
        // 如果配置了输出文件且启用了冷启动，尝试加载
        if let Some(ref output_path) = config.output {
            if config.cold_start.as_ref().map_or(false, |cs| cs.enabled) {
                if let Err(e) = Self::load_from_file(output_path, &cache) {
                    warn!("加载规则缓存文件 {} 失败: {}, 将从空缓存开始", output_path, e);
                }
            }
        }
        
        info!("创建 Rule Cache: default_upstream={}, output={:?}", default_upstream, config.output);
        Self {
            cache,
            output_path: config.output.clone(),
            default_upstream,
        }
    }
    
    /// 从文件加载缓存
    fn load_from_file(path: &str, cache: &Arc<RwLock<HashMap<String, (String, String)>>>) -> Result<()> {
        if !Path::new(path).exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(path)?;
        let mut loaded = 0;
        
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            // 格式: |cache ID|match domain|upstream|
            let parts: Vec<&str> = line.split('|').filter(|s| !s.is_empty()).collect();
            if parts.len() != 3 {
                continue;
            }
            
            let cache_id = parts[0].to_string();
            let domain = parts[1].to_string();
            let upstream = parts[2].to_string();
            
            cache.write().unwrap().insert(domain, (upstream, cache_id));
            loaded += 1;
        }
        
        info!("从文件 {} 加载了 {} 条规则缓存", path, loaded);
        Ok(())
    }

    /// 查询缓存
    pub fn get(&self, domain: &str) -> Option<(String, String)> {
        let cache = self.cache.read().unwrap();
        if let Some((upstream, cache_id)) = cache.get(domain) {
            debug!("Rule Cache 命中: {} -> {} (cache_id: {})", domain, upstream, cache_id);
            return Some((upstream.clone(), cache_id.clone()));
        }
        debug!("Rule Cache 未命中: {}", domain);
        None
    }
    
    /// 按域名深度查询匹配的 match domain（深度大者优先）
    /// 返回: Vec<(match_domain, upstream, cache_id)>
    pub fn get_matches_by_depth(&self, qname: &str) -> Vec<(String, String, String)> {
        let cache = self.cache.read().unwrap();
        let mut matches = Vec::new();
        
        // 遍历所有 match domain，找到匹配 qname 的条目
        for (match_domain, (upstream, cache_id)) in cache.iter() {
            if Self::domain_matches(qname, match_domain) {
                matches.push((match_domain.clone(), upstream.clone(), cache_id.clone()));
            }
        }
        
        // 按域名深度排序（深度大者优先）
        matches.sort_by(|a, b| {
            let depth_a = Self::get_domain_depth(&a.0);
            let depth_b = Self::get_domain_depth(&b.0);
            depth_b.cmp(&depth_a) // 降序
        });
        
        if !matches.is_empty() {
            debug!("Rule Cache 按深度匹配: {} -> {} 个匹配项", qname, matches.len());
        }
        
        matches
    }
    
    /// 计算域名深度
    fn get_domain_depth(domain: &str) -> usize {
        if domain == "." {
            return 0;
        }
        domain.matches('.').count() + 1
    }
    
    /// 判断 qname 是否匹配 match_domain
    fn domain_matches(qname: &str, match_domain: &str) -> bool {
        if match_domain == "." {
            return true; // 根域名匹配所有
        }
        
        // 精确匹配
        if qname == match_domain {
            return true;
        }
        
        // 后缀匹配：qname 以 .match_domain 结尾
        if qname.ends_with(&format!(".{}", match_domain)) {
            return true;
        }
        
        false
    }

    /// 插入缓存
    pub fn insert(&self, domain: String, upstream: String, cache_id: String) {
        let mut cache = self.cache.write().unwrap();
        cache.insert(domain.clone(), (upstream.clone(), cache_id.clone()));
        debug!("Rule Cache 写入: {} -> {} (cache_id: {})", domain, upstream, cache_id);
    }

    /// 清空所有缓存（reload 时调用）
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        let count = cache.len();
        cache.clear();
        info!("Rule Cache 已清空: {} 条记录", count);
    }
    
    /// 验证 rule.cache 条目是否符合当前 rules 配置
    /// 返回: (valid_entries, invalid_count)
    pub fn validate_against_rules(
        &self,
        rules: &IndexMap<String, Vec<String>>,
        lists: &HashMap<String, Vec<String>>,
    ) -> (Vec<(String, String, String)>, usize) {
        let cache = self.cache.read().unwrap();
        let mut valid_entries = Vec::new();
        let mut invalid_count = 0;
        
        for (match_domain, (upstream, cache_id)) in cache.iter() {
            // 根域名 "." 禁止参与冷启动机制
            if match_domain == "." {
                invalid_count += 1;
                debug!("Rule Cache 冷启动验证: 跳过根域名 '.' (禁止参与冷启动)");
                continue;
            }
            
            // 验证逻辑：检查 match_domain 是否在任何规则组的域名列表中
            let mut is_valid = false;
            
            // 跳过 servers 和 final 规则组
            for (group_name, list_names) in rules.iter() {
                if group_name == "servers" || group_name == "final" {
                    continue;
                }
                
                // 检查此规则组的所有列表
                for list_name in list_names {
                    if let Some(domains) = lists.get(list_name) {
                        if domains.iter().any(|d| d == match_domain || match_domain.ends_with(&format!(".{}", d))) {
                            is_valid = true;
                            break;
                        }
                    }
                }
                
                if is_valid {
                    break;
                }
            }
            
            if is_valid {
                valid_entries.push((match_domain.clone(), upstream.clone(), cache_id.clone()));
            } else {
                invalid_count += 1;
                debug!("Rule Cache 冷启动验证: 移除无效条目 {} -> {}", match_domain, upstream);
            }
        }
        
        (valid_entries, invalid_count)
    }
    
    /// 使用验证后的条目重新构建缓存
    pub fn rebuild_from_validated(&self, valid_entries: Vec<(String, String, String)>) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        
        for (match_domain, upstream, cache_id) in valid_entries {
            cache.insert(match_domain, (upstream, cache_id));
        }
        
        info!("Rule Cache 冷启动: 重建完成，共 {} 条有效记录", cache.len());
    }
    
    /// 导出缓存到文件
    pub fn export_to_file(&self) -> Result<()> {
        if let Some(ref output_path) = self.output_path {
            // 创建输出目录
            if let Some(parent) = Path::new(output_path).parent() {
                fs::create_dir_all(parent)?;
            }
            
            let cache = self.cache.read().unwrap();
            let mut file = File::create(output_path)?;
            
            // 按域名排序输出
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(domain, _)| *domain);
            
            for (domain, (upstream, cache_id)) in entries {
                // 格式: |cache ID|match domain|upstream|
                writeln!(file, "|{}|{}|{}|", cache_id, domain, upstream)?;
            }
            
            info!("Rule Cache: 已导出 {} 条缓存到 {}", cache.len(), output_path);
        }
        Ok(())
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> RuleCacheStats {
        let cache = self.cache.read().unwrap();
        RuleCacheStats {
            total: cache.len(),
        }
    }
}

/// Rule Cache 统计信息
#[derive(Debug)]
pub struct RuleCacheStats {
    /// 总记录数
    pub total: usize,
}

/// 缓存管理器
pub struct CacheManager {
    /// 规则缓存（固定名称为 "rule"）
    rule_cache: Option<Arc<RuleCache>>,
    /// 域名缓存集合 (name -> cache)
    domain_caches: HashMap<String, Arc<DomainCache>>,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(cache_configs: &HashMap<String, CacheConfig>, default_upstream: String) -> Result<Self> {
        let mut rule_cache = None;
        let mut domain_caches = HashMap::new();
        
        for (name, config) in cache_configs {
            match config.r#type {
                CacheType::Rule => {
                    if name == "rule" {
                        rule_cache = Some(Arc::new(RuleCache::from_config(config, default_upstream.clone())));
                        info!("已初始化规则缓存 '{}', 容量: {}", name, config.size);
                    } else {
                        warn!("忽略规则缓存配置 '{}': 规则缓存名称必须为 'rule'", name);
                    }
                }
                CacheType::Domain => {
                    domain_caches.insert(name.clone(), Arc::new(DomainCache::from_config(config, name.clone())));
                    info!("已初始化域名缓存 '{}', 容量: {}, min_ttl: {:?}, max_ttl: {:?}", 
                        name, config.size, config.min_ttl, config.max_ttl);
                }
            }
        }
        
        Ok(Self {
            rule_cache,
            domain_caches,
        })
    }
    
    /// 获取规则缓存
    pub fn get_rule_cache(&self) -> Option<Arc<RuleCache>> {
        self.rule_cache.clone()
    }
    
    /// 获取指定的域名缓存
    pub fn get_domain_cache(&self, name: &str) -> Option<Arc<DomainCache>> {
        self.domain_caches.get(name).cloned()
    }
    
    /// 导出所有缓存到文件
    pub fn export_all(&self) -> Result<()> {
        if let Some(ref rule_cache) = self.rule_cache {
            rule_cache.export_to_file()?;
        }
        
        for cache in self.domain_caches.values() {
            cache.export_to_file()?;
        }
        
        info!("已导出所有缓存到文件");
        Ok(())
    }
    
    /// 清理所有过期的域名缓存
    pub fn cleanup_all_expired(&self) {
        for cache in self.domain_caches.values() {
            cache.cleanup_expired();
        }
    }
    
    /// 执行冷启动流程
    /// 1. 加载并验证 rule.cache
    /// 2. 使用有效的 rule.cache 验证 domain.cache
    /// 3. 返回需要预热的域名列表
    pub async fn cold_start(
        &self,
        config: &Config,
    ) -> Result<Vec<(String, String, String, String)>> {
        info!("开始缓存冷启动流程...");
        let mut all_warm_up_list = Vec::new();
        
        // 第1步：处理 rule.cache
        if let Some(ref rule_cache) = self.rule_cache {
            info!("冷启动: 验证 Rule Cache...");
            
            // 构建域名列表映射
            let mut lists = HashMap::new();
            for (list_name, list_config) in &config.lists {
                lists.insert(list_name.clone(), list_config.domains.clone());
            }
            
            // 验证 rule.cache
            let (valid_entries, invalid_count) = rule_cache.validate_against_rules(&config.rules, &lists);
            
            if invalid_count > 0 {
                warn!("冷启动: Rule Cache 移除了 {} 条无效条目", invalid_count);
            }
            
            if valid_entries.is_empty() {
                info!("冷启动: Rule Cache 无有效条目，跳过");
            } else {
                info!("冷启动: Rule Cache 验证完成，保留 {} 条有效记录", valid_entries.len());
                
                // 重建 rule.cache
                rule_cache.rebuild_from_validated(valid_entries.clone());
                
                // 第2步：使用有效的 rule.cache 验证所有 domain.cache
                for (cache_name, domain_cache) in &self.domain_caches {
                    info!("冷启动: 验证 Domain Cache '{}'...", cache_name);
                    
                    let (_valid_records, invalid_count, warm_up_list) = 
                        domain_cache.validate_against_rule_cache(&valid_entries);
                    
                    if invalid_count > 0 {
                        warn!("冷启动: Domain Cache '{}' 移除了 {} 条无效条目", cache_name, invalid_count);
                    }
                    
                    info!("冷启动: Domain Cache '{}' 验证完成，{} 个域名需要预热", 
                        cache_name, warm_up_list.len());
                    
                    all_warm_up_list.extend(warm_up_list);
                }
            }
        } else {
            warn!("冷启动: 未配置 Rule Cache，跳过验证");
        }
        
        info!("缓存冷启动验证完成，共 {} 个域名需要预热", all_warm_up_list.len());
        Ok(all_warm_up_list)
    }
    
    /// 获取所有缓存的统计信息
    pub fn stats_all(&self) -> String {
        let mut stats = String::new();
        
        if let Some(ref rule_cache) = self.rule_cache {
            let rule_stats = rule_cache.stats();
            stats.push_str(&format!("规则缓存: {}\n", rule_stats.total));
        }
        
        for (name, cache) in &self.domain_caches {
            let cache_stats = cache.stats();
            stats.push_str(&format!("域名缓存 '{}': {}/{} (有效: {})\n", 
                name, cache_stats.total, cache_stats.total, cache_stats.valid));
        }
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::op::{Message, Query};
    use hickory_proto::rr::{Name, RecordType};
    use std::str::FromStr;

    #[test]
    fn test_domain_cache_basic() {
        let cache = DomainCache::new("test".to_string(), 10, None, None);

        // 创建测试消息
        let mut msg = Message::new();
        let name = Name::from_str("example.com.").unwrap();
        msg.add_query(Query::query(name, RecordType::A));

        // 测试插入
        cache.insert(
            "example.com".to_string(),
            "test_cache".to_string(),
            "example.com".to_string(),
            "test_upstream".to_string(),
            msg.clone(),
            300,
        );

        // 测试查询
        assert!(cache.get("example.com").is_some());
        assert!(cache.get("not-exist.com").is_none());

        // 测试统计
        let stats = cache.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.valid, 1);
    }

    #[test]
    fn test_domain_cache_ttl_limits() {
        let cache = DomainCache::new("test".to_string(), 10, Some(100), Some(500));

        // 测试 TTL 调整
        assert_eq!(cache.adjust_ttl(50), 100); // 小于 min_ttl
        assert_eq!(cache.adjust_ttl(300), 300); // 正常范围
        assert_eq!(cache.adjust_ttl(1000), 500); // 大于 max_ttl
    }

    #[test]
    fn test_domain_cache_expiry() {
        let cache = DomainCache::new("test".to_string(), 10, None, None);

        let mut msg = Message::new();
        let name = Name::from_str("example.com.").unwrap();
        msg.add_query(Query::query(name, RecordType::A));

        // 插入短 TTL 记录
        cache.insert(
            "example.com".to_string(),
            "test_cache".to_string(),
            "example.com".to_string(),
            "test_upstream".to_string(),
            msg,
            0, // 立即过期
        );

        // 稍等一下
        std::thread::sleep(Duration::from_millis(10));

        // 应该已过期
        assert!(cache.get("example.com").is_none());
    }
}
