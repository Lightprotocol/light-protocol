#!/usr/bin/env python3
"""
V2 TPS Analyzer - Comprehensive forester TPS analysis for V2 operations.
Focuses on understanding transaction throughput, instruction rates, and processing efficiency.
"""

import re
import sys
from datetime import datetime, timedelta
from collections import defaultdict
from typing import Dict, List, Tuple, Optional
import argparse
import statistics
import json

class V2TpsAnalyzer:
    def __init__(self):
        # ANSI color removal
        self.ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
        
        # TPS metric patterns
        self.operation_start_pattern = re.compile(
            r'V2_TPS_METRIC: operation_start tree_type=(\w+) (?:operation=(\w+) )?tree=(\S+) epoch=(\d+)'
        )
        self.operation_complete_pattern = re.compile(
            r'V2_TPS_METRIC: operation_complete tree_type=(\w+) (?:operation=(\w+) )?tree=(\S+) epoch=(\d+) zkp_batches=(\d+) transactions=(\d+) instructions=(\d+) duration_ms=(\d+) tps=([\d.]+) ips=([\d.]+)(?:\s+items_processed=(\d+))?'
        )
        self.transaction_sent_pattern = re.compile(
            r'V2_TPS_METRIC: transaction_sent tree_type=(\w+) (?:operation=(\w+) )?tree=(\S+) tx_num=(\d+)/(\d+) signature=(\S+) instructions=(\d+) tx_duration_ms=(\d+)'
        )
        
        # Data storage
        self.operations: List[Dict] = []
        self.transactions: List[Dict] = []
        self.operation_summaries: List[Dict] = []
    
    def clean_line(self, line: str) -> str:
        """Remove ANSI color codes."""
        return self.ansi_escape.sub('', line)
    
    def parse_timestamp(self, line: str) -> Optional[datetime]:
        """Extract timestamp from log line."""
        timestamp_match = re.search(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z)', line)
        if timestamp_match:
            return datetime.fromisoformat(timestamp_match.group(1).replace('Z', '+00:00'))
        return None
    
    def parse_log_line(self, line: str) -> None:
        """Parse a single log line for V2 TPS metrics."""
        clean_line = self.clean_line(line)
        timestamp = self.parse_timestamp(clean_line)
        
        if not timestamp:
            return
        
        # Parse operation start
        start_match = self.operation_start_pattern.search(clean_line)
        if start_match:
            self.operations.append({
                'type': 'start',
                'timestamp': timestamp,
                'tree_type': start_match.group(1),
                'operation': start_match.group(2) or 'batch',
                'tree': start_match.group(3),
                'epoch': int(start_match.group(4))
            })
            return
        
        # Parse operation complete
        complete_match = self.operation_complete_pattern.search(clean_line)
        if complete_match:
            self.operation_summaries.append({
                'timestamp': timestamp,
                'tree_type': complete_match.group(1),
                'operation': complete_match.group(2) or 'batch',
                'tree': complete_match.group(3),
                'epoch': int(complete_match.group(4)),
                'zkp_batches': int(complete_match.group(5)),
                'transactions': int(complete_match.group(6)),
                'instructions': int(complete_match.group(7)),
                'duration_ms': int(complete_match.group(8)),
                'tps': float(complete_match.group(9)),
                'ips': float(complete_match.group(10)),
                'items_processed': int(complete_match.group(11)) if complete_match.group(11) else 0
            })
            return
        
        # Parse transaction sent
        tx_match = self.transaction_sent_pattern.search(clean_line)
        if tx_match:
            self.transactions.append({
                'timestamp': timestamp,
                'tree_type': tx_match.group(1),
                'operation': tx_match.group(2) or 'batch',
                'tree': tx_match.group(3),
                'tx_num': int(tx_match.group(4)),
                'total_txs': int(tx_match.group(5)),
                'signature': tx_match.group(6),
                'instructions': int(tx_match.group(7)),
                'tx_duration_ms': int(tx_match.group(8))
            })
    
    def print_summary_stats(self) -> None:
        """Print high-level summary statistics."""
        print("\n" + "="*80)
        print("V2 FORESTER TPS ANALYSIS REPORT")
        print("="*80)
        
        if not self.operation_summaries:
            print("No V2 TPS metrics found in logs")
            return
        
        print(f"\nSUMMARY:")
        print(f"  Total operations analyzed: {len(self.operation_summaries)}")
        print(f"  Total transactions sent: {len(self.transactions)}")
        
        # Time span
        if self.operation_summaries:
            start_time = min(op['timestamp'] for op in self.operation_summaries)
            end_time = max(op['timestamp'] for op in self.operation_summaries)
            time_span = (end_time - start_time).total_seconds()
            print(f"  Analysis time span: {time_span:.1f}s ({time_span/60:.1f} minutes)")
    
    def print_tree_type_analysis(self) -> None:
        """Analyze performance by tree type."""
        print("\n## PERFORMANCE BY TREE TYPE")
        print("-" * 60)
        
        tree_type_stats = defaultdict(lambda: {
            'operations': [],
            'total_transactions': 0,
            'total_instructions': 0,
            'total_zkp_batches': 0,
            'total_duration_ms': 0,
            'tps_values': [],
            'ips_values': [],
            'items_processed': 0
        })
        
        for op in self.operation_summaries:
            stats = tree_type_stats[op['tree_type']]
            stats['operations'].append(op)
            stats['total_transactions'] += op['transactions']
            stats['total_instructions'] += op['instructions']
            stats['total_zkp_batches'] += op['zkp_batches']
            stats['total_duration_ms'] += op['duration_ms']
            if op['tps'] > 0:
                stats['tps_values'].append(op['tps'])
            if op['ips'] > 0:
                stats['ips_values'].append(op['ips'])
            stats['items_processed'] += op['items_processed']
        
        for tree_type, stats in sorted(tree_type_stats.items()):
            print(f"\n{tree_type}:")
            print(f"  Operations: {len(stats['operations'])}")
            print(f"  Total transactions: {stats['total_transactions']}")
            print(f"  Total instructions: {stats['total_instructions']}")
            print(f"  Total ZKP batches: {stats['total_zkp_batches']}")
            print(f"  Total items processed: {stats['items_processed']}")
            print(f"  Total processing time: {stats['total_duration_ms']/1000:.2f}s")
            
            if stats['tps_values']:
                print(f"  TPS - Min: {min(stats['tps_values']):.2f}, Max: {max(stats['tps_values']):.2f}, Mean: {statistics.mean(stats['tps_values']):.2f}")
            if stats['ips_values']:
                print(f"  IPS - Min: {min(stats['ips_values']):.2f}, Max: {max(stats['ips_values']):.2f}, Mean: {statistics.mean(stats['ips_values']):.2f}")
            
            # Calculate aggregate rates
            if stats['total_duration_ms'] > 0:
                aggregate_tps = stats['total_transactions'] / (stats['total_duration_ms'] / 1000)
                aggregate_ips = stats['total_instructions'] / (stats['total_duration_ms'] / 1000)
                print(f"  Aggregate TPS: {aggregate_tps:.2f}")
                print(f"  Aggregate IPS: {aggregate_ips:.2f}")
    
    def generate_report(self) -> None:
        """Generate comprehensive TPS analysis report."""
        self.print_summary_stats()
        self.print_tree_type_analysis()
        print("\n" + "="*80)

def main():
    parser = argparse.ArgumentParser(description='Analyze V2 forester TPS metrics')
    parser.add_argument('logfile', nargs='?', default='-', help='Log file to analyze')
    parser.add_argument('--tree-type', help='Filter to specific tree type')
    
    args = parser.parse_args()
    
    analyzer = V2TpsAnalyzer()
    
    # Read and parse log file
    if args.logfile == '-':
        log_file = sys.stdin
    else:
        log_file = open(args.logfile, 'r')
    
    try:
        for line in log_file:
            if 'V2_TPS_METRIC' not in line:
                continue
                
            analyzer.parse_log_line(line)
    finally:
        if args.logfile != '-':
            log_file.close()
    
    # Apply filters
    if args.tree_type:
        analyzer.operation_summaries = [op for op in analyzer.operation_summaries if op['tree_type'] == args.tree_type]
        analyzer.transactions = [tx for tx in analyzer.transactions if tx['tree_type'] == args.tree_type]
    
    # Generate report
    analyzer.generate_report()

if __name__ == '__main__':
    main()