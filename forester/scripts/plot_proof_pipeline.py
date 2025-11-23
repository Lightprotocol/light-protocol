#!/usr/bin/env python3
"""
Visualize the proof-to-transaction pipeline timing.

Shows:
1. When proof jobs are submitted to prover
2. When proofs complete
3. When transactions are sent

This helps identify:
- Queue wait time (submission → completion)
- TX batching delays (proof ready → tx sent)
- Parallel proof generation patterns
"""

import re
import sys
from datetime import datetime
from collections import defaultdict
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from matplotlib.patches import Rectangle
import numpy as np

def parse_timestamp(ts_str):
    """Parse ISO timestamp to datetime."""
    # Handle format: 2025-12-09T14:18:41.265968Z
    ts_str = ts_str.rstrip('Z')
    if '.' in ts_str:
        return datetime.fromisoformat(ts_str)
    return datetime.fromisoformat(ts_str)

def parse_log(filename):
    """Parse log file for proof lifecycle events."""

    submissions = []  # (timestamp, job_id, seq, type, tree)
    completions = []  # (timestamp, job_id, seq, round_trip_ms, proof_ms)
    txs = []          # (timestamp, type, ixs, seq_range, epoch)

    # Patterns
    submit_pattern = re.compile(
        r'(\d{4}-\d{2}-\d{2}T[\d:.]+)Z.*Submitted proof job seq=(\d+) type=(\w+) job_id=([\w-]+)'
    )
    complete_pattern = re.compile(
        r'(\d{4}-\d{2}-\d{2}T[\d:.]+)Z.*Proof completed for seq=(\d+) job_id=([\w-]+) round_trip=(\d+)ms proof=(\d+)ms'
    )
    # Updated pattern to capture tree, timing info
    # Format: tx sent: <sig> type=<type> ixs=<count> tree=<pubkey> root=<root> seq=<start>..<end> epoch=<epoch> e2e=<ms>ms
    tx_pattern = re.compile(
        r'(\d{4}-\d{2}-\d{2}T[\d:.]+)Z.*tx sent: \w+ type=([^\s]+) ixs=(\d+)(?: tree=(\w+))?(?: root=\[[^\]]+\])? seq=(\d+)\.\.(\d+) epoch=(\d+)(?: e2e=(\d+)ms)?'
    )

    with open(filename, 'r') as f:
        for line in f:
            # Remove ANSI codes
            line = re.sub(r'\x1b\[[0-9;]*m', '', line)

            if m := submit_pattern.search(line):
                ts = parse_timestamp(m.group(1))
                submissions.append({
                    'timestamp': ts,
                    'seq': int(m.group(2)),
                    'type': m.group(3),
                    'job_id': m.group(4),
                })

            if m := complete_pattern.search(line):
                ts = parse_timestamp(m.group(1))
                completions.append({
                    'timestamp': ts,
                    'seq': int(m.group(2)),
                    'job_id': m.group(3),
                    'round_trip_ms': int(m.group(4)),
                    'proof_ms': int(m.group(5)),
                })

            if m := tx_pattern.search(line):
                ts = parse_timestamp(m.group(1))
                txs.append({
                    'timestamp': ts,
                    'type': m.group(2),
                    'ixs': int(m.group(3)),
                    'tree': m.group(4) if m.group(4) else None,
                    'seq_start': int(m.group(5)),
                    'seq_end': int(m.group(6)),
                    'epoch': int(m.group(7)),
                    'e2e_ms': int(m.group(8)) if m.group(8) else None,
                })

    return submissions, completions, txs

