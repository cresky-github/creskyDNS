use crate::config::{Config, UpstreamList};
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
}

impl DnsForwarder {
    /// 创建新的 DNS 转发器
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self { config })
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
        
        // 根据域名匹配规则选择上游
        let upstream_list = self.match_domain(&qname)?;
        self.forward_to_upstream_list(request, upstream_list).await
    }

    /// 根据域名匹配规则
    fn match_domain(&self, domain: &str) -> Result<&UpstreamList> {
        // 首先尝试服务器规则匹配
        if let Some(server_upstream) = self.match_server_rule(domain)? {
            return Ok(server_upstream);
        }

        // 如果没有服务器规则，则按域名规则匹配
        self.match_domain_rules(domain)
    }

    /// 匹配服务器规则（按实例）
    fn match_server_rule(&self, domain: &str) -> Result<Option<&UpstreamList>> {
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
    fn match_domain_rules(&self, domain: &str) -> Result<&UpstreamList> {
        // 按 YAML 顺序遍历所有规则组（IndexMap 保证顺序）
        for (group_name, rules) in &self.config.rules {
            // 在每个规则组内找到最优匹配
            if let Some(upstream_list) = self.find_best_match_in_group(domain, rules) {
                debug!("域名 {} 在规则组 '{}' 中匹配到上游 '{}'", domain, group_name, upstream_list);
                return self.config.upstreams.get(&upstream_list)
                    .ok_or_else(|| anyhow::anyhow!("上游列表 '{}' 未找到", upstream_list));
            }
        }

        // 如果没有匹配，返回错误
        anyhow::bail!("域名 {} 未匹配到任何规则", domain)
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
}
