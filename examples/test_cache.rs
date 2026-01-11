use std::time::Instant;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("DNS 两级缓存功能测试");
    println!("====================");
    println!("测试 Rule Cache + Domain Cache 两级缓存机制\n");

    let server = "127.0.0.1:15353";
    let test_domains = vec![
        ("baidu.com", "direct → google DNS"),
        ("qq.com", "direct → google DNS"),
        ("google.com", "proxy → cloudflare DNS"),
        ("youtube.com", "proxy → cloudflare DNS"),
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

    println!("第一轮查询（未缓存）:");
    println!("-------------------");
    let mut first_times = std::collections::HashMap::new();

    for (domain, route) in &test_domains {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(server).await?;

        let query = create_dns_query(domain);
        let start = Instant::now();
        socket.send(&query).await?;

        let mut buf = [0u8; 512];
        let len = socket.recv(&mut buf).await?;
        let elapsed = start.elapsed().as_millis();

        println!("✅ {:15} : {:4}ms ({:3}字节) [{}]", domain, elapsed, len, route);
        first_times.insert(*domain, elapsed);

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    println!("\n第二轮查询（应该命中缓存）:");
    println!("-------------------------");

    for (domain, route) in &test_domains {
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

        let indicator = if speedup > 1.5 { "🚀" } else { "⚡" };
        println!("{} {:15} : {:4}ms ({:3}字节) - 加速 {:.1}x [{}]",
                indicator, domain, elapsed, len, speedup, route);

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    println!("\n====================");
    println!("测试完成！");
    println!("\n两级缓存说明:");
    println!("1️⃣  Rule Cache: 域名 → upstream 映射（最快）");
    println!("2️⃣  Domain Cache: 完整 DNS 响应缓存（次快）");
    println!("\n如果第二次查询明显更快，说明两级缓存都生效。");
    println!("查看服务器日志可以看到:");
    println!("  - \"Rule Cache 命中\" 或 \"Rule Cache 未命中\"");
    println!("  - \"Domain Cache 命中\" 或 \"Rule Cache 写入\"");

    Ok(())
}
