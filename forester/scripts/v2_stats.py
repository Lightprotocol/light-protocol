#!/usr/bin/env python3

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
        
        self.v1_operation_start_pattern = re.compile(
            r'V1_TPS_METRIC: operation_start tree_type=(\w+) tree=(\S+) epoch=(\d+)'
        )
        self.v1_operation_complete_pattern = re.compile(
            r'V1_TPS_METRIC: operation_complete tree_type=(\w+) tree=(\S+) epoch=(\d+) transactions=(\d+) duration_ms=(\d+) tps=([\d.]+)'
        )
        self.v2_operation_start_pattern = re.compile(
            r'V2_TPS_METRIC: operation_start tree_type=(\w+) (?:operation=(\w+) )?tree=(\S+) epoch=(\d+)'
        )
        self.v2_operation_complete_pattern = re.compile(
            r'V2_TPS_METRIC: operation_complete tree_type=(\w+) (?:operation=(\w+) )?tree=(\S+) epoch=(\d+) zkp_batches=(\d+) transactions=(\d+) instructions=(\d+) duration_ms=(\d+) tps=([\d.]+) ips=([\d.]+)(?:\s+items_processed=(\d+))?'
        )
        self.v2_transaction_sent_pattern = re.compile(
            r'V2_TPS_METRIC: transaction_sent tree_type=(\w+) (?:operation=(\w+) )?tree=(\S+) tx_num=(\d+) signature=(\S+) instructions=(\d+) tx_duration_ms=(\d+)'
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
        """Parse a single log line for V1/V2 TPS metrics."""
        clean_line = self.clean_line(line)
        timestamp = self.parse_timestamp(clean_line)
        
        if not timestamp:
            return
        
        # Parse V1 operation start
        v1_start_match = self.v1_operation_start_pattern.search(clean_line)
        if v1_start_match:
            self.operations.append({
                'type': 'start',
                'version': 'V1',
                'timestamp': timestamp,
                'tree_type': v1_start_match.group(1),
                'tree': v1_start_match.group(2),
                'epoch': int(v1_start_match.group(3))
            })
            return
        
        # Parse V1 operation complete
        v1_complete_match = self.v1_operation_complete_pattern.search(clean_line)
        if v1_complete_match:
            self.operation_summaries.append({
                'version': 'V1',
                'timestamp': timestamp,
                'tree_type': v1_complete_match.group(1),
                'tree': v1_complete_match.group(2),
                'epoch': int(v1_complete_match.group(3)),
                'transactions': int(v1_complete_match.group(4)),
                'duration_ms': int(v1_complete_match.group(5)),
                'tps': float(v1_complete_match.group(6)),
                'zkp_batches': 0,  # V1 doesn't have zkp batches
                'instructions': int(v1_complete_match.group(4)),  # For V1, instructions = transactions
                'ips': float(v1_complete_match.group(6)),  # For V1, ips = tps
                'items_processed': 0
            })
            return
        
        # Parse V2 operation start
        v2_start_match = self.v2_operation_start_pattern.search(clean_line)
        if v2_start_match:
            self.operations.append({
                'type': 'start',
                'version': 'V2',
                'timestamp': timestamp,
                'tree_type': v2_start_match.group(1),
                'operation': v2_start_match.group(2) or 'batch',
                'tree': v2_start_match.group(3),
                'epoch': int(v2_start_match.group(4))
            })
            return
        
        # Parse V2 operation complete
        v2_complete_match = self.v2_operation_complete_pattern.search(clean_line)
        if v2_complete_match:
            self.operation_summaries.append({
                'version': 'V2',
                'timestamp': timestamp,
                'tree_type': v2_complete_match.group(1),
                'operation': v2_complete_match.group(2) or 'batch',
                'tree': v2_complete_match.group(3),
                'epoch': int(v2_complete_match.group(4)),
                'zkp_batches': int(v2_complete_match.group(5)),
                'transactions': int(v2_complete_match.group(6)),
                'instructions': int(v2_complete_match.group(7)),
                'duration_ms': int(v2_complete_match.group(8)),
                'tps': float(v2_complete_match.group(9)),
                'ips': float(v2_complete_match.group(10)),
                'items_processed': int(v2_complete_match.group(11)) if v2_complete_match.group(11) else 0
            })
            return
        
        # Parse V2 transaction sent
        v2_tx_match = self.v2_transaction_sent_pattern.search(clean_line)
        if v2_tx_match:
            self.transactions.append({
                'version': 'V2',
                'timestamp': timestamp,
                'tree_type': v2_tx_match.group(1),
                'operation': v2_tx_match.group(2) or 'batch',
                'tree': v2_tx_match.group(3),
                'tx_num': int(v2_tx_match.group(4)),
                'signature': v2_tx_match.group(5),
                'instructions': int(v2_tx_match.group(6)),
                'tx_duration_ms': int(v2_tx_match.group(7))
            })
    
    def print_summary_stats(self) -> None:
        """Print high-level summary statistics."""
        print("\n" + "="*80)
        print("FORESTER PERFORMANCE ANALYSIS REPORT (V1 & V2)")
        print("="*80)
        
        if not self.operation_summaries:
            print("No TPS metrics found in logs")
            return
        
        print(f"\nSUMMARY:")
        print(f"  Total operations analyzed: {len(self.operation_summaries)}")
        
        # Count total transactions from operation summaries
        total_txs_from_ops = sum(op.get('transactions', 0) for op in self.operation_summaries)
        print(f"  Total transactions (from operations): {total_txs_from_ops}")
        print(f"  Total transaction events logged: {len(self.transactions)}")
        
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
        print("\nNOTE: V1 and V2 use different transaction models:")
        print("  V1: 1 tree update = 1 transaction (~1 slot/400ms latency)")
        print("  V2: 10+ tree updates = 1 transaction (multi-slot batching + ZKP generation)")
        print("  ")
        print("  TPS comparison is misleading - V2 optimizes for cost efficiency, not transaction count.")
        print("  Focus on 'Items Processed Per Second' and 'Total items processed' for V2.")
        print("  V2's higher latency is architectural (batching) not a performance issue.")
        print()
        
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
                
                # For V2 trees, show Items Processed Per Second (more meaningful than TPS)
                if 'V2' in tree_type and stats['items_processed'] > 0:
                    items_per_second = stats['items_processed'] / (stats['total_duration_ms'] / 1000)
                    print(f"  *** Items Processed Per Second (IPPS): {items_per_second:.2f} ***")
                    print(f"      ^ This is the meaningful throughput metric for V2 (actual tree updates/sec)")
                    
                    # Show batching efficiency
                    if stats['total_zkp_batches'] > 0:
                        avg_items_per_batch = stats['items_processed'] / stats['total_zkp_batches']
                        print(f"  Avg items per ZKP batch: {avg_items_per_batch:.1f}")
    
    def generate_report(self) -> None:
        """Generate comprehensive TPS analysis report."""
        self.print_summary_stats()
        self.print_tree_type_analysis()
        print("\n" + "="*80)

def main():
    parser = argparse.ArgumentParser(description='Analyze forester performance metrics (V1 & V2) - Focus on IPPS for V2')
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
            if 'TPS_METRIC' not in line:  # Match both V1 and V2
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