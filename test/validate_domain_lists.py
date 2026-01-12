#!/usr/bin/env python3
"""
域名列表文件格式验证脚本
检查域名列表文件是否符合规范格式

用法: python validate_domain_lists.py [files...]
例如: python validate_domain_lists.py direct_domains.txt proxy_domains.txt
"""

import sys
import os
import re
from pathlib import Path


class DomainListValidator:
    """域名列表文件验证器"""
    
    # 有效的域名模式（简化版）
    VALID_DOMAIN_PATTERN = re.compile(
        r'^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)*[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$',
        re.IGNORECASE
    )
    
    # 禁止的前缀/后缀
    INVALID_PREFIXES = ['*.', '||', '^', '@@||']
    INVALID_SUFFIXES = ['$', '|', '/*']
    
    def __init__(self):
        self.errors = []
        self.warnings = []
        self.stats = {
            'total_lines': 0,
            'valid_domains': 0,
            'comments': 0,
            'empty_lines': 0,
            'invalid_lines': 0
        }
    
    def validate_file(self, filepath):
        """验证单个文件"""
        print(f"\n检查文件: {filepath}")
        print("=" * 60)
        
        if not os.path.exists(filepath):
            print(f"❌ 错误: 文件不存在: {filepath}")
            return False
        
        self.errors = []
        self.warnings = []
        self.stats = {
            'total_lines': 0,
            'valid_domains': 0,
            'comments': 0,
            'empty_lines': 0,
            'invalid_lines': 0
        }
        
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        for line_no, line in enumerate(lines, 1):
            self.stats['total_lines'] += 1
            self._validate_line(line, line_no)
        
        self._print_results(filepath)
        return len(self.errors) == 0
    
    def _validate_line(self, line, line_no):
        """验证单行"""
        line = line.rstrip('\n\r')
        
        # 跳过空行
        if not line.strip():
            self.stats['empty_lines'] += 1
            return
        
        # 跳过注释
        if line.startswith('#'):
            self.stats['comments'] += 1
            return
        
        domain = line.strip()
        
        # 检查禁止的前缀
        for prefix in self.INVALID_PREFIXES:
            if domain.startswith(prefix):
                self.errors.append(
                    f"行 {line_no}: 不应该有前缀 '{prefix}' - {domain}"
                )
                self.stats['invalid_lines'] += 1
                return
        
        # 检查禁止的后缀
        for suffix in self.INVALID_SUFFIXES:
            if domain.endswith(suffix):
                self.errors.append(
                    f"行 {line_no}: 不应该有后缀 '{suffix}' - {domain}"
                )
                self.stats['invalid_lines'] += 1
                return
        
        # 检查是否包含协议
        if '://' in domain:
            self.errors.append(
                f"行 {line_no}: 不应该包含协议 - {domain}"
            )
            self.stats['invalid_lines'] += 1
            return
        
        # 检查是否包含路径
        if '/' in domain:
            self.errors.append(
                f"行 {line_no}: 不应该包含路径 - {domain}"
            )
            self.stats['invalid_lines'] += 1
            return
        
        # 检查是否包含端口
        if ':' in domain and not domain.startswith('['):
            self.errors.append(
                f"行 {line_no}: 不应该包含端口 - {domain}"
            )
            self.stats['invalid_lines'] += 1
            return
        
        # 检查是否包含等号（赋值格式）
        if '=' in domain:
            self.errors.append(
                f"行 {line_no}: 不应该使用赋值格式 - {domain}"
            )
            self.stats['invalid_lines'] += 1
            return
        
        # 检查是否是有效的域名
        if not self.VALID_DOMAIN_PATTERN.match(domain):
            self.errors.append(
                f"行 {line_no}: 无效的域名格式 - {domain}"
            )
            self.stats['invalid_lines'] += 1
            return
        
        self.stats['valid_domains'] += 1
    
    def _print_results(self, filepath):
        """打印验证结果"""
        print(f"\n统计信息:")
        print(f"  总行数: {self.stats['total_lines']}")
        print(f"  有效域名: {self.stats['valid_domains']} ✓")
        print(f"  注释行: {self.stats['comments']}")
        print(f"  空行: {self.stats['empty_lines']}")
        print(f"  无效行: {self.stats['invalid_lines']}")
        
        if self.errors:
            print(f"\n❌ 发现 {len(self.errors)} 个错误:")
            for error in self.errors[:10]:  # 只显示前10个错误
                print(f"  - {error}")
            if len(self.errors) > 10:
                print(f"  ... 还有 {len(self.errors) - 10} 个错误")
        else:
            print("\n✅ 所有域名格式都是有效的!")
        
        if self.warnings:
            print(f"\n⚠️ 发现 {len(self.warnings)} 个警告:")
            for warning in self.warnings[:5]:
                print(f"  - {warning}")


def main():
    """主函数"""
    if len(sys.argv) < 2:
        # 如果没有指定文件，检查默认的域名列表文件
        default_files = [
            'direct_domains.txt',
            'proxy_domains.txt',
            'adblock_domains.txt',
            'custom_domains.txt'
        ]
        files = [f for f in default_files if os.path.exists(f)]
        
        if not files:
            print("用法: python validate_domain_lists.py [files...]")
            print("例如: python validate_domain_lists.py direct_domains.txt proxy_domains.txt")
            print("\n如果没有指定文件，将检查以下默认文件（如果存在）:")
            for f in default_files:
                print(f"  - {f}")
            sys.exit(1)
    else:
        files = sys.argv[1:]
    
    validator = DomainListValidator()
    all_valid = True
    
    for filepath in files:
        if not validator.validate_file(filepath):
            all_valid = False
    
    print("\n" + "=" * 60)
    if all_valid:
        print("✅ 所有文件验证通过!")
        sys.exit(0)
    else:
        print("❌ 部分文件验证失败，请检查上面的错误信息")
        sys.exit(1)


if __name__ == '__main__':
    main()
