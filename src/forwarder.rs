use crate::config::{Config, UpstreamList};
use crate::cache::{DomainCache, RuleCache};
use anyhow::Result;
use hickory_proto::op::Message;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{UdpSocket, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, warn, info};
use base64;
use std::sync::Arc;
use rustls::ClientConfig;
use reqwest::Url;

/// DNS 协议类型
#[derive(Clone, Debug)]
pub enum Protocol {
    Udp,
    Tcp,
    Dot,   // DNS over TLS
    Doh,   // DNS over HTTPS
    Doq,   // DNS over QUIC
    Rcode(u16), // 特殊协议：返回指定的 RCODE（如 rcode://3 返回 NXDOMAIN）
}

/// DNS 转发器
pub struct DnsForwarder {
    config: Config,
    rule_cache: Option<Arc<RuleCache>>,
    domain_cache: Option<Arc<DomainCache>>,
}

impl DnsForwarder {
    /// 创建新的 DNS 转发器
    pub fn new(config: Config, rule_cache: Option<Arc<RuleCache>>, domain_cache: Option<Arc<DomainCache>>) -> Result<Self> {
        Ok(Self { config, rule_cache, domain_cache })
    }

    /// 解析上游服务器地址
    fn parse_address(addr: &str) -> Result<(String, u16)> {
        // 处理 DoH/DoT/QUIC 等协议，保留完整 URL
        if addr.starts_with("https://") {
            // DoH 地址，返回完整 URL 和默认端口
            return Ok((addr.to_string(), 443));
        }
        
        // 移除协议前缀（链式移除）
        let mut cleaned = addr;
        cleaned = cleaned.strip_prefix("udp://").unwrap_or(cleaned);
        cleaned = cleaned.strip_prefix("tcp://").unwrap_or(cleaned);
        cleaned = cleaned.strip_prefix("tls://").unwrap_or(cleaned);
        cleaned = cleaned.strip_prefix("doq://").unwrap_or(cleaned);
        cleaned = cleaned.strip_prefix("quic://").unwrap_or(cleaned);
        
        if let Some((host, port_str)) = cleaned.rsplit_once(':') {
            let port = port_str.parse::<u16>()?;
            Ok((host.to_string(), port))
        } else {
            Ok((cleaned.to_string(), 53))
        }
    }

    /// 添加转发方法（带监听器名称）
    pub async fn forward_with_listener(
        &self,
        request: &Message,
        listener_name: &str,
    ) -> Result<Message> {
        self.process_request(request, Some(listener_name)).await
    }

    /// 处理UDP请求
    pub async fn handle_udp_request(
        &self,
        socket: &UdpSocket,
        addr: SocketAddr,
        data: &[u8],
    ) -> Result<()> {
        let request = crate::dns::parse_dns(data)?;
        let response = self.process_request(&request, None).await?;
        let response_data = crate::dns::encode_dns(&response)?;
        socket.send_to(&response_data, addr).await?;
        Ok(())
    }

    /// 处理TCP连接
    pub async fn handle_tcp_connection(&self, mut socket: TcpStream) -> Result<()> {
        // 读取TCP DNS消息（前2字节是长度）
        let mut len_buf = [0u8; 2];
        socket.read_exact(&mut len_buf).await?;
        let msg_len = u16::from_be_bytes(len_buf) as usize;
        
        let mut msg_buf = vec![0u8; msg_len];
        socket.read_exact(&mut msg_buf).await?;
        
        let request = crate::dns::parse_dns(&msg_buf)?;
        let response = self.process_request(&request, None).await?;
        let response_data = crate::dns::encode_dns(&response)?;
        
        // 发送TCP DNS消息（前2字节是长度）
        let len_bytes = (response_data.len() as u16).to_be_bytes();
        socket.write_all(&len_bytes).await?;
        socket.write_all(&response_data).await?;
        
        Ok(())
    }

    /// 处理DNS请求
    async fn process_request(&self, request: &Message, listener_name: Option<&str>) -> Result<Message> {
        let qname = crate::dns::get_qname(request)
            .ok_or_else(|| anyhow::anyhow!("无法获取查询名称"))?;
        
        // 记录查询请求
        let query_type = request.queries().first().map(|q| format!("{:?}", q.query_type())).unwrap_or_else(|| "UNKNOWN".to_string());
        info!("查询: {} ({}){}", qname, query_type, 
              listener_name.map(|n| format!(" [监听器: {}]", n)).unwrap_or_default());
        
        // 1. 优先检查 servers 规则（不使用缓存，直接转发）
        if let Some(listener) = listener_name {
            if self.config.rules.contains_key("servers") {
                if let Some((server_upstream, rule_name)) = self.match_server_rule(Some(listener))? {
                    let response = self.forward_to_upstream_list(request, server_upstream).await?;
                    let upstream_name = self.extract_upstream_name(&rule_name);
                    let answer_count = response.answers().len();
                    info!("响应: {} -> {} [规则: {}, 答案数: {}]", qname, upstream_name, rule_name, answer_count);
                    return Ok(response);
                }
            }
        }
        
        // 2. 查询 Rule Cache（按域名深度匹配）+ Domain Cache（复合KEY查询）
        if let Some(rule_cache) = &self.rule_cache {
            if let Some(domain_cache) = &self.domain_cache {
                // 按深度查询所有匹配的 match domain
                let matches = rule_cache.get_matches_by_depth(&qname);
                
                // 遍历匹配项，用复合KEY查询 domain cache
                for (match_domain, upstream, cache_id) in matches {
                    if let Some(cached_response) = domain_cache.get_by_key(
                        &cache_id, 
                        &match_domain, 
                        &upstream, 
                        &qname
                    ) {
                        info!("缓存命中: {} -> {} [KEY: {}|{}|{}]", 
                            qname, upstream, cache_id, match_domain, upstream);
                        return Ok(cached_response);
                    }
                }
            }
        }
        
        // 3. 根据域名匹配规则选择上游（缓存未命中时）
        let (_upstream_list, rule_name, matched_domain, response) = self.match_domain(&qname, request, listener_name).await?;
        
        // 从 rule_name 中提取 upstream_name
        // rule_name 格式: "group:matched_domain@upstream" 或 "servers:upstream" 或 "final:..."
        let upstream_list_name = if rule_name.contains('@') {
            // 格式: "group:matched_domain@upstream" -> 提取 @ 后的部分
            rule_name.split('@').last().unwrap_or(&rule_name).to_string()
        } else {
            // 格式: "servers:upstream" 或 "final:..." -> 提取 : 后的部分
            self.extract_upstream_name(&rule_name)
        };
        
        // cache_id 默认为 upstream_list_name
        let cache_id = upstream_list_name.clone();
        
        // 4. 写入 Rule Cache
        // Rule Cache 存储: match_domain -> (upstream_name, cache_id)
        // 注意：servers 规则和 final 规则不参与缓存
        if let Some(rule_cache) = &self.rule_cache {
            if !rule_name.starts_with("servers:") && !rule_name.starts_with("final:") {
                let match_domain_for_cache = if matched_domain.is_empty() { 
                    ".".to_string()  // 未匹配到具体域名，使用根域名
                } else { 
                    matched_domain.clone() 
                };
                rule_cache.insert(match_domain_for_cache, upstream_list_name.clone(), cache_id.clone());
            }
        }
        
        // 5. 写入 Domain Cache
        // 注意：servers 规则和 final 规则不参与缓存
        if let Some(cache) = &self.domain_cache {
            if !rule_name.starts_with("servers:") && !rule_name.starts_with("final:") {
                // 从响应中提取最小 TTL
                let ttl = self.extract_min_ttl(&response);
                // Domain Cache 使用匹配到的域名作为规则标识（链接到 rule.cache）
                let match_domain_str = if matched_domain.is_empty() { ".".to_string() } else { matched_domain.clone() };
                cache.insert(
                    qname.clone(),
                    cache_id.clone(),
                    match_domain_str,
                    upstream_list_name.clone(),
                    response.clone(),
                    ttl
                );
            }
        }
        
        // 记录响应结果
        let answer_count = response.answers().len();
        info!("响应: {} -> {} [规则: {}, 答案数: {}]", qname, upstream_list_name, rule_name, answer_count);
        
        Ok(response)
    }

