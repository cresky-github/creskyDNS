use anyhow::Result;
use hickory_proto::op::Message;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};
use std::collections::HashMap;

mod config;
mod forwarder;
mod dns;
mod cache;
mod log;

use config::{Config, DomainListReloadState};
use forwarder::DnsForwarder;
use cache::{CacheManager};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载配置
    let mut config = load_config()?;
    
    // 初始化日志系统
    log::init_logging(&config.log)?;
    
    info!("DNS 转发器启动");
    info!("请求超时: {}s", config.timeout_secs);
    
    // 验证监听器端口配置
    config.validate_listener_ports()?;
    
    // 加载域名列表文件
    for (name, list) in &mut config.lists {
        let path_copy = list.path.clone();
        if let Some(path) = path_copy {
            match list.load().await {
                Ok(_) => {
                    let item_type = if list.r#type == "ipcidr" { "条记录" } else { "个域名" };
                    info!("域名列表 '{}' 从文件 '{}' 加载成功: {} {}",
                          name, &path, list.domains.len(), item_type);
                }
                Err(e) => {
                    error!("域名列表 '{}' 从文件 '{}' 加载失败: {}",
                           name, &path, e);
                    // 不中断启动，继续使用配置中的域名列表
                }
            }
        }
    }
    
    // 显示所有缓存配置
    for (id, cache_config) in &config.cache {
        info!("缓存 '{}' 配置: 大小={}, min_ttl={:?}, max_ttl={:?}",
              id, cache_config.size, cache_config.min_ttl, cache_config.max_ttl);
    }

    // 显示所有域名列表
    for (name, list) in &config.lists {
        let item_desc = if list.r#type == "ipcidr" {
            format!("{} 条记录", list.domains.len())
        } else {
            format!("{} 个域名", list.domains.len())
        };
        info!("域名列表 '{}' (类型: {}, 格式: {}): {}",
              name, list.r#type, list.format, item_desc);
    }

    // 显示所有上游列表
    for (name, upstream_list) in &config.upstreams {
        info!("上游列表 '{}' : {:?}", name, upstream_list.addr);
    }

    // 显示所有规则
    for (rule_name, rules) in &config.rules {
        info!("规则组 '{}': {} 条规则", rule_name, rules.len());
        for rule_str in rules {
            info!("  - {}", rule_str);
        }
    }

    // 显示所有监听器
    for (name, port) in &config.listener {
        info!("监听器 '{}' 端口: {}", name, port);
    }

    // 创建共享的域名列表管理器（用于热重新加载）
    let domain_lists: Arc<RwLock<HashMap<String, Vec<String>>>> = Arc::new(RwLock::new(
        config.lists.iter()
            .map(|(name, list)| (name.clone(), list.domains.clone()))
            .collect()
    ));

    // 创建并初始化重新加载状态
    let reload_states: Arc<Mutex<HashMap<String, DomainListReloadState>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    {
        let mut states = reload_states.lock().unwrap();
        for (name, list) in &config.lists {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            
            let modified_time = list.get_file_modified_time().unwrap_or(0);
            states.insert(name.clone(), DomainListReloadState {
                last_modified: modified_time,
                last_loaded: now,
                pending_update: false,
            });
        }
    }

    // 获取默认上游服务器（YAML 顺序最后一个）
    let default_upstream = config.upstreams.keys().last()
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    
    // 初始化缓存管理器
    let cache_manager = Arc::new(CacheManager::new(&config.cache, default_upstream)?);
    info!("缓存管理器已初始化");
    
    // 执行冷启动流程（如果启用）
    let warm_up_list = cache_manager.cold_start(&config).await?;
    
    // 创建转发器（在冷启动之后）
    let forwarder = Arc::new(DnsForwarder::new(
        config.clone(),
        cache_manager.get_rule_cache(),
        cache_manager.get_domain_cache("domain"), // 使用 "domain" 缓存作为默认
    )?);
    
    // 执行预热查询（如果有需要预热的域名）
    if !warm_up_list.is_empty() {
        info!("开始预热查询: {} 个域名", warm_up_list.len());
        
        // 读取冷启动配置
        let cold_start_config = config.cache.get("rule")
            .and_then(|c| c.cold_start.as_ref())
            .cloned()
            .unwrap_or_default();
        
        if cold_start_config.enabled {
            let forwarder_clone = Arc::clone(&forwarder);
            tokio::spawn(async move {
                warm_up_queries(forwarder_clone, warm_up_list, &cold_start_config).await;
            });
        } else {
            info!("冷启动预热已禁用，跳过预热查询");
        }
    } else {
        info!("无需预热的域名");
    }
    // 启动缓存清理和导出任务（根据配置的 interval）
    for (cache_name, cache_config) in &config.cache {
        let interval_secs = Config::parse_interval(&cache_config.interval)
            .unwrap_or(300); // 默认 5 分钟
        info!("缓存 '{}' 导出间隔: {} 秒", cache_name, interval_secs);
        
        let cache_manager_clone = Arc::clone(&cache_manager);
        let cache_name_clone = cache_name.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(interval_secs)).await;
                
                // 清理过期缓存
                cache_manager_clone.cleanup_all_expired();
                
                // 导出缓存
                if let Err(e) = cache_manager_clone.export_all() {
                    error!("导出缓存 '{}' 失败: {}", cache_name_clone, e);
                } else {
                    debug!("缓存 '{}' 已导出", cache_name_clone);
                }
            }
        });
    }

    // 启动域名列表重新加载监视任务
    let reload_config = config.clone();
    let reload_lists = Arc::clone(&domain_lists);
    let reload_states_clone = Arc::clone(&reload_states);
    let reload_handle = tokio::spawn(async move {
        monitor_domain_list_reload(reload_config, reload_lists, reload_states_clone).await;
    });

    // 为每个监听器启动处理任务
    let mut handles = vec![];

    for (name, port) in config.listener {
        let forwarder = Arc::clone(&forwarder);
        let handle = tokio::spawn(async move {
            if let Err(e) = run_listener(name, port, forwarder).await {
                error!("监听器错误: {}", e);
            }
        });
        handles.push(handle);
    }

    // 等待所有服务器
    handles.push(reload_handle);  // 添加重新加载任务到列表
    
    for handle in handles {
        if let Err(e) = handle.await {
            error!("任务失败: {}", e);
        }
    }

    Ok(())
}