def plot_pipeline(submissions, completions, txs, output_file='proof_pipeline.png'):
    """Create timeline visualization."""

    if not submissions and not completions and not txs:
        print("No data to plot!")
        return

    fig, axes = plt.subplots(3, 1, figsize=(16, 12), sharex=True)

    # Get time range
    all_times = []
    if submissions:
        all_times.extend([s['timestamp'] for s in submissions])
    if completions:
        all_times.extend([c['timestamp'] for c in completions])
    if txs:
        all_times.extend([t['timestamp'] for t in txs])

    if not all_times:
        print("No timestamps found!")
        return

    min_time = min(all_times)
    max_time = max(all_times)

    # Convert to seconds from start
    def to_seconds(dt):
        return (dt - min_time).total_seconds()

    # Color map for proof types
    type_colors = {
        'append': '#2ecc71',      # green
        'update': '#e74c3c',      # red
        'address_append': '#3498db',  # blue
    }

    # Plot 1: Proof Submissions
    ax1 = axes[0]
    ax1.set_title('Proof Job Submissions (when sent to prover)', fontsize=12, fontweight='bold')
    ax1.set_ylabel('Proof Type')

    type_y = {'append': 0, 'update': 1, 'address_append': 2}
    for sub in submissions:
        t = to_seconds(sub['timestamp'])
        y = type_y.get(sub['type'], 0)
        color = type_colors.get(sub['type'], 'gray')
        ax1.scatter(t, y, c=color, alpha=0.6, s=20, marker='|')

    ax1.set_yticks([0, 1, 2])
    ax1.set_yticklabels(['append', 'update', 'address_append'])
    ax1.set_ylim(-0.5, 2.5)
    ax1.grid(True, alpha=0.3)

    # Plot 2: Proof Completions with round-trip time
    ax2 = axes[1]
    ax2.set_title('Proof Completions (color = round-trip time)', fontsize=12, fontweight='bold')
    ax2.set_ylabel('Round-trip (ms)')

    if completions:
        times = [to_seconds(c['timestamp']) for c in completions]
        round_trips = [c['round_trip_ms'] for c in completions]

        scatter = ax2.scatter(times, round_trips, c=round_trips, cmap='RdYlGn_r',
                             alpha=0.7, s=30, vmin=0, vmax=max(10000, max(round_trips)))
        plt.colorbar(scatter, ax=ax2, label='Round-trip (ms)')

    ax2.set_yscale('log')
    ax2.grid(True, alpha=0.3)

    # Plot 3: Transaction Timeline
    ax3 = axes[2]
    ax3.set_title('Transactions Sent', fontsize=12, fontweight='bold')
    ax3.set_ylabel('Epoch')
    ax3.set_xlabel('Time (seconds from start)')

    if txs:
        times = [to_seconds(t['timestamp']) for t in txs]
        epochs = [t['epoch'] for t in txs]

        # Color by tx type
        tx_colors = []
        for tx in txs:
            if 'Append+Nullify' in tx['type']:
                tx_colors.append('#9b59b6')  # purple
            elif 'Append' in tx['type']:
                tx_colors.append('#2ecc71')  # green
            elif 'Nullify' in tx['type']:
                tx_colors.append('#e74c3c')  # red
            elif 'Address' in tx['type']:
                tx_colors.append('#3498db')  # blue
            else:
                tx_colors.append('gray')

        ax3.scatter(times, epochs, c=tx_colors, alpha=0.7, s=50, marker='s')

        # Add vertical lines for epoch boundaries
        epoch_changes = []
        prev_epoch = None
        for tx in txs:
            if prev_epoch is not None and tx['epoch'] != prev_epoch:
                epoch_changes.append(to_seconds(tx['timestamp']))
            prev_epoch = tx['epoch']

        for ec in epoch_changes:
            ax3.axvline(x=ec, color='red', linestyle='--', alpha=0.5, linewidth=1)

    ax3.grid(True, alpha=0.3)

    # Add legend
    from matplotlib.lines import Line2D
    legend_elements = [
        Line2D([0], [0], marker='s', color='w', markerfacecolor='#2ecc71', markersize=10, label='Append'),
        Line2D([0], [0], marker='s', color='w', markerfacecolor='#e74c3c', markersize=10, label='Nullify'),
        Line2D([0], [0], marker='s', color='w', markerfacecolor='#3498db', markersize=10, label='AddressAppend'),
        Line2D([0], [0], marker='s', color='w', markerfacecolor='#9b59b6', markersize=10, label='Append+Nullify'),
        Line2D([0], [0], color='red', linestyle='--', alpha=0.5, label='Epoch change'),
    ]
    ax3.legend(handles=legend_elements, loc='upper right')

    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    print(f"Saved pipeline visualization to {output_file}")

    # Print statistics
    print(f"\n{'='*60}")
    print("PROOF PIPELINE STATISTICS")
    print('='*60)
    print(f"Total duration: {(max_time - min_time).total_seconds():.1f}s")
    print(f"Proof submissions: {len(submissions)}")
    print(f"Proof completions: {len(completions)}")
    print(f"Transactions sent: {len(txs)}")

    if completions:
        round_trips = [c['round_trip_ms'] for c in completions]
        print(f"\nRound-trip times:")
        print(f"  Min: {min(round_trips)}ms")
        print(f"  Max: {max(round_trips)}ms")
        print(f"  Mean: {np.mean(round_trips):.0f}ms")
        print(f"  Median: {np.median(round_trips):.0f}ms")

    if txs:
        # Calculate inter-tx gaps
        tx_times = sorted([t['timestamp'] for t in txs])
        gaps = [(tx_times[i+1] - tx_times[i]).total_seconds() for i in range(len(tx_times)-1)]
        if gaps:
            print(f"\nInter-TX gaps:")
            print(f"  Max gap: {max(gaps):.1f}s")
            print(f"  Gaps > 5s: {sum(1 for g in gaps if g > 5)}")
            print(f"  Gaps > 10s: {sum(1 for g in gaps if g > 10)}")

        # TX timing stats (new log format)
        tx_e2e_times = [t['e2e_ms'] for t in txs if t.get('e2e_ms') is not None]
        if tx_e2e_times:
            print(f"\nTX end-to-end latency (proof submit → tx sent):")
            print(f"  Min: {min(tx_e2e_times)}ms")
            print(f"  Max: {max(tx_e2e_times)}ms")
            print(f"  Mean: {np.mean(tx_e2e_times):.0f}ms")
            print(f"  Median: {np.median(tx_e2e_times):.0f}ms")

        # Per-tree stats
        trees = set(t.get('tree') for t in txs if t.get('tree'))
        if trees:
            print(f"\nTXs per tree:")
            for tree in sorted(trees):
                count = sum(1 for t in txs if t.get('tree') == tree)
                print(f"  {tree[:8]}...: {count} txs")