    /// 根据域名匹配规则（返回 upstream 和规则名称）
    async fn match_domain(&self, domain: &str, request: &Message, listener_name: Option<&str>) -> Result<(&UpstreamList, String, String, Message)> {
        // 按 yaml 中 rules 的顺序遍历所有规则组
        for (group_name, rules) in &self.config.rules {
            // 跳过 final 规则，它在 handle_no_match 中处理
            if group_name == "final" {
                continue;
            }
            
            // servers 规则：按监听器匹配
            if group_name == "servers" {
                if let Some((server_upstream, rule_name)) = self.match_server_rule(listener_name)? {
                    let response = self.forward_to_upstream_list(request, server_upstream).await?;
                    return Ok((server_upstream, rule_name, String::new(), response));
                }
                continue;
            }
            
            // 其他规则组：按域名匹配
            if let Some((upstream_name, matched_domain)) = self.find_best_match_in_group(domain, rules) {
                let upstream = self.config.upstreams.get(&upstream_name)
                    .ok_or_else(|| anyhow::anyhow!("规则组 '{}' 中的上游 '{}' 未找到", group_name, upstream_name))?;
                let rule_name = format!("{}:{}@{}", group_name, matched_domain, upstream_name);  // 格式: group:matched_domain@upstream
                let response = self.forward_to_upstream_list(request, upstream).await?;
                return Ok((upstream, rule_name, matched_domain, response));
            }
        }
        
        // 所有规则组都未匹配，尝试 Final 规则或全局默认上游
        self.handle_no_match(domain, request, listener_name).await
    }

    /// 处理未匹配任何规则的情况
    async fn handle_no_match(&self, domain: &str, request: &Message, _listener_name: Option<&str>) -> Result<(&UpstreamList, String, String, Message)> {
        // 如果配置了 Final 规则，使用 Final 规则处理
        if let Some(final_rule) = &self.config.final_rule {
            debug!("域名 {} 未匹配任何规则，触发 Final 规则", domain);
            return self.process_final_rule(domain, request, final_rule).await;
        }

        // 如果没有 Final 规则，使用默认上游降级
        let default_upstream_names = vec!["default_dns", "cn_dns", "direct_dns", "global_dns"];
        
        for upstream_name in default_upstream_names {
            if let Some(upstream) = self.config.upstreams.get(upstream_name) {
                debug!("域名 {} 未匹配任何规则，使用默认上游 '{}'", domain, upstream_name);
                let rule_name = format!("default:{}", upstream_name);
                let response = self.forward_to_upstream_list(request, upstream).await?;
                return Ok((upstream, rule_name, String::new(), response));
            }
        }
        
        // 如果连默认上游都没有，使用第一个可用的上游
        if let Some((name, upstream)) = self.config.upstreams.iter().next() {
            debug!("域名 {} 未匹配任何规则，使用第一个可用上游 '{}'", domain, name);
            let rule_name = format!("fallback:{}", name);
            let response = self.forward_to_upstream_list(request, upstream).await?;
            return Ok((upstream, rule_name, String::new(), response));
        }

        // 如果没有任何上游，返回错误
        anyhow::bail!("域名 {} 未匹配到任何规则，且没有可用的默认上游", domain)
    }

    /// 匹配服务器规则（按监听器实例）
    fn match_server_rule(&self, listener_name: Option<&str>) -> Result<Option<(&UpstreamList, String)>> {
        // 如果没有 listener_name 或者没有配置 servers 规则，返回 None
        let listener_name = match listener_name {
            Some(name) => {
                debug!("检查监听器 '{}' 的 servers 规则", name);
                name
            },
            None => {
                debug!("未提供 listener_name，跳过 servers 规则匹配");
                return Ok(None);
            }
        };
        
        // 获取 servers 规则组
        let servers_rules = match self.config.rules.get("servers") {
            Some(rules) => {
                debug!("找到 servers 规则组，共 {} 条规则", rules.len());
                rules
            },
            None => {
                debug!("配置中没有 servers 规则组");
                return Ok(None);
            }
        };
        
        // 遍历 servers 规则，查找匹配的监听器
        for rule in servers_rules {
            // 规则格式: "listener_name,upstream_name"
            let parts: Vec<&str> = rule.split(',').map(|s| s.trim()).collect();
            if parts.len() != 2 {
                warn!("无效的 servers 规则格式: '{}', 应为 'listener_name,upstream_name'", rule);
                continue;
            }
            
            let rule_listener = parts[0];
            let upstream_name = parts[1];
            
            debug!("检查规则 '{}' - 规则监听器: '{}', 当前监听器: '{}'", rule, rule_listener, listener_name);
            
            // 匹配监听器名称
            if rule_listener == listener_name {
                // 找到上游配置
                let upstream = self.config.upstreams.get(upstream_name)
                    .ok_or_else(|| anyhow::anyhow!("servers 规则中的上游 '{}' 未找到", upstream_name))?;
                let rule_name = format!("servers:{}", upstream_name);
                info!("✓ 监听器 '{}' 匹配到 servers 规则，使用上游 '{}'", listener_name, upstream_name);
                return Ok(Some((upstream, rule_name)));
            }
        }
        
        debug!("监听器 '{}' 未匹配到任何 servers 规则", listener_name);
        Ok(None)
    }