/// 监视域名列表文件变化并重新加载
async fn monitor_domain_list_reload(
    config: Config,
    domain_lists: Arc<RwLock<HashMap<String, Vec<String>>>>,
    reload_states: Arc<Mutex<HashMap<String, DomainListReloadState>>>,
) {
    // 每 5 秒检查一次是否需要重新加载
    let check_interval = Duration::from_secs(5);
    
    loop {
        sleep(check_interval).await;
        
        let mut states = reload_states.lock().unwrap();
        let mut lists_updated = false;
        
        for (name, list) in &config.lists {
            if list.path.is_none() {
                continue;  // 跳过没有文件路径的列表
            }
            
            let state = states.entry(name.clone()).or_insert_with(|| {
                DomainListReloadState {
                    last_modified: 0,
                    last_loaded: 0,
                    pending_update: false,
                }
            });
            
            // 检查是否需要重新加载
            if list.should_reload(state) {
                // 创建一个可变的副本来重新加载
                let mut list_copy = list.clone();
                
                match list_copy.load_sync() {
                    Ok(_) => {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        
                        let modified = list_copy.get_file_modified_time().unwrap_or(0);
                        
                        // 更新共享的域名列表
                        {
                            let mut lists = domain_lists.write().unwrap();
                            lists.insert(name.clone(), list_copy.domains.clone());
                        }
                        
                        state.last_modified = modified;
                        state.last_loaded = now;
                        state.pending_update = false;
                        lists_updated = true;
                        
                        info!("域名列表 '{}' 已重新加载: {} 个域名", 
                              name, list_copy.domains.len());
                    }
                    Err(e) => {
                        error!("域名列表 '{}' 重新加载失败: {}", name, e);
                    }
                }
            }
        }
        drop(states);
        
        if lists_updated {
            info!("域名列表已更新，重新加载完成");
        }
    }
}

