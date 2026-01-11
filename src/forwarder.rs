use crate::config::{Config, UpstreamList};
use crate::cache::{DomainCache, RuleCache};
use anyhow::Result;
use hickory_proto::op::Message;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{UdpSocket, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::debug;
use base64;
use std::sync::Arc;
use rustls::ClientConfig;

/// DNS 协议类型
#[derive(Clone, Debug)]
pub enum Protocol {
    Udp,
    Tcp,
    Dot,   // DNS over TLS
    Doh,   // DNS over HTTPS
    Doq,   // DNS over QUIC
    H3,    // HTTP/3 (用于 DoH3)
}

/// DNS 转发器
pub struct DnsForwarder {
    config: Config,
    rule_cache: Option<RuleCache>,
    domain_cache: Option<DomainCache>,
}

impl DnsForwarder {
    /// 创建新的 DNS 转发器
    pub fn new(config: Config, rule_cache: Option<RuleCache>, domain_cache: Option<DomainCache>) -> Result<Self> {
        Ok(Self { config, rule_cache, domain_cache })
    }

    /// 解析上游服务器地址
    fn parse_address(addr: &str) -> Result<(String, u16)> {
        // 移除协议前缀
        let addr = addr
            .strip_prefix("udp://").or(Some(addr)).unwrap()
            .strip_prefix("tcp://").or(Some(addr)).unwrap()
            .strip_prefix("tls://").or(Some(addr)).unwrap()
            .strip_prefix("https://").or(Some(addr)).unwrap();
        
        if let Some((host, port_str)) = addr.rsplit_once(':') {
            let port = port_str.parse::<u16>()?;
            Ok((host.to_string(), port))
        } else {
            Ok((addr.to_string(), 53))
        }
    }

    /// 添加转发方法（带监听器名称）
    pub async fn forward_with_listener(
        &self,
        request: &Message,
        _listener_name: &str,
    ) -> Result<Message> {
        self.process_request(request).await
    }

    /// 处理UDP请求
    pub async fn handle_udp_request(
        &self,
        socket: &UdpSocket,
        addr: SocketAddr,
        data: &[u8],
    ) -> Result<()> {
        let request = crate::dns::parse_dns(data)?;
        let response = self.process_request(&request).await?;
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
        let response = self.process_request(&request).await?;
        let response_data = crate::dns::encode_dns(&response)?;
        
        // 发送TCP DNS消息（前2字节是长度）
        let len_bytes = (response_data.len() as u16).to_be_bytes();
        socket.write_all(&len_bytes).await?;
        socket.write_all(&response_data).await?;
        
        Ok(())
    }

    /// 处理DNS请求
    async fn process_request(&self, request: &Message) -> Result<Message> {
        let qname = crate::dns::get_qname(request)
            .ok_or_else(|| anyhow::anyhow!("无法获取查询名称"))?;
        
        // 1. 查询 Rule Cache（最高优先级）
        let upstream_name = if let Some(rule_cache) = &self.rule_cache {
            if let Some(cached_upstream) = rule_cache.get(&qname) {
                Some(cached_upstream)
            } else {
                None
            }
        } else {
            None
        };
        
        // 2. 查询 Domain Cache（第二优先级）
        if upstream_name.is_none() {
            if let Some(cache) = &self.domain_cache {
                if let Some(cached_response) = cache.get(&qname) {
                    debug!("Domain Cache 命中: {}", qname);
                    return Ok(cached_response);
                }
            }
        }
        
        // 3. 根据域名匹配规则选择上游（如果 Rule Cache 未命中）
        let (upstream_list, rule_name, upstream_list_name, response) = if let Some(cached_upstream) = upstream_name {
            // Rule Cache 命中，直接使用缓存的上游
            let upstream_list = self.config.upstreams.get(&cached_upstream)
                .ok_or_else(|| anyhow::anyhow!("上游列表 '{}' 未找到", cached_upstream))?;
            let response = self.forward_to_upstream_list(request, upstream_list).await?;
            (upstream_list, format!("cached:{}", cached_upstream), cached_upstream, response)
        } else {
            // Rule Cache 未命中，执行规则匹配
            let (upstream_list, rule_name, response) = self.match_domain(&qname, request).await?;
            let upstream_list_name = self.extract_upstream_name(&rule_name);
            (upstream_list, rule_name, upstream_list_name, response)
        };
        
        // 4. 写入 Rule Cache（原来的步骤5）
        if let Some(rule_cache) = &self.rule_cache {
            rule_cache.insert(qname.clone(), upstream_list_name.clone());
        }
        
        // 5. 写入 Domain Cache（原来的步骤6）
        if let Some(cache) = &self.domain_cache {
            // 从响应中提取最小 TTL
            let ttl = self.extract_min_ttl(&response);
            cache.insert(qname.clone(), rule_name, response.clone(), ttl);
        }
        
        Ok(response)
    }

    /// 根据域名匹配规则（返回 upstream 和规则名称）
    async fn match_domain(&self, domain: &str, request: &Message) -> Result<(&UpstreamList, String, Message)> {
        // 首先尝试服务器规则匹配
        if let Some((server_upstream, rule_name)) = self.match_server_rule(domain)? {
            let response = self.forward_to_upstream_list(request, server_upstream).await?;
            return Ok((server_upstream, rule_name, response));
        }

        // 如果没有服务器规则，则按域名规则匹配
        match self.match_domain_rules(domain) {
            Ok((upstream, rule_name)) => {
                let response = self.forward_to_upstream_list(request, upstream).await?;
                Ok((upstream, rule_name, response))
            }
            Err(e) if e.to_string() == "NO_MATCH" => {
                // 未匹配任何规则，尝试 Final 规则或默认上游
                self.handle_no_match(domain, request).await
            }
            Err(e) => Err(e),
        }
    }

    /// 处理未匹配任何规则的情况
    async fn handle_no_match(&self, domain: &str, request: &Message) -> Result<(&UpstreamList, String, Message)> {
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
                return Ok((upstream, rule_name, response));
            }
        }
        
        // 如果连默认上游都没有，使用第一个可用的上游
        if let Some((name, upstream)) = self.config.upstreams.iter().next() {
            debug!("域名 {} 未匹配任何规则，使用第一个可用上游 '{}'", domain, name);
            let rule_name = format!("fallback:{}", name);
            let response = self.forward_to_upstream_list(request, upstream).await?;
            return Ok((upstream, rule_name, response));
        }

        // 如果没有任何上游，返回错误
        anyhow::bail!("域名 {} 未匹配到任何规则，且没有可用的默认上游", domain)
    }

    /// 匹配服务器规则（按实例）
    fn match_server_rule(&self, _domain: &str) -> Result<Option<(&UpstreamList, String)>> {
        // 这里可以根据服务器实例进行匹配
        // 暂时返回None，交给域名规则处理
        Ok(None)
    }

    /// 匹配域名规则（按规则组顺序）
    /// 规则说明：
    /// 1. 按 YAML 配置文件中定义的顺序遍历规则组
    /// 2. 在每个规则组内，同时匹配所有list
    /// 3. 按domain suffix方式匹配
    /// 4. 取域名深度最大的规则
    /// 5. 如果深度相同，取group内最后一个匹配的规则
    /// 6. 一旦某个规则组有匹配，立即返回，不再检查后续规则组
    fn match_domain_rules(&self, domain: &str) -> Result<(&UpstreamList, String)> {
        // 按 YAML 顺序遍历所有规则组（IndexMap 保证顺序）
        for (group_name, rules) in &self.config.rules {
            // 跳过 'final' 规则组，它在最后单独处理
            if group_name == "final" {
                continue;
            }
            
            // 在每个规则组内找到最优匹配
            if let Some(upstream_list) = self.find_best_match_in_group(domain, rules) {
                debug!("域名 {} 在规则组 '{}' 中匹配到上游 '{}'", domain, group_name, upstream_list);
                let upstream = self.config.upstreams.get(&upstream_list)
                    .ok_or_else(|| anyhow::anyhow!("上游列表 '{}' 未找到", upstream_list))?;
                let rule_name = format!("{}:{}", group_name, upstream_list);
                return Ok((upstream, rule_name));
            }
        }

        // 未匹配任何规则，返回 None 表示需要走 Final 规则或默认上游
        anyhow::bail!("NO_MATCH")
    }

    /// 在单个group内找到最优匹配
    /// 同时评估所有规则，按深度降序、rule_index降序排序，取第一个匹配
    fn find_best_match_in_group(
        &self,
        domain: &str,
        rules: &[String],
    ) -> Option<String> {
        let mut matches: Vec<(usize, usize, String)> = Vec::new(); // (depth, rule_index, upstream_list)

        // 同时评估所有规则
        for (rule_index, rule_str) in rules.iter().enumerate() {
            if let Some((domain_list, upstream_list)) = self.parse_rule_string(rule_str) {
                if let Some(depth) = self.get_match_depth(domain, &domain_list) {
                    matches.push((depth, rule_index, upstream_list));
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

        // 返回最优匹配的上游列表名
        matches.first().map(|(_, _, upstream_list)| upstream_list.clone())
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
    fn get_match_depth(&self, domain: &str, domain_list_name: &str) -> Option<usize> {
        let domain_list = self.config.lists.get(domain_list_name)?;
        let domain_parts: Vec<&str> = domain.split('.').filter(|s| !s.is_empty()).collect();

        // 检查各级域名是否匹配
        for depth in (0..=domain_parts.len()).rev() {
            let check_domain = if depth == 0 {
                ".".to_string()
            } else {
                domain_parts[domain_parts.len() - depth..].join(".")
            };

            if domain_list.domains.contains(&check_domain) {
                return Some(depth);
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
        let upstream_addr = upstream_list.addresses.first()
            .ok_or_else(|| anyhow::anyhow!("上游列表为空"))?;
        
        let protocol = Self::parse_protocol(upstream_addr)?;
        
        match protocol {
            Protocol::Udp => self.forward_udp(request, upstream_addr).await,
            Protocol::Tcp => self.forward_tcp(request, upstream_addr).await,
            Protocol::Dot => self.forward_dot(request, upstream_addr).await,
            Protocol::Doh => self.forward_doh(request, upstream_addr).await,
            Protocol::Doq => self.forward_doq(request, upstream_addr).await,
            Protocol::H3 => self.forward_h3(request, upstream_addr).await,
        }
    }

    /// 解析协议类型
    fn parse_protocol(addr: &str) -> Result<Protocol> {
        if addr.starts_with("h3://") {
            Ok(Protocol::H3)
        } else if addr.starts_with("https3://") {
            Ok(Protocol::H3)
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

    /// UDP 转发
    async fn forward_udp(&self, request: &Message, upstream_addr: &str) -> Result<Message> {
        let (host, port) = Self::parse_address(upstream_addr)?;
        let upstream_socket_addr: SocketAddr = format!("{}:{}", host, port).parse()?;

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

        let (host, port) = Self::parse_address(upstream_addr)?;
        let upstream_socket_addr: SocketAddr = format!("{}:{}", host, port).parse()?;

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
    async fn forward_dot(&self, request: &Message, upstream_addr: &str) -> Result<Message> {
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

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let socket_addr = format!("{}:{}", host, port).parse::<SocketAddr>()?;

        // 创建 TLS 连接
        let stream = TcpStream::connect(socket_addr).await?;
        let root_store = Self::load_root_certs();
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        );

        let server_name = host.as_str().try_into()
            .map_err(|_| anyhow::anyhow!("无效的服务器名称"))?;
        
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

    /// DoH (DNS over HTTPS) 转发
    async fn forward_doh(&self, request: &Message, upstream_addr: &str) -> Result<Message> {
        let url = upstream_addr.to_string();
        let timeout = Duration::from_secs(self.config.timeout_secs);

        // 将 DNS 消息编码为 base64
        let request_data = request.to_vec()?;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        let dns_query = URL_SAFE_NO_PAD.encode(&request_data);

        // 构建 DoH 请求
        let client = reqwest::Client::builder()
            .timeout(timeout)
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

    /// DoQ (DNS over QUIC) 转发
    async fn forward_doq(&self, request: &Message, upstream_addr: &str) -> Result<Message> {
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

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let socket_addr = format!("{}:{}", host, port).parse::<SocketAddr>()?;

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

        // 连接到 QUIC 服务器
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
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| anyhow::anyhow!("打开 QUIC 双向流失败: {}", e))?;

        let mut response_data: Vec<u8> = Vec::new();
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

    /// H3/DoH3 (DNS over HTTP/3) 转发
    async fn forward_h3(&self, request: &Message, upstream_addr: &str) -> Result<Message> {
        // 提取 URL
        let addr_part = upstream_addr.strip_prefix("h3://")
            .or_else(|| upstream_addr.strip_prefix("https3://"))
            .unwrap_or(upstream_addr)
            .to_string();

        let url = if addr_part.contains("://") {
            format!("https://{}", addr_part)
        } else {
            format!("https://{}/dns-query", addr_part)
        };

        let timeout = Duration::from_secs(self.config.timeout_secs);

        // 将 DNS 消息编码为 base64
        let request_data = request.to_vec()?;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        let dns_query = URL_SAFE_NO_PAD.encode(&request_data);

        // 构建 H3 请求 (使用 reqwest 的 http3 特性如果可用，否则降级到 HTTPS)
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()?;

        let response = tokio::time::timeout(
            timeout,
            client
                .get(&url)
                .query(&[("dns", &dns_query)])
                .header("Accept", "application/dns-message")
                .header("User-Agent", "creskyDNS/h3")
                .send()
        ).await??;

        if !response.status().is_success() {
            anyhow::bail!("H3 请求失败: HTTP {}", response.status());
        }

        let response_data = response.bytes().await?;
        let message = Message::from_vec(&response_data)?;
        
        debug!("H3 收到来自 {} 的响应", upstream_addr);
        Ok(message)
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
    ) -> Result<(&UpstreamList, String, Message)> {
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
        Ok((final_upstream, rule_name, final_response))
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
                let country = parts[2].trim();
                
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
}
