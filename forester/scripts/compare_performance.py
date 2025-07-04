#!/usr/bin/env python3
"""
Performance Comparison Script for Forester Logs
Compares queue processing performance between old and new forester versions.
"""

import re
import sys
import argparse
from datetime import datetime
from collections import defaultdict
from typing import Dict, List, Tuple, Optional
import statistics

class PerformanceAnalyzer:
    def __init__(self):
        # ANSI color removal
        self.ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
        
        # Patterns
        self.timestamp_pattern = re.compile(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z)')
        self.queue_metric_pattern = re.compile(r'QUEUE_METRIC: (queue_empty|queue_has_elements) tree_type=(\S+) tree=(\S+)')
        self.operation_start_pattern = re.compile(r'V2_TPS_METRIC: operation_start tree_type=(\w+)')
        self.operation_complete_pattern = re.compile(r'V2_TPS_METRIC: operation_complete.*?duration_ms=(\d+).*?items_processed=(\d+)')
        self.transaction_sent_pattern = re.compile(r'V2_TPS_METRIC: transaction_sent.*?tx_duration_ms=(\d+)')
    
    def clean_line(self, line: str) -> str:
        return self.ansi_escape.sub('', line)
    
    def parse_timestamp(self, line: str) -> Optional[datetime]:
        timestamp_match = self.timestamp_pattern.search(line)
        if timestamp_match:
            return datetime.fromisoformat(timestamp_match.group(1).replace('Z', '+00:00'))
        return None
    
    def analyze_log(self, filename: str) -> Dict:
        """Comprehensive analysis of a log file."""
        results = {
            'filename': filename,
            'queue_events': [],
            'operations': [],
            'transactions': [],
            'queue_emptying_times': [],
            'queue_response_times': [],
            'processing_rates': [],
            'transaction_durations': []
        }
        
        with open(filename, 'r') as f:
            current_operation = None
            
            for line in f:
                clean_line = self.clean_line(line)
                timestamp = self.parse_timestamp(clean_line)
                
                if not timestamp:
                    continue
                
                # Parse queue metrics
                if 'QUEUE_METRIC:' in clean_line:
                    queue_match = self.queue_metric_pattern.search(clean_line)
                    if queue_match:
                        state = queue_match.group(1)
                        results['queue_events'].append((timestamp, state))
                
                # Parse operation start
                elif 'operation_start' in clean_line:
                    start_match = self.operation_start_pattern.search(clean_line)
                    if start_match:
                        current_operation = {
                            'start_time': timestamp,
                            'tree_type': start_match.group(1)
                        }
                
                # Parse operation complete
                elif 'operation_complete' in clean_line and current_operation:
                    complete_match = self.operation_complete_pattern.search(clean_line)
                    if complete_match:
                        duration_ms = int(complete_match.group(1))
                        items_processed = int(complete_match.group(2))
                        
                        operation = {
                            'start_time': current_operation['start_time'],
                            'end_time': timestamp,
                            'duration_ms': duration_ms,
                            'items_processed': items_processed,
                            'tree_type': current_operation['tree_type'],
                            'processing_rate': items_processed / (duration_ms / 1000) if duration_ms > 0 else 0
                        }
                        
                        results['operations'].append(operation)
                        results['processing_rates'].append(operation['processing_rate'])
                        current_operation = None
                
                # Parse transaction sent
                elif 'transaction_sent' in clean_line:
                    tx_match = self.transaction_sent_pattern.search(clean_line)
                    if tx_match:
                        tx_duration = int(tx_match.group(1))
                        results['transactions'].append({
                            'timestamp': timestamp,
                            'duration_ms': tx_duration
                        })
                        results['transaction_durations'].append(tx_duration)
        
        # Calculate queue metrics
        self._calculate_queue_metrics(results)
        
        return results
    
    def _calculate_queue_metrics(self, results: Dict):
        """Calculate queue emptying and response times."""
        events = results['queue_events']
        
        for i in range(1, len(events)):
            prev_time, prev_state = events[i-1]
            curr_time, curr_state = events[i]
            
            duration = (curr_time - prev_time).total_seconds()
            
            # Queue emptying time: has_elements -> empty
            if prev_state == 'queue_has_elements' and curr_state == 'queue_empty':
                results['queue_emptying_times'].append(duration)
            
            # Response time: empty -> has_elements (filter out immediate responses)
            elif prev_state == 'queue_empty' and curr_state == 'queue_has_elements':
                if duration > 0.01:  # Filter immediate responses
                    results['queue_response_times'].append(duration)
    
    def generate_stats(self, data: List[float], name: str) -> Dict:
        """Generate statistics for a dataset."""
        if not data:
            return {'name': name, 'count': 0}
        
        return {
            'name': name,
            'count': len(data),
            'min': min(data),
            'max': max(data),
            'mean': statistics.mean(data),
            'median': statistics.median(data),
            'std_dev': statistics.stdev(data) if len(data) > 1 else 0
        }
    
    def print_stats(self, stats: Dict, unit: str = ""):
        """Print statistics in a formatted way."""
        if stats['count'] == 0:
            print(f"  {stats['name']}: No data")
            return
        
        print(f"  {stats['name']}:")
        print(f"    Count: {stats['count']}")
        print(f"    Min: {stats['min']:.2f}{unit}")
        print(f"    Max: {stats['max']:.2f}{unit}")
        print(f"    Mean: {stats['mean']:.2f}{unit}")
        print(f"    Median: {stats['median']:.2f}{unit}")
        if stats['count'] > 1:
            print(f"    Std Dev: {stats['std_dev']:.2f}{unit}")
    
    def compare_stats(self, old_stats: Dict, new_stats: Dict, unit: str = "") -> Dict:
        """Compare two statistics and return improvement metrics."""
        if old_stats['count'] == 0 or new_stats['count'] == 0:
            return {'valid': False}
        
        mean_improvement = ((old_stats['mean'] - new_stats['mean']) / old_stats['mean']) * 100
        median_improvement = ((old_stats['median'] - new_stats['median']) / old_stats['median']) * 100
        
        return {
            'valid': True,
            'mean_improvement': mean_improvement,
            'median_improvement': median_improvement,
            'old_mean': old_stats['mean'],
            'new_mean': new_stats['mean'],
            'old_median': old_stats['median'],
            'new_median': new_stats['median'],
            'unit': unit
        }
    
    def print_comparison(self, comparison: Dict, metric_name: str):
        """Print comparison results."""
        if not comparison['valid']:
            print(f"  {metric_name}: Insufficient data for comparison")
            return
        
        unit = comparison['unit']
        print(f"  {metric_name}:")
        print(f"    Mean: {comparison['old_mean']:.2f}{unit} → {comparison['new_mean']:.2f}{unit} ({comparison['mean_improvement']:+.1f}%)")
        print(f"    Median: {comparison['old_median']:.2f}{unit} → {comparison['new_median']:.2f}{unit} ({comparison['median_improvement']:+.1f}%)")
    
    def analyze_and_compare(self, old_file: str, new_file: str):
        """Main analysis and comparison function."""
        print("FORESTER PERFORMANCE COMPARISON")
        print("=" * 60)
        print()
        
        # Analyze both files
        print("Analyzing log files...")
        old_results = self.analyze_log(old_file)
        new_results = self.analyze_log(new_file)
        
        print(f"Old version: {old_file}")
        print(f"New version: {new_file}")
        print()
        
        # Overall summary
        print("OVERALL SUMMARY")
        print("-" * 40)
        print(f"Old version: {len(old_results['operations'])} operations, {len(old_results['transactions'])} transactions")
        print(f"New version: {len(new_results['operations'])} operations, {len(new_results['transactions'])} transactions")
        print()
        
        # Queue Performance Analysis
        print("QUEUE PERFORMANCE ANALYSIS")
        print("-" * 40)
        
        # Queue emptying times
        old_emptying = self.generate_stats(old_results['queue_emptying_times'], "Queue Emptying Time")
        new_emptying = self.generate_stats(new_results['queue_emptying_times'], "Queue Emptying Time")
        
        print("Old Version:")
        self.print_stats(old_emptying, "s")
        print()
        print("New Version:")
        self.print_stats(new_emptying, "s")
        print()
        
        emptying_comparison = self.compare_stats(old_emptying, new_emptying, "s")
        print("COMPARISON - Queue Emptying:")
        self.print_comparison(emptying_comparison, "Queue Emptying Time")
        print()
        
        # Response times
        old_response = self.generate_stats(old_results['queue_response_times'], "Response Time")
        new_response = self.generate_stats(new_results['queue_response_times'], "Response Time")
        
        response_comparison = self.compare_stats(old_response, new_response, "s")
        print("COMPARISON - Response Time:")
        self.print_comparison(response_comparison, "Response Time to New Work")
        print()
        
        # Transaction Performance Analysis
        print("TRANSACTION PERFORMANCE ANALYSIS")
        print("-" * 40)
        
        old_tx = self.generate_stats(old_results['transaction_durations'], "Transaction Duration")
        new_tx = self.generate_stats(new_results['transaction_durations'], "Transaction Duration")
        
        tx_comparison = self.compare_stats(old_tx, new_tx, "ms")
        print("COMPARISON - Transaction Duration:")
        self.print_comparison(tx_comparison, "Individual Transaction Time")
        print()
        
        # Processing Rate Analysis
        print("PROCESSING RATE ANALYSIS")
        print("-" * 40)
        
        old_rate = self.generate_stats(old_results['processing_rates'], "Processing Rate")
        new_rate = self.generate_stats(new_results['processing_rates'], "Processing Rate")
        
        rate_comparison = self.compare_stats(old_rate, new_rate, " items/sec")
        print("COMPARISON - Processing Rate:")
        self.print_comparison(rate_comparison, "Items Processing Rate")
        print()
        
        # Key Insights
        print("KEY INSIGHTS")
        print("-" * 40)
        
        insights = []
        
        if emptying_comparison['valid']:
            if emptying_comparison['mean_improvement'] > 0:
                insights.append(f"✅ Queue emptying is {emptying_comparison['mean_improvement']:.1f}% faster")
            else:
                insights.append(f"⚠️ Queue emptying is {abs(emptying_comparison['mean_improvement']):.1f}% slower")
        
        if response_comparison['valid']:
            if response_comparison['mean_improvement'] > 0:
                insights.append(f"✅ Response to new work is {response_comparison['mean_improvement']:.1f}% faster")
            else:
                insights.append(f"⚠️ Response to new work is {abs(response_comparison['mean_improvement']):.1f}% slower")
        
        if tx_comparison['valid']:
            if tx_comparison['median_improvement'] > 0:
                insights.append(f"✅ Individual transactions are {tx_comparison['median_improvement']:.1f}% faster")
            else:
                insights.append(f"⚠️ Individual transactions are {abs(tx_comparison['median_improvement']):.1f}% slower")
        
        if rate_comparison['valid']:
            if rate_comparison['mean_improvement'] > 0:
                insights.append(f"✅ Processing rate improved by {rate_comparison['mean_improvement']:.1f}%")
            else:
                insights.append(f"⚠️ Processing rate decreased by {abs(rate_comparison['mean_improvement']):.1f}%")
        
        for insight in insights:
            print(f"  {insight}")
        
        if not insights:
            print("  No significant performance differences detected")
        
        print()
        print("=" * 60)

def main():
    parser = argparse.ArgumentParser(description='Compare forester performance between two log files')
    parser.add_argument('old_log', help='Path to old version log file')
    parser.add_argument('new_log', help='Path to new version log file')
    
    args = parser.parse_args()
    
    analyzer = PerformanceAnalyzer()
    try:
        analyzer.analyze_and_compare(args.old_log, args.new_log)
    except FileNotFoundError as e:
        print(f"Error: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"Error analyzing logs: {e}")
        sys.exit(1)

if __name__ == '__main__':
    main()