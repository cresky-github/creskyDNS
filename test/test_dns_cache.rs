#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tokio::net::UdpSocket;
    use std::time::Instant;

    println!("DNS 缓存功能测试");
    println!("===============");

    let server = "127.0.0.1:15353";
    let test_domains = vec![
        "baidu.com",
        "qq.com",
        "google.com",
        "youtube.com",
    ];

    // 创建 DNS 查询包（简化版）
    fn create_dns_query(domain: &str) -> Vec<u8> {
        let mut query = vec![
            0x12, 0x34, // Transaction ID
            0x01, 0x00, // Flags: standard query
            0x00, 0x01, // Questions: 1
            0x00, 0x00, // Answer RRs: 0
            0x00, 0x00, // Authority RRs: 0
            0x00, 0x00, // Additional RRs: 0
        ];

        // Query name
        for part in domain.split('.') {
            query.push(part.len() as u8);
            query.extend_from_slice(part.as_bytes());
        }
        query.push(0); // End of name

        // Type A, Class IN
        query.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);

        query
    }

    println!("\n第一轮查询（未缓存）:");
    println!("-----------");
    let mut first_times = std::collections::HashMap::new();

    for domain in &test_domains {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(server).await?;

        let query = create_dns_query(domain);
        let start = Instant::now();
        socket.send(&query).await?;

        let mut buf = [0u8; 512];
        let len = socket.recv(&mut buf).await?;
        let elapsed = start.elapsed().as_millis();

        println!("✅ {} : {}ms ({}字节)", domain, elapsed, len);
        first_times.insert(*domain, elapsed);

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!("\n第二轮查询（应该命中缓存）:");
    println!("-----------");

    for domain in &test_domains {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(server).await?;

        let query = create_dns_query(domain);
        let start = Instant::now();
        socket.send(&query).await?;

        let mut buf = [0u8; 512];
        let len = socket.recv(&mut buf).await?;
        let elapsed = start.elapsed().as_millis();

        let first_time = first_times.get(domain).unwrap_or(&elapsed);
        let speedup = if elapsed > 0 {
            *first_time as f64 / elapsed as f64
        } else {
            0.0
        };

        println!("✅ {} : {}ms ({}字节) - 加速 {:.1}x",
                domain, elapsed, len, speedup);

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!("\n测试完成！如果第二次查询更快，说明缓存生效。");

    Ok(())
}