    /// 匹配域名规则（按规则组顺序）
    /// 
    /// 规则说明：
    /// 1. 按 YAML 配置文件中定义的顺序遍历规则组
    /// 2. 在每个规则组内，同时匹配所有list
    /// 3. 按domain suffix方式匹配
    /// 4. 取域名深度最大的规则（深度定义：. = 0, com = 1, google.com = 2, www.google.com = 3）
    /// 5. 如果深度相同，取group内最后一个匹配的规则
    /// 6. 一旦某个规则组有匹配，立即返回，不再检查后续规则组
    /// 
    /// 返回: (upstream, rule_name, matched_domain)
    fn match_domain_rules(&self, domain: &str) -> Result<(&UpstreamList, String, String)> {
        // 按 YAML 顺序遍历所有规则组（IndexMap 保证顺序）
        for (group_name, rules) in &self.config.rules {
            // 跳过 'final' 规则组，它在最后单独处理
            if group_name == "final" {
                continue;
            }
            
            // 在每个规则组内找到最优匹配
            if let Some((upstream_list, matched_domain)) = self.find_best_match_in_group(domain, rules) {
                debug!("域名 {} 在规则组 '{}' 中匹配到上游 '{}', 匹配域名: {}", domain, group_name, upstream_list, matched_domain);
                
                // 记录命中（servers 规则组除外）
                if group_name != "servers" {
                    self.record_hit(domain, &upstream_list, &matched_domain);
                }
                
                let upstream = self.config.upstreams.get(&upstream_list)
                    .ok_or_else(|| anyhow::anyhow!("上游列表 '{}' 未找到", upstream_list))?;
                let rule_name = format!("{}:{}", group_name, upstream_list);
                return Ok((upstream, rule_name, matched_domain));
            }
        }

        // 未匹配任何规则，返回 None 表示需要走 Final 规则或默认上游
        anyhow::bail!("NO_MATCH")
    }

    /// 在单个group内找到最优匹配
    /// 
    /// 同时评估所有规则，按深度降序、rule_index降序排序，取第一个匹配
    /// 
    /// 深度定义（越大越精确）：
    /// - 深度0: `.` (根域名)
    /// - 深度1: `com` (顶级域名)
    /// - 深度2: `google.com` (二级域名)  
    /// - 深度3: `www.google.com` (三级域名)
    /// 
    /// 返回: Some((upstream_list, matched_domain)) 或 None
    fn find_best_match_in_group(
        &self,
        domain: &str,
        rules: &[String],
    ) -> Option<(String, String)> {
        let mut matches: Vec<(usize, usize, String, String)> = Vec::new(); // (depth, rule_index, upstream_list, matched_domain)

        // 同时评估所有规则
        for (rule_index, rule_str) in rules.iter().enumerate() {
            if let Some((domain_list, upstream_list)) = self.parse_rule_string(rule_str) {
                if let Some((depth, matched_domain)) = self.get_match_depth(domain, &domain_list) {
                    matches.push((depth, rule_index, upstream_list, matched_domain));
                }
            }
        }

        if matches.is_empty() {
            return None;
        }

        // 按深度降序排序，深度相同则按rule_index降序排序
        // 深度大的优先，深度相同则选择后面的规则
        matches.sort_by(|a, b| {
            match b.0.cmp(&a.0) {
                std::cmp::Ordering::Equal => b.1.cmp(&a.1), // 深度相同，取rule_index大的（后面的）
                other => other, // 深度不同，深度大的优先
            }
        });

        // 返回最优匹配的上游列表名和匹配的域名
        matches.first().map(|(_, _, upstream_list, matched_domain)| (upstream_list.clone(), matched_domain.clone()))
    }

    /// 解析规则字符串 "domain_list,upstream_list"
    fn parse_rule_string(&self, rule_str: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = rule_str.split(',').collect();
        if parts.len() == 2 {
            Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
        } else {
            None
        }
    }

    /// 获取域名与规则的匹配深度
    /// 
    /// 域名深度定义：
    /// - 深度0: `.` (根域名)
    /// - 深度1: `com` (顶级域名)
    /// - 深度2: `google.com` (二级域名)
    /// - 深度3: `www.google.com` (三级域名)
    /// - 以此类推
    /// 
    /// 深度越大表示匹配越精确，优先级越高
    /// 
    /// 返回: Some((depth, matched_domain)) 或 None
    /// 
    /// 示例：
    /// ```
    /// // 查询 www.google.com，规则列表包含 google.com
    /// get_match_depth("www.google.com", "list") → Some((2, "google.com"))
    /// 
    /// // 查询 api.google.com，规则列表包含 google.com 和 api.google.com
    /// get_match_depth("api.google.com", "list") → Some((3, "api.google.com"))
    /// ```
    fn get_match_depth(&self, domain: &str, domain_list_name: &str) -> Option<(usize, String)> {
        let domain_list = self.config.lists.get(domain_list_name)?;
        let domain_parts: Vec<&str> = domain.split('.').filter(|s| !s.is_empty()).collect();

        // 检查各级域名是否匹配（从最深到最浅）
        // 优先匹配深度大的（更精确的）域名
        for depth in (0..=domain_parts.len()).rev() {
            let check_domain = if depth == 0 {
                ".".to_string() // 根域名，匹配所有
            } else {
                // 取后 depth 个部分组成域名
                // 例如: ["www", "google", "com"], depth=2 → "google.com"
                domain_parts[domain_parts.len() - depth..].join(".")
            };

            if domain_list.domains.contains(&check_domain) {
                return Some((depth, check_domain));
            }
        }

        None
    }

    /// 从 rule_name 中提取 upstream 名称
    /// rule_name 格式: "group_name:upstream_name" 或 "cached:upstream_name"
    fn extract_upstream_name(&self, rule_name: &str) -> String {
        if let Some(colon_pos) = rule_name.rfind(':') {
            rule_name[colon_pos + 1..].to_string()
        } else {
            rule_name.to_string()
        }
    }