/// 运行单个监听器
async fn run_listener(
    name: String,
    port: u16,
    forwarder: Arc<DnsForwarder>,
) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    info!("监听器 '{}' 启动在 {} (UDP/TCP)", name, addr);

    // 启动 UDP 监听器
    let udp_forwarder = Arc::clone(&forwarder);
    let udp_name = format!("{}-udp", &name);
    let name_for_udp = name.clone();
    let udp_handle = tokio::spawn(async move {
        if let Err(e) = run_udp_listener(udp_name, port, udp_forwarder, name_for_udp).await {
            error!("UDP 监听器错误: {}", e);
        }
    });

    // 启动 TCP 监听器
    let tcp_forwarder = Arc::clone(&forwarder);
    let tcp_name = format!("{}-tcp", &name);
    let tcp_handle = tokio::spawn(async move {
        if let Err(e) = run_tcp_listener(tcp_name, port, tcp_forwarder, name).await {
            error!("TCP 监听器错误: {}", e);
        }
    });

    // 等待两个监听器
    let (udp_result, tcp_result) = tokio::join!(udp_handle, tcp_handle);
    
    if let Err(e) = udp_result {
        error!("UDP 监听器任务失败: {}", e);
    }
    if let Err(e) = tcp_result {
        error!("TCP 监听器任务失败: {}", e);
    }

    Ok(())
}

/// 运行 UDP 监听器
async fn run_udp_listener(
    name: String,
    port: u16,
    forwarder: Arc<DnsForwarder>,
    listener_name: String,
) -> Result<()> {
    let listen_addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    let socket = TokioUdpSocket::bind(listen_addr).await?;
    let socket = Arc::new(socket);

    debug!("UDP 监听器 '{}' 绑定到 {}", name, listen_addr);

    let log_name = name.clone();
    // 处理 DNS 查询
    loop {
        let mut buf = [0u8; 512];
        match socket.recv_from(&mut buf).await {
            Ok((len, peer_addr)) => {
                let socket = Arc::clone(&socket);
                let forwarder = Arc::clone(&forwarder);
                let data = buf[..len].to_vec();
                let listener_name = listener_name.clone();
                let log_name = log_name.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_udp_query(socket, forwarder, peer_addr, data, listener_name).await {
                        error!("UDP 监听器 '{}' 处理查询失败 [{}]: {}", log_name, peer_addr, e);
                    }
                });
            }
            Err(e) => {
                error!("UDP 监听器 '{}' 接收数据失败: {}", name, e);
            }
        }
    }
}

/// 运行 TCP 监听器
async fn run_tcp_listener(
    name: String,
    port: u16,
    forwarder: Arc<DnsForwarder>,
    listener_name: String,
) -> Result<()> {
    use tokio::net::TcpListener;

    let listen_addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&listen_addr).await?;

    debug!("TCP 监听器 '{}' 绑定到 {}", name, listen_addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                let forwarder = Arc::clone(&forwarder);
                let listener_name = listener_name.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_connection(socket, forwarder, peer_addr, listener_name).await {
                        error!("TCP 连接处理失败 [{}]: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                error!("TCP 监听器 '{}' 接受连接失败: {}", name, e);
            }
        }
    }
}