def plot_proof_lifecycle(submissions, completions, output_file='proof_lifecycle.png'):
    """Create Gantt-style chart showing proof lifecycles."""

    # Match submissions to completions by job_id
    job_lifecycles = {}

    for sub in submissions:
        job_id = sub['job_id']
        job_lifecycles[job_id] = {
            'submit_time': sub['timestamp'],
            'type': sub['type'],
            'seq': sub['seq'],
        }

    for comp in completions:
        job_id = comp['job_id']
        if job_id in job_lifecycles:
            job_lifecycles[job_id]['complete_time'] = comp['timestamp']
            job_lifecycles[job_id]['round_trip_ms'] = comp['round_trip_ms']

    # Filter to jobs with both submit and complete
    complete_jobs = {k: v for k, v in job_lifecycles.items()
                    if 'complete_time' in v}

    if not complete_jobs:
        print("No complete job lifecycles found!")
        return

    # Sort by submit time
    sorted_jobs = sorted(complete_jobs.values(), key=lambda x: x['submit_time'])

    # Limit to first 100 for readability
    sorted_jobs = sorted_jobs[:100]

    fig, ax = plt.subplots(figsize=(16, 10))

    min_time = min(j['submit_time'] for j in sorted_jobs)

    type_colors = {
        'append': '#2ecc71',
        'update': '#e74c3c',
        'address_append': '#3498db',
    }

    for i, job in enumerate(sorted_jobs):
        start = (job['submit_time'] - min_time).total_seconds()
        end = (job['complete_time'] - min_time).total_seconds()
        duration = end - start

        color = type_colors.get(job['type'], 'gray')

        # Draw bar from submit to complete
        ax.barh(i, duration, left=start, height=0.8,
               color=color, alpha=0.7, edgecolor='black', linewidth=0.5)

    ax.set_xlabel('Time (seconds from start)')
    ax.set_ylabel('Proof Job (ordered by submission)')
    ax.set_title('Proof Job Lifecycles (submit → complete)', fontsize=14, fontweight='bold')

    # Legend
    from matplotlib.patches import Patch
    legend_elements = [
        Patch(facecolor='#2ecc71', label='append'),
        Patch(facecolor='#e74c3c', label='update/nullify'),
        Patch(facecolor='#3498db', label='address_append'),
    ]
    ax.legend(handles=legend_elements, loc='upper right')

    ax.grid(True, alpha=0.3, axis='x')

    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    print(f"Saved lifecycle chart to {output_file}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python plot_proof_pipeline.py <logfile>")
        sys.exit(1)

    logfile = sys.argv[1]
    print(f"Parsing {logfile}...")

    submissions, completions, txs = parse_log(logfile)

    print(f"Found {len(submissions)} submissions, {len(completions)} completions, {len(txs)} txs")

    # Generate both visualizations
    plot_pipeline(submissions, completions, txs)
    plot_proof_lifecycle(submissions, completions)