    /// 从 DNS 响应中提取最小 TTL
    fn extract_min_ttl(&self, response: &Message) -> u64 {
        let mut min_ttl = 300; // 默认 5 分钟

        // 从答案、权威和附加记录中提取 TTL
        for record in response.answers().iter()
            .chain(response.name_servers().iter())
            .chain(response.additionals().iter())
        {
            let ttl = record.ttl();
            if ttl > 0 && ttl < min_ttl {
                min_ttl = ttl;
            }
        }

        min_ttl as u64
    }

    /// 转发到上游列表
    async fn forward_to_upstream_list(&self, request: &Message, upstream_list: &UpstreamList) -> Result<Message> {
        // 目前只使用第一个上游地址
        let upstream_addr = upstream_list.addr.first()
            .ok_or_else(|| anyhow::anyhow!("上游列表为空"))?;
        
        let protocol = Self::parse_protocol(upstream_addr)?;
        
        match protocol {
            Protocol::Rcode(rcode) => {
                // 特殊协议：直接返回指定的 RCODE 响应
                debug!("使用 rcode 协议返回 RCODE: {}", rcode);
                Ok(Self::create_rcode_response(request, rcode))
            }
            Protocol::Udp => self.forward_udp(request, upstream_addr).await,
            Protocol::Tcp => self.forward_tcp(request, upstream_addr).await,
            Protocol::Dot => self.forward_dot(request, upstream_addr, upstream_list.bootstrap.as_ref(), upstream_list.proxy.as_ref()).await,
            Protocol::Doh => self.forward_doh(request, upstream_addr, upstream_list.bootstrap.as_ref(), upstream_list.proxy.as_ref()).await,
            // DoQ 基于 UDP，SOCKS5 代理主要支持 TCP，暂不支持
            Protocol::Doq => self.forward_doq(request, upstream_addr, upstream_list.bootstrap.as_ref()).await,
        }
    }

    /// 解析协议类型
    fn parse_protocol(addr: &str) -> Result<Protocol> {
        if addr.starts_with("rcode://") {
            // 格式：rcode://0-65535 或 rcode://NXDOMAIN
            let rcode_str = addr.strip_prefix("rcode://").unwrap_or("3");
            let rcode = match rcode_str.to_uppercase().as_str() {
                "NOERROR" => 0,
                "FORMERR" => 1,
                "SERVFAIL" => 2,
                "NXDOMAIN" => 3,
                "NOTIMP" => 4,
                "REFUSED" => 5,
                _ => rcode_str.parse::<u16>().unwrap_or(3), // 默认 NXDOMAIN
            };
            Ok(Protocol::Rcode(rcode))
        } else if addr.starts_with("quic://") {
            Ok(Protocol::Doq)
        } else if addr.starts_with("doq://") {
            Ok(Protocol::Doq)
        } else if addr.starts_with("https://") {
            Ok(Protocol::Doh)
        } else if addr.starts_with("tls://") {
            Ok(Protocol::Dot)
        } else if addr.starts_with("tcp://") {
            Ok(Protocol::Tcp)
        } else if addr.starts_with("udp://") {
            Ok(Protocol::Udp)
        } else {
            // 默认当作UDP处理
            Ok(Protocol::Udp)
        }
    }
    
    /// 创建指定 RCODE 的响应
    /// 
    /// RCODE 常用值：
    /// - 0: NOERROR (成功，但无数据返回)
    /// - 1: FORMERR (格式错误)
    /// - 2: SERVFAIL (服务器失败)
    /// - 3: NXDOMAIN (域名不存在)
    /// - 4: NOTIMP (未实现)
    /// - 5: REFUSED (拒绝查询)
    fn create_rcode_response(request: &Message, rcode: u16) -> Message {
        use hickory_proto::op::{ResponseCode, Header, MessageType};
        
        let mut response = Message::new();
        let mut header = Header::new();
        
        // 复制请求的 ID
        header.set_id(request.id());
        header.set_message_type(MessageType::Response);
        header.set_op_code(request.op_code());
        
        // 设置响应代码
        let response_code = match rcode {
            0 => ResponseCode::NoError,
            1 => ResponseCode::FormErr,
            2 => ResponseCode::ServFail,
            3 => ResponseCode::NXDomain,
            4 => ResponseCode::NotImp,
            5 => ResponseCode::Refused,
            _ => ResponseCode::ServFail, // 未知代码默认为 ServFail
        };
        header.set_response_code(response_code);
        
        // 设置标志
        header.set_authoritative(false);
        header.set_recursion_desired(request.recursion_desired());
        header.set_recursion_available(true);
        
        response.set_header(header);
        
        // 复制查询部分
        for query in request.queries() {
            response.add_query(query.clone());
        }
        
        debug!("创建 RCODE {} 响应: {}", rcode, response_code);
        response
    }

    /// UDP 转发
    async fn forward_udp(&self, request: &Message, upstream_addr: &str) -> Result<Message> {
        // 检查是否为 HTTPS/DoH 地址
        if upstream_addr.starts_with("https://") {
            anyhow::bail!("UDP 转发不支持 HTTPS/DoH 地址: {}", upstream_addr);
        }
        
        let (host, port) = Self::parse_address(upstream_addr)?;
        let upstream_socket_addr: SocketAddr = format!("{}:{}", host, port).parse()
            .map_err(|e| anyhow::anyhow!("无法解析地址 '{}:{}': {}", host, port, e))?;

        // 创建 UDP 套接字
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let timeout = Duration::from_secs(self.config.timeout_secs);

        // 发送请求
        let request_data = request.to_vec()?;
        socket.send_to(&request_data, upstream_socket_addr).await?;
        
        debug!("UDP 已向 {} 发送 DNS 查询", upstream_addr);

        // 接收响应
        let mut buf = [0u8; 512];
        let result = tokio::time::timeout(timeout, socket.recv_from(&mut buf)).await;

        match result {
            Ok(Ok((len, _))) => {
                let response = Message::from_vec(&buf[..len])?;
                debug!("UDP 收到来自 {} 的响应", upstream_addr);
                Ok(response)
            }
            Ok(Err(e)) => {
                anyhow::bail!("UDP 接收响应失败: {}", e);
            }
            Err(_) => {
                anyhow::bail!("UDP DNS 查询超时 ({}s)", self.config.timeout_secs);
            }
        }
    }

    /// TCP 转发
    async fn forward_tcp(&self, request: &Message, upstream_addr: &str) -> Result<Message> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        // 检查是否为 HTTPS/DoH 地址
        if upstream_addr.starts_with("https://") {
            anyhow::bail!("TCP 转发不支持 HTTPS/DoH 地址: {}", upstream_addr);
        }