/// 处理 TCP 连接
async fn handle_tcp_connection(
    mut socket: tokio::net::TcpStream,
    forwarder: Arc<DnsForwarder>,
    peer_addr: SocketAddr,
    listener_name: String,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // 读取长度前缀 (2 字节)
    let mut len_buf = [0u8; 2];
    socket.read_exact(&mut len_buf).await?;
    let msg_len = u16::from_be_bytes(len_buf) as usize;

    if msg_len == 0 || msg_len > 4096 {
        anyhow::bail!("无效的消息长度: {}", msg_len);
    }

    // 读取 DNS 消息
    let mut buf = vec![0u8; msg_len];
    socket.read_exact(&mut buf).await?;

    // 解析 DNS 请求
    let request = Message::from_vec(&buf)?;
    
    debug!("收到来自 {} 的 TCP DNS 查询 (监听器: {})", peer_addr, listener_name);
    for query in request.queries() {
        debug!("  查询: {} ({})", query.name(), query.query_type());
    }

    // 转发查询
    let response = forwarder.forward_with_listener(&request, &listener_name).await?;
    let response_data = response.to_vec()?;

    // 发送长度前缀
    let response_len = (response_data.len() as u16).to_be_bytes();
    socket.write_all(&response_len).await?;
    
    // 发送响应数据
    socket.write_all(&response_data).await?;

    debug!("TCP 响应已发送至 {}", peer_addr);
    Ok(())
}

async fn handle_udp_query(
    socket: Arc<TokioUdpSocket>,
    forwarder: Arc<DnsForwarder>,
    peer_addr: SocketAddr,
    data: Vec<u8>,
    listener_name: String,
) -> Result<()> {
    // 解析 DNS 请求
    let request = Message::from_vec(&data)?;
    
    debug!("收到来自 {} 的 UDP DNS 查询 (监听器: {})", peer_addr, listener_name);
    for query in request.queries() {
        debug!("  查询: {} ({})", query.name(), query.query_type());
    }

    // 转发查询
    let response = forwarder.forward_with_listener(&request, &listener_name).await?;

    // 返回响应
    let response_data = response.to_vec()?;
    socket.send_to(&response_data, peer_addr).await?;

    debug!("UDP 响应已发送至 {}", peer_addr);
    Ok(())
}

/// 解析命令行参数
fn parse_args() -> (Option<String>, Option<String>) {
    let args: Vec<String> = env::args().collect();
    let mut config_path: Option<String> = None;
    let mut work_dir: Option<String> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("错误: -c/--config 参数需要指定配置文件路径");
                    std::process::exit(1);
                }
            }
            "-w" | "--work-dir" => {
                if i + 1 < args.len() {
                    work_dir = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("错误: -w/--work-dir 参数需要指定工作目录");
                    std::process::exit(1);
                }
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-v" | "--version" => {
                println!("CreskyDNS v{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {
                // 兼容旧的用法：直接指定配置文件路径（无参数）
                if config_path.is_none() && !args[i].starts_with('-') {
                    config_path = Some(args[i].clone());
                    i += 1;
                } else {
                    eprintln!("错误: 未知参数 '{}'", args[i]);
                    eprintln!("使用 -h 或 --help 查看帮助信息");
                    std::process::exit(1);
                }
            }
        }
    }
    
    (config_path, work_dir)
}

/// 打印帮助信息
fn print_help() {
    println!("CreskyDNS - 智能 DNS 转发器");
    println!();
    println!("用法:");
    println!("  creskyDNS [选项]");
    println!("  creskyDNS [配置文件路径]  # 兼容旧版用法");
    println!();
    println!("选项:");
    println!("  -c, --config <文件>    指定配置文件路径");
    println!("  -w, --work-dir <目录>  指定工作目录（相对路径将基于此目录）");
    println!("  -h, --help            显示此帮助信息");
    println!("  -v, --version         显示版本信息");
    println!();
    println!("示例:");
    println!("  creskyDNS -c /etc/creskydns/config.yaml");
    println!("  creskyDNS -w /opt/creskydns -c config.yaml");
    println!("  creskyDNS config.yaml  # 简写方式");
    println!();
    println!("配置文件查找顺序:");
    println!("  1. 命令行参数指定的配置文件 (-c)");
    println!("  2. 环境变量 DNS_FORWARDER_CONFIG");
    println!("  3. 当前目录下的 config.yaml, config.yml, config.json");
    println!("  4. ./etc/creskyDNS.yaml");
    println!("  5. 使用内置默认配置");
}

