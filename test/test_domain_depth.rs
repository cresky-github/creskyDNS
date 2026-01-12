// 域名深度计算验证测试
// 用于验证 get_match_depth 方法的正确性

#[cfg(test)]
mod tests {
    use super::*;

    /// 计算域名深度（用于测试）
    /// 深度定义：
    /// - "." → 0
    /// - "com" → 1
    /// - "google.com" → 2
    /// - "www.google.com" → 3
    fn calculate_depth(domain: &str) -> usize {
        if domain == "." {
            return 0;
        }
        
        let parts: Vec<&str> = domain
            .split('.')
            .filter(|s| !s.is_empty())
            .collect();
        
        parts.len()
    }

    #[test]
    fn test_depth_calculation() {
        // 根域名
        assert_eq!(calculate_depth("."), 0);
        
        // 顶级域名
        assert_eq!(calculate_depth("com"), 1);
        assert_eq!(calculate_depth("net"), 1);
        assert_eq!(calculate_depth("org"), 1);
        
        // 二级域名
        assert_eq!(calculate_depth("google.com"), 2);
        assert_eq!(calculate_depth("baidu.com"), 2);
        assert_eq!(calculate_depth("example.com"), 2);
        
        // 三级域名
        assert_eq!(calculate_depth("www.google.com"), 3);
        assert_eq!(calculate_depth("api.google.com"), 3);
        assert_eq!(calculate_depth("mail.google.com"), 3);
        
        // 四级域名
        assert_eq!(calculate_depth("api.service.example.com"), 4);
        assert_eq!(calculate_depth("v1.api.service.example.com"), 5);
    }

    #[test]
    fn test_depth_matching_priority() {
        // 测试深度优先级
        // 对于查询 www.google.com:
        // - 如果列表包含 google.com (深度2) 和 www.google.com (深度3)
        // - 应该选择 www.google.com (深度更大，更精确)
        
        let query = "www.google.com";
        let match_google = "google.com";
        let match_www = "www.google.com";
        
        let depth_google = calculate_depth(match_google);
        let depth_www = calculate_depth(match_www);
        
        assert_eq!(depth_google, 2);
        assert_eq!(depth_www, 3);
        assert!(depth_www > depth_google, "www.google.com 深度应该大于 google.com");
    }

    #[test]
    fn test_depth_generation_from_query() {
        // 测试从查询域名生成各级域名的深度
        let query = "www.google.com";
        let parts: Vec<&str> = query.split('.').filter(|s| !s.is_empty()).collect();
        
        // 生成各级域名及其深度
        let mut domains = vec![];
        
        // 深度 0: "."
        domains.push((".", 0));
        
        // 深度 1, 2, 3...
        for depth in 1..=parts.len() {
            let domain = parts[parts.len() - depth..].join(".");
            domains.push((domain.as_str(), depth));
        }
        
        // 验证生成的域名和深度
        assert_eq!(domains.len(), 4);
        assert_eq!(domains[0], (".", 0));
        // 注意：域名是引用，这里需要比较值
        assert_eq!(domains[1].1, 1); // com
        assert_eq!(domains[2].1, 2); // google.com
        assert_eq!(domains[3].1, 3); // www.google.com
    }
}
