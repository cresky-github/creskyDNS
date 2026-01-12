import socket
import struct
import time

def create_dns_query(domain):
    """创建 DNS 查询包"""
    # Transaction ID
    transaction_id = struct.pack('>H', 0x1234)
    
    # Flags: standard query
    flags = struct.pack('>H', 0x0100)
    
    # Questions, Answer RRs, Authority RRs, Additional RRs
    counts = struct.pack('>HHHH', 1, 0, 0, 0)
    
    # Query name
    query_name = b''
    for part in domain.split('.'):
        query_name += struct.pack('B', len(part)) + part.encode()
    query_name += b'\x00'  # End of name
    
    # Query type (A) and class (IN)
    query_type_class = struct.pack('>HH', 1, 1)
    
    return transaction_id + flags + counts + query_name + query_type_class

def send_dns_query(domain, server='127.0.0.1', port=15353):
    """发送 DNS 查询"""
    query = create_dns_query(domain)
    
    # 创建 UDP 套接字
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(5)
    
    try:
        # 发送查询
        start_time = time.time()
        sock.sendto(query, (server, port))
        
        # 接收响应
        response, _ = sock.recvfrom(512)
        elapsed = (time.time() - start_time) * 1000
        
        print(f"✅ 查询 {domain} 成功 (耗时: {elapsed:.2f}ms)")
        print(f"   响应大小: {len(response)} 字节")
        return True, elapsed
    except socket.timeout:
        print(f"❌ 查询 {domain} 超时")
        return False, 0
    except Exception as e:
        print(f"❌ 查询 {domain} 错误: {e}")
        return False, 0
    finally:
        sock.close()

def test_cache():
    """测试 DNS 缓存功能"""
    print("=" * 60)
    print("DNS 缓存功能测试")
    print("=" * 60)
    
    test_domains = [
        ("baidu.com", "google DNS"),
        ("qq.com", "google DNS"),
        ("google.com", "cloudflare DNS"),
        ("youtube.com", "cloudflare DNS"),
    ]
    
    print("\n第一轮查询（未缓存）:")
    print("-" * 60)
    first_times = {}
    for domain, upstream in test_domains:
        success, elapsed = send_dns_query(domain)
        first_times[domain] = elapsed
        time.sleep(0.5)
    
    print("\n第二轮查询（应该命中缓存）:")
    print("-" * 60)
    for domain, upstream in test_domains:
        success, elapsed = send_dns_query(domain)
        if domain in first_times and first_times[domain] > 0:
            speedup = first_times[domain] / elapsed if elapsed > 0 else 0
            print(f"   加速比: {speedup:.2f}x (第一次: {first_times[domain]:.2f}ms -> 第二次: {elapsed:.2f}ms)")
        time.sleep(0.5)
    
    print("\n" + "=" * 60)
    print("测试完成！")
    print("如果第二次查询明显更快，说明缓存工作正常。")
    print("=" * 60)

if __name__ == "__main__":
    test_cache()