/// 预热查询：对冷启动加载的域名进行实际 DNS 查询
async fn warm_up_queries(
    forwarder: Arc<DnsForwarder>,
    warm_up_list: Vec<(String, String, String, String)>,
    cold_start_config: &config::ColdStartConfig,
) {
    use hickory_proto::op::{Message, Query, OpCode};
    use hickory_proto::rr::{Name, RecordType};
    use std::str::FromStr;
    use futures::stream::{self, StreamExt};
    
    let total = warm_up_list.len();
    let parallel = cold_start_config.parallel;
    let timeout_ms = cold_start_config.timeout;
    
    info!("预热查询: {} 个域名，并发数: {}, 超时: {}ms", total, parallel, timeout_ms);
    
    let mut success = 0;
    let mut failed = 0;
    
    // 使用并发流处理
    let results = stream::iter(warm_up_list)
        .map(|(qname, _match_domain, _upstream, _cache_id)| {
            let forwarder = Arc::clone(&forwarder);
            let qname = qname.clone();
            async move {
                // 构造 DNS 查询
                let domain_name = match Name::from_str(&format!("{}.", &qname)) {
                    Ok(name) => name,
                    Err(e) => {
                        error!("预热查询: 域名格式错误 '{}': {}", qname, e);
                        return Err(());
                    }
                };
                
                let mut request = Message::new();
                request.set_id(rand::random());
                request.set_op_code(OpCode::Query);
                request.set_recursion_desired(true);
                request.add_query(Query::query(domain_name, RecordType::A));
                
                // 执行查询（带超时）
                let query_result = tokio::time::timeout(
                    Duration::from_millis(timeout_ms),
                    forwarder.forward_with_listener(&request, "rule")
                ).await;
                
                match query_result {
                    Ok(Ok(_response)) => {
                        debug!("预热查询成功: {}", qname);
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        debug!("预热查询失败: {} - {}", qname, e);
                        Err(())
                    }
                    Err(_) => {
                        debug!("预热查询超时: {}", qname);
                        Err(())
                    }
                }
            }
        })
        .buffer_unordered(parallel)
        .collect::<Vec<_>>()
        .await;
    
    // 统计结果
    for result in results {
        match result {
            Ok(_) => success += 1,
            Err(_) => failed += 1,
        }
    }
    
    info!("预热查询完成: 成功 {}/{}, 失败 {}", success, total, failed);
}

/// 加载配置文件
fn load_config() -> Result<Config> {
    // 解析命令行参数
    let (config_arg, work_dir) = parse_args();
    
    // 如果指定了工作目录，切换到该目录
    if let Some(dir) = work_dir {
        info!("切换工作目录到: {}", dir);
        if let Err(e) = env::set_current_dir(&dir) {
            return Err(anyhow::anyhow!("无法切换到工作目录 '{}': {}", dir, e));
        }
    }
    
    // 优先级：命令行参数 > 环境变量 > 默认路径 > 默认配置
    
    // 1. 检查命令行参数
    if let Some(config_path) = config_arg {
        info!("从命令行参数加载配置: {}", config_path);
        return Config::from_file(&config_path);
    }

    // 2. 检查环境变量
    if let Ok(config_path) = env::var("DNS_FORWARDER_CONFIG") {
        info!("从环境变量加载配置: {}", config_path);
        return Config::from_file(&config_path);
    }

    // 3. 检查默认位置
    let default_paths = vec![
        "config.yaml",
        "config.yml",
        "config.json",
        "./etc/creskyDNS.yaml",
    ];

    for path in default_paths {
        if std::path::Path::new(path).exists() {
            info!("从默认位置加载配置: {}", path);
            return Config::from_file(path);
        }
    }

    // 4. 使用默认配置
    info!("使用默认配置");
    Ok(Config::default())
}
