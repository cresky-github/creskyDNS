use hickory_proto::op::Message;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// DNS 缓存记录
#[derive(Clone, Debug)]
pub struct CachedDnsRecord {
    /// 匹配的规则名称
    pub rule: String,
    /// 查询的域名
    pub domain: String,
    /// 过期时间点
    pub expire_at: Instant,
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
        }
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

    /// 插入缓存
    pub fn insert(&self, domain: String, rule: String, message: Message, ttl: u64) {
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

        let expire_at = Instant::now() + Duration::from_secs(adjusted_ttl);
        let record = CachedDnsRecord {
            rule: rule.clone(),
            domain: domain.clone(),
            expire_at,
            message,
        };

        cache.insert(domain.clone(), record);
        debug!(
            "Domain Cache '{}': 写入域名 {} (规则: {}, TTL: {}s)",
            self.cache_id, domain, rule, adjusted_ttl
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
/// 格式：|rule|upstream| (domain -> upstream_name)
/// 用于加速 DNS 解析，避免重复的规则匹配
#[derive(Clone)]
pub struct RuleCache {
    /// 缓存数据（domain -> upstream_name）
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl RuleCache {
    /// 创建新的 Rule Cache
    pub fn new() -> Self {
        info!("创建 Rule Cache (内存规则缓存)");
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 查询缓存
    pub fn get(&self, domain: &str) -> Option<String> {
        let cache = self.cache.read().unwrap();
        if let Some(upstream) = cache.get(domain) {
            debug!("Rule Cache 命中: {} -> {}", domain, upstream);
            return Some(upstream.clone());
        }
        debug!("Rule Cache 未命中: {}", domain);
        None
    }

    /// 插入缓存
    pub fn insert(&self, domain: String, upstream: String) {
        let mut cache = self.cache.write().unwrap();
        cache.insert(domain.clone(), upstream.clone());
        debug!("Rule Cache 写入: {} -> {}", domain, upstream);
    }

    /// 清空所有缓存（reload 时调用）
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        let count = cache.len();
        cache.clear();
        info!("Rule Cache 已清空: {} 条记录", count);
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
            "test_rule".to_string(),
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
            "test_rule".to_string(),
            msg,
            0, // 立即过期
        );

        // 稍等一下
        std::thread::sleep(Duration::from_millis(10));

        // 应该已过期
        assert!(cache.get("example.com").is_none());
    }
}