        let (host, port) = Self::parse_address(upstream_addr)?;
        let upstream_socket_addr: SocketAddr = format!("{}:{}", host, port).parse()
            .map_err(|e| anyhow::anyhow!("无法解析地址 '{}:{}': {}", host, port, e))?;

        // 连接到上游服务器
        let mut stream = TcpStream::connect(upstream_socket_addr).await?;
        let timeout = Duration::from_secs(self.config.timeout_secs);

        // 发送长度前缀 + 请求数据
        let request_data = request.to_vec()?;
        let len_prefix = (request_data.len() as u16).to_be_bytes();
        
        tokio::time::timeout(timeout, stream.write_all(&len_prefix)).await??;
        tokio::time::timeout(timeout, stream.write_all(&request_data)).await??;
        
        debug!("TCP 已向 {} 发送 DNS 查询", upstream_addr);

        // 读取长度前缀
        let mut len_buf = [0u8; 2];
        tokio::time::timeout(timeout, stream.read_exact(&mut len_buf)).await??;
        let msg_len = u16::from_be_bytes(len_buf) as usize;

        if msg_len == 0 || msg_len > 4096 {
            anyhow::bail!("TCP 无效的消息长度: {}", msg_len);
        }

        // 读取响应数据
        let mut buf = vec![0u8; msg_len];
        tokio::time::timeout(timeout, stream.read_exact(&mut buf)).await??;

        let response = Message::from_vec(&buf)?;
        debug!("TCP 收到来自 {} 的响应", upstream_addr);
        Ok(response)
    }

    /// DoT (DNS over TLS) 转发
    /// DoT (DNS over TLS) 转发
    async fn forward_dot(&self, request: &Message, upstream_addr: &str, bootstrap: Option<&Vec<String>>, proxy: Option<&String>) -> Result<Message> {
        // 提取用户查询的域名（用于日志）
        let query_name = request.queries().first()
            .map(|q| q.name().to_utf8())
            .unwrap_or_else(|| "<unknown>".to_string());
        debug!("[DoT] 开始处理查询: {} -> {}", query_name, upstream_addr);
        
        // 提取主机名和端口
        let addr_part = upstream_addr.strip_prefix("tls://")
            .ok_or_else(|| anyhow::anyhow!("无效的 DoT 地址"))?
            .to_string();
        
        let (host, port) = if let Some(colon_pos) = addr_part.rfind(':') {
            let (h, p) = addr_part.split_at(colon_pos);
            (h.to_string(), p[1..].parse::<u16>()?)
        } else {
            (addr_part, 853) // DoT 默认端口
        };

        // 如果配置了 bootstrap DNS，使用 bootstrap 解析域名获取 IP
        let resolved_host = if let Some(bootstrap_servers) = bootstrap {
            match self.resolve_with_bootstrap(&host, bootstrap_servers).await {
                Ok(ips) => {
                    let ip = ips.first()
                        .ok_or_else(|| anyhow::anyhow!("Bootstrap 解析未返回 IP 地址"))?;
                    debug!("[Bootstrap] DoT 服务器 {} -> IP: {}", host, ip);
                    ip.clone()
                }
                Err(e) => {
                    warn!("DoT Bootstrap DNS 解析失败: {}, 回退到系统 DNS", e);
                    host.clone()
                }
            }
        } else {
            debug!("DoT 未配置 bootstrap DNS，使用系统 DNS 解析");
            host.clone()
        };

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let socket_addr = format!("{}:{}", resolved_host, port);

        // 根据是否配置代理，选择连接方式
        let stream = if let Some(proxy_url) = proxy {
            // 通过 SOCKS5 代理连接
            debug!("DoT 通过代理 {} 连接到 {}", proxy_url, socket_addr);
            
            // 解析代理地址
            let proxy_addr = proxy_url
                .strip_prefix("socks5://")
                .or_else(|| proxy_url.strip_prefix("socks://"))
                .unwrap_or(proxy_url);
            
            tokio_socks::tcp::Socks5Stream::connect(proxy_addr, socket_addr.as_str())
                .await
                .map_err(|e| anyhow::anyhow!("DoT SOCKS5 连接失败: {}", e))?
                .into_inner()
        } else {
            // 直接连接
            debug!("DoT 直连到 {}", socket_addr);
            TcpStream::connect(&socket_addr).await?
        };

        let root_store = Self::load_root_certs();
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        );

        // 使用原始主机名作为 SNI（即使连接的是 IP）
        let server_name = host.as_str().try_into()
            .map_err(|_| anyhow::anyhow!("无效的服务器名称"))?;
        
        if bootstrap.is_some() {
            debug!("[DoT] 使用 IP 连接，设置 SNI: {}", host);
        }
        
        let connector = tokio_rustls::TlsConnector::from(client_config);
        let mut tls_stream = connector.connect(server_name, stream).await?;
        
        // 发送 DNS 请求（长度前缀 + DNS 消息）
        let request_data = request.to_vec()?;
        let len_prefix = (request_data.len() as u16).to_be_bytes();
        
        tokio::time::timeout(timeout, tls_stream.write_all(&len_prefix)).await??;
        tokio::time::timeout(timeout, tls_stream.write_all(&request_data)).await??;
        
        debug!("DoT 已向 {} 发送 DNS 查询", upstream_addr);

        // 接收响应
        let mut len_buf = [0u8; 2];
        tokio::time::timeout(timeout, tls_stream.read_exact(&mut len_buf)).await??;
        let msg_len = u16::from_be_bytes(len_buf) as usize;

        if msg_len == 0 || msg_len > 4096 {
            anyhow::bail!("DoT 无效的消息长度: {}", msg_len);
        }

        let mut buf = vec![0u8; msg_len];
        tokio::time::timeout(timeout, tls_stream.read_exact(&mut buf)).await??;

        let response = Message::from_vec(&buf)?;
        debug!("DoT 收到来自 {} 的响应", upstream_addr);
        Ok(response)
    }

    /// 加载根证书
    fn load_root_certs() -> rustls::RootCertStore {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        root_store
    }

    /// 使用 Bootstrap DNS 解析域名
    /// 
    /// 返回解析到的 IP 地址列表
    async fn resolve_with_bootstrap(&self, domain: &str, bootstrap_servers: &[String]) -> Result<Vec<String>> {
        use hickory_proto::op::{Message, Query, OpCode};
        use hickory_proto::rr::{Name, RecordType, RData};
        use std::str::FromStr;
        
        // 尝试每个 bootstrap DNS 服务器
        for bootstrap_addr in bootstrap_servers {
            debug!("[Bootstrap] 使用 {} 解析 DNS 服务器域名: {}", bootstrap_addr, domain);
            
            // 构造 DNS 查询（A 记录）
            let domain_name = match Name::from_str(&format!("{}.", domain)) {
                Ok(name) => name,
                Err(e) => {
                    warn!("Bootstrap 解析: 域名格式错误 '{}': {}", domain, e);
                    continue;
                }
            };
            
            let mut request = Message::new();
            request.set_id(rand::random());
            request.set_op_code(OpCode::Query);
            request.set_recursion_desired(true);
            request.add_query(Query::query(domain_name, RecordType::A));
            
            // 使用 UDP 查询 bootstrap DNS
            match self.forward_udp(&request, bootstrap_addr).await {
                Ok(response) => {
                    // 提取 A 记录中的 IP 地址
                    let mut ips = Vec::new();
                    for answer in response.answers() {
                        if let Some(RData::A(ipv4)) = answer.data() {
                            ips.push(ipv4.to_string());
                        }
                    }
                    
                    if !ips.is_empty() {
                        debug!("Bootstrap DNS 解析成功: {} -> {:?}", domain, ips);
                        return Ok(ips);
                    } else {
                        debug!("Bootstrap DNS {} 未返回 A 记录", bootstrap_addr);
                        continue;
                    }
                }
                Err(e) => {
                    warn!("Bootstrap DNS {} 查询失败: {}", bootstrap_addr, e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("所有 Bootstrap DNS 服务器都无法解析域名: {}", domain)
    }

    /// DoH (DNS over HTTPS) 转发
    async fn forward_doh(&self, request: &Message, upstream_addr: &str, bootstrap: Option<&Vec<String>>, proxy: Option<&String>) -> Result<Message> {
        let timeout = Duration::from_secs(self.config.timeout_secs);
        
        // 提取用户查询的域名（用于日志）
        let query_name = request.queries().first()
            .map(|q| q.name().to_utf8())
            .unwrap_or_else(|| "<unknown>".to_string());
        debug!("[DoH] 开始处理查询: {} -> {}", query_name, upstream_addr);
        
        // 解析 URL，提取域名和路径
        let url = upstream_addr.to_string();
        let (domain, port, path) = {
            let url_without_scheme = url
                .strip_prefix("https://")
                .ok_or_else(|| anyhow::anyhow!("无效的 DoH URL: {}", url))?;
            
            let (host_port, path) = url_without_scheme
                .split_once('/')
                .map(|(h, p)| (h, format!("/{}", p)))
                .unwrap_or((url_without_scheme, "/dns-query".to_string()));
            
            let (domain, port) = if let Some((d, p)) = host_port.split_once(':') {
                (d.to_string(), p.parse::<u16>()?)
            } else {
                (host_port.to_string(), 443)
            };
            
            (domain, port, path)
        };
        
        // 如果配置了 bootstrap DNS，使用 bootstrap 解析域名并获取 IP 地址
        let (target_host, original_domain) = if let Some(bootstrap_servers) = bootstrap {
            match self.resolve_with_bootstrap(&domain, bootstrap_servers).await {
                Ok(ips) => {
                    let ip = ips.first()
                        .ok_or_else(|| anyhow::anyhow!("Bootstrap 解析未返回 IP 地址"))?;
                    debug!("[Bootstrap] DoH 服务器 {} -> IP: {}", domain, ip);
                    (ip.clone(), Some(domain.clone())) // 返回 IP 和原始域名
                }
                Err(e) => {
                    warn!("Bootstrap DNS 解析失败: {}, 回退到系统 DNS", e);
                    (domain.clone(), None) // 使用域名连接，无需特殊处理
                }
            }
        } else {
            debug!("DoH 未配置 bootstrap DNS，使用系统 DNS 解析");
            (domain.clone(), None)
        };

        // 将 DNS 消息编码为 base64
        let request_data = request.to_vec()?;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        let dns_query = URL_SAFE_NO_PAD.encode(&request_data);

        // 如果使用 bootstrap，需要手动建立 TLS 连接以控制 SNI
        if let Some(sni_domain) = original_domain {
            debug!("[DoH] 使用 bootstrap: 连接到 {}:{}, SNI: {}", target_host, port, sni_domain);
            
            // 解析目标地址
            let socket_addr: SocketAddr = format!("{}:{}", target_host, port).parse()?;
            
            // 建立 TCP 连接
            let stream = tokio::time::timeout(
                timeout,
                TcpStream::connect(socket_addr)
            ).await
            .map_err(|_| anyhow::anyhow!("连接超时"))??;
            
            debug!("[DoH] TCP 连接已建立到 {}", socket_addr);
            
            // 配置 TLS
            let mut root_store = rustls::RootCertStore::empty();
            root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            }));
            
            let tls_config = Arc::new(ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth());
            
            // 创建 TLS 连接器并设置 SNI
            let connector = tokio_rustls::TlsConnector::from(tls_config);
            let server_name = rustls::ServerName::try_from(sni_domain.as_str())
                .map_err(|e| anyhow::anyhow!("无效的服务器名称: {}", e))?;
            
            let mut tls_stream = tokio::time::timeout(
                timeout,
                connector.connect(server_name, stream)
            ).await
            .map_err(|_| anyhow::anyhow!("TLS 握手超时"))??;
            
            debug!("[DoH] TLS 握手完成，SNI: {}", sni_domain);
            
            // 构建 HTTP/1.1 请求
            let request_line = format!("GET {}?dns={} HTTP/1.1\r\n", path, dns_query);
            let headers = format!(
                "Host: {}\r\nAccept: application/dns-message\r\nConnection: close\r\n\r\n",
                sni_domain
            );
            let http_request = format!("{}{}", request_line, headers);
            
            debug!("[DoH] 发送 HTTP 请求: GET {}?dns=...", path);
            
            // 发送 HTTP 请求
            tls_stream.write_all(http_request.as_bytes()).await?;
            
            // 读取 HTTP 响应
            let mut response_data = Vec::new();
            tokio::time::timeout(
                timeout,
                tls_stream.read_to_end(&mut response_data)
            ).await
            .map_err(|_| anyhow::anyhow!("读取响应超时"))??;
            
            debug!("[DoH] 收到响应，大小: {} 字节", response_data.len());
            
            // 解析 HTTP 响应（在字节级别分割 header 和 body）
            let header_separator = b"\r\n\r\n";
            let body_start = response_data
                .windows(header_separator.len())
                .position(|window| window == header_separator)
                .ok_or_else(|| anyhow::anyhow!("无效的 HTTP 响应格式：未找到 header 分隔符"))?
                + header_separator.len();
            
            // 提取 header（用于状态码检查）
            let header_bytes = &response_data[..body_start - header_separator.len()];
            let header_str = String::from_utf8_lossy(header_bytes);
            
            // 检查 HTTP 状态码
            let status_line = header_str.lines().next()
                .ok_or_else(|| anyhow::anyhow!("缺少 HTTP 状态行"))?;
            
            if !status_line.contains("200") {
                anyhow::bail!("DoH 请求失败: {}", status_line);
            }
            
            // 提取 body（二进制数据）
            let body_bytes = &response_data[body_start..];
            
            // 解析 DNS 响应（直接从二进制数据）
            let message = Message::from_vec(body_bytes)?;
            
            debug!("DoH 收到来自 {} 的响应", upstream_addr);
            Ok(message)
        } else {
            // 不使用 bootstrap，使用 reqwest 标准方式
            debug!("[DoH] 标准连接到 {}:{}", domain, port);
            
            let client = reqwest::Client::builder()
                .timeout(timeout)
                .use_rustls_tls()
                .build()?;
            
            let response = client
                .get(&url)
                .query(&[("dns", &dns_query)])
                .header("Accept", "application/dns-message")
                .send()
                .await?;
            
            if !response.status().is_success() {
                anyhow::bail!("DoH 请求失败: HTTP {}", response.status());
            }
            
            let response_data = response.bytes().await?;
            let message = Message::from_vec(&response_data)?;
            
            debug!("DoH 收到来自 {} 的响应", upstream_addr);
            Ok(message)
        }
    }

    /// DoQ (DNS over QUIC) 转发
    async fn forward_doq(&self, request: &Message, upstream_addr: &str, bootstrap: Option<&Vec<String>>) -> Result<Message> {
        // 提取用户查询的域名（用于日志）
        let query_name = request.queries().first()
            .map(|q| q.name().to_utf8())
            .unwrap_or_else(|| "<unknown>".to_string());
        debug!("[DoQ] 开始处理查询: {} -> {}", query_name, upstream_addr);
        
        // 提取主机名和端口
        let addr_part = upstream_addr.strip_prefix("doq://")
            .or_else(|| upstream_addr.strip_prefix("quic://"))
            .ok_or_else(|| anyhow::anyhow!("无效的 DoQ 地址"))?
            .to_string();
        
        let (host, port) = if let Some(colon_pos) = addr_part.rfind(':') {
            let (h, p) = addr_part.split_at(colon_pos);
            (h.to_string(), p[1..].parse::<u16>()?)
        } else {
            (addr_part, 784) // DoQ 默认端口
        };

        // 如果配置了 bootstrap DNS，使用 bootstrap 解析域名获取 IP
        let resolved_host = if let Some(bootstrap_servers) = bootstrap {
            match self.resolve_with_bootstrap(&host, bootstrap_servers).await {
                Ok(ips) => {
                    let ip = ips.first()
                        .ok_or_else(|| anyhow::anyhow!("Bootstrap 解析未返回 IP 地址"))?;
                    debug!("[Bootstrap] DoQ 服务器 {} -> IP: {}", host, ip);
                    ip.clone()
                }
                Err(e) => {
                    warn!("DoQ Bootstrap DNS 解析失败: {}, 回退到系统 DNS", e);
                    host.clone()
                }
            }
        } else {
            debug!("DoQ 未配置 bootstrap DNS，使用系统 DNS 解析");
            host.clone()
        };

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let socket_addr = format!("{}:{}", resolved_host, port).parse::<SocketAddr>()?;

        // 创建 QUIC 客户端配置
        let client_config = quinn::ClientConfig::with_native_roots();

        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        let mut endpoint = quinn::Endpoint::new(
            Default::default(),
            None,
            socket,
            Arc::new(quinn::TokioRuntime),
        )?;
        endpoint.set_default_client_config(client_config);

        // 连接到 QUIC 服务器（使用原始域名作为 SNI）
        if bootstrap.is_some() {
            debug!("[DoQ] 使用 IP {} 连接，设置 SNI: {}", resolved_host, host);
        }
        
        let connection = endpoint
            .connect(socket_addr, &host)?
            .await
            .map_err(|e| anyhow::anyhow!("QUIC 连接失败: {}", e))?;

        // 打开单向流发送 DNS 查询
        let mut send = connection
            .open_uni()
            .await
            .map_err(|e| anyhow::anyhow!("打开 QUIC 流失败: {}", e))?;

        let request_data = request.to_vec()?;
        send.write_all(&request_data)
            .await
            .map_err(|e| anyhow::anyhow!("写入 QUIC 数据失败: {}", e))?;

        send.finish()
            .await
            .map_err(|e| anyhow::anyhow!("关闭 QUIC 发送流失败: {}", e))?;

        debug!("DoQ 已向 {} 发送 DNS 查询", upstream_addr);

        // 通过双向流接收响应
        let (_send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| anyhow::anyhow!("打开 QUIC 双向流失败: {}", e))?;

        let _response_data: Vec<u8> = Vec::new();
        let result = tokio::time::timeout(
            timeout,
            recv.read_to_end(4096)
        ).await;

        match result {
            Ok(Ok(data)) => {
                let message = Message::from_vec(&data)?;
                debug!("DoQ 收到来自 {} 的响应", upstream_addr);
                Ok(message)
            }
            Ok(Err(e)) => {
                anyhow::bail!("DoQ 接收响应失败: {}", e);
            }
            Err(_) => {
                anyhow::bail!("DoQ DNS 查询超时 ({}s)", self.config.timeout_secs);
            }
        }
    }

    /// 处理 Final 规则
    /// 1. 使用 primary_upstream 查询
    /// 2. 检查返回的 IP 是否在指定的 ipcidr 列表中匹配到 CN
    /// 3. 如果是 CN，采用 primary 结果；否则使用 fallback_upstream 再查询
    /// 4. 将域名写入 output 文件
    async fn process_final_rule(
        &self, 
        domain: &str,
        request: &Message, 
        final_rule: &crate::config::FinalRule
    ) -> Result<(&UpstreamList, String, String, Message)> {
        use std::io::Write;
        use std::fs::OpenOptions;
        
        // 1. 使用 primary_upstream 查询
        let primary_upstream = self.config.upstreams.get(&final_rule.primary_upstream)
            .ok_or_else(|| anyhow::anyhow!("Final 规则的 primary_upstream '{}' 未找到", final_rule.primary_upstream))?;
        
        debug!("Final 规则: 使用 primary_upstream '{}' 查询域名 {}", final_rule.primary_upstream, domain);
        let primary_response = self.forward_to_upstream_list(request, primary_upstream).await?;
        
        // 2. 从响应中提取 IP 地址
        let ips = self.extract_ips_from_response(&primary_response);
        
        // 3. 检查 IP 是否在 CN 的 CIDR 列表中
        let is_cn = if let Some(ipcidr_list) = self.config.lists.get(&final_rule.ipcidr) {
            ips.iter().any(|ip| self.is_ip_in_cidr_list(ip, &ipcidr_list.domains))
        } else {
            false
        };
        
        // 4. 根据 IP 归属决定使用哪个结果
        let (final_upstream, final_response, upstream_name) = if is_cn {
            debug!("Final 规则: 域名 {} 的 IP 属于 CN，使用 primary 结果", domain);
            (primary_upstream, primary_response, final_rule.primary_upstream.clone())
        } else {
            debug!("Final 规则: 域名 {} 的 IP 不属于 CN，使用 fallback_upstream '{}'", domain, final_rule.fallback_upstream);
            let fallback_upstream = self.config.upstreams.get(&final_rule.fallback_upstream)
                .ok_or_else(|| anyhow::anyhow!("Final 规则的 fallback_upstream '{}' 未找到", final_rule.fallback_upstream))?;
            let fallback_response = self.forward_to_upstream_list(request, fallback_upstream).await?;
            (fallback_upstream, fallback_response, final_rule.fallback_upstream.clone())
        };
        
        // 5. 将域名写入 output 文件（如果配置了）
        if let Some(output_path) = &final_rule.output {
            if let Err(e) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(output_path)
                .and_then(|mut file| writeln!(file, "{}", domain.trim_end_matches('.')))
            {
                debug!("Final 规则: 写入 output 文件失败: {}", e);
            }
        }
        
        let rule_name = format!("final:{}", upstream_name);
        // Final 规则不记录匹配的域名，使用空字符串
        Ok((final_upstream, rule_name, String::new(), final_response))
    }

    /// 从 DNS 响应中提取 IP 地址
    fn extract_ips_from_response(&self, response: &Message) -> Vec<String> {
        let mut ips = Vec::new();
        
        // 从答案记录中提取 A 和 AAAA 记录的 IP
        for answer in response.answers() {
            if let Some(rdata) = answer.data() {
                let ip_str = format!("{}", rdata);
                // 简单提取 IP 地址（需要更精确的解析）
                if ip_str.contains('.') || ip_str.contains(':') {
                    ips.push(ip_str);
                }
            }
        }
        
        ips
    }

    /// 检查 IP 是否在 CIDR 列表中（简化实现）
    /// CIDR 列表格式：|CIDR|country_code|，例如：|39.156.0.0/16|CN|
    fn is_ip_in_cidr_list(&self, ip: &str, cidr_list: &[String]) -> bool {
        use std::net::IpAddr;
        
        // 解析 IP 地址
        let ip_addr: IpAddr = match ip.parse() {
            Ok(addr) => addr,
            Err(_) => return false,
        };
        
        // 遍历 CIDR 列表，检查是否匹配 CN
        for cidr_entry in cidr_list {
            // 格式：|CIDR|country_code|
            let parts: Vec<&str> = cidr_entry.split('|').collect();
            if parts.len() >= 3 {
                let cidr = parts[1].trim();
                let country = parts[2].trim().to_uppercase(); // 转换为大写进行比较
                
                // 只检查 CN 的 CIDR
                if country == "CN" {
                    if self.ip_in_cidr(&ip_addr, cidr) {
                        return true;
                    }
                }
            }
        }
        
        false
    }

    /// 检查 IP 是否在 CIDR 范围内（简化实现）
    fn ip_in_cidr(&self, ip: &std::net::IpAddr, cidr: &str) -> bool {
        use std::net::IpAddr;
        
        // 解析 CIDR
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return false;
        }
        
        let network: IpAddr = match parts[0].parse() {
            Ok(addr) => addr,
            Err(_) => return false,
        };
        
        let prefix_len: u8 = match parts[1].parse() {
            Ok(len) => len,
            Err(_) => return false,
        };
        
        // 简单实现：只支持 IPv4
        match (ip, network) {
            (IpAddr::V4(ip_v4), IpAddr::V4(net_v4)) => {
                let ip_bits = u32::from(*ip_v4);
                let net_bits = u32::from(net_v4);
                let mask = if prefix_len == 0 {
                    0
                } else {
                    !0u32 << (32 - prefix_len)
                };
                (ip_bits & mask) == (net_bits & mask)
            }
            _ => false,
        }
    }
    
    /// 记录域名命中到 .hit. 文件
    /// 规则：
    /// 1. 如果列表路径已包含 ".hit." 则不记录
    /// 2. servers 和 final 规则组不记录（由调用方保证）
    /// 3. 文件名格式：<list_name>.hit.txt
    fn record_hit(&self, domain: &str, list_name: &str, _matched_domain: &str) {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::path::Path;
        
        // 获取列表配置
        let list = match self.config.lists.get(list_name) {
            Some(l) => l,
            None => {
                debug!("记录命中失败: 列表 '{}' 不存在", list_name);
                return;
            }
        };
        
        // 检查路径是否已包含 ".hit."
        if let Some(ref path) = list.path {
            if path.contains(".hit.") {
                debug!("跳过记录命中: 列表 '{}' 路径已包含 .hit.", list_name);
                return;
            }
        }
        
        // 生成 hit 文件路径
        let hit_path = if let Some(ref path) = list.path {
            // 基于原文件路径生成 .hit.txt 文件
            let path_obj = Path::new(path);
            let parent = path_obj.parent().unwrap_or(Path::new("."));
            let stem = path_obj.file_stem().unwrap_or_default().to_string_lossy();
            parent.join(format!("{}.hit.txt", stem)).to_string_lossy().to_string()
        } else {
            // 如果没有 path，使用列表名称
            format!("./{}.hit.txt", list_name)
        };
        
        // 追加写入域名（每行一个）
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&hit_path)
        {
            Ok(mut file) => {
                if let Err(e) = writeln!(file, "{}", domain) {
                    warn!("写入命中文件 {} 失败: {}", hit_path, e);
                } else {
                    debug!("记录命中: {} -> {}", domain, hit_path);
                }
            }
            Err(e) => {
                warn!("打开命中文件 {} 失败: {}", hit_path, e);
            }
        }
    }
}
