#!/usr/bin/env python3
"""
Enhanced Forester Performance Analysis Tool

Parses forester logs and generates comprehensive performance visualizations including:
- Proof round-trip latency distribution and timeline
- Transaction throughput with gap analysis
- Pipeline utilization (proof requests vs completions)
- Queue drain rates and bottleneck identification
- Indexer sync wait analysis
"""

import argparse
import re
from pathlib import Path
from dataclasses import dataclass
from typing import Optional
from datetime import datetime, timedelta

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
import numpy as np

# Regex patterns
TS_RE = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z")
ROUND_TRIP_RE = re.compile(r"round_trip=(\d+)ms")
PROOF_TIME_RE = re.compile(r"proof=(\d+)ms")
IXS_RE = re.compile(r"ixs=(\d+)")
TYPE_RE = re.compile(r"type=(\w+)")
QUEUE_ITEMS_RE = re.compile(r"(\d+)\s+items")
BATCH_LIMIT_RE = re.compile(r"Queue size (\d+) would produce (\d+) batches, limiting to (\d+)")
CIRCUIT_RE = re.compile(r"circuit type:\s*(\w+)")


@dataclass
class ProofEvent:
    timestamp: datetime
    round_trip_ms: int
    seq: Optional[int] = None
    job_id: Optional[str] = None
    proof_type: Optional[str] = None
    proof_ms: Optional[int] = None  # Pure proof generation time from prover server

    @property
    def queue_wait_ms(self) -> Optional[int]:
        """Time spent waiting in queue (round_trip - proof)."""
        if self.proof_ms is not None:
            return self.round_trip_ms - self.proof_ms
        return None


@dataclass
class TxEvent:
    timestamp: datetime
    ixs: int
    tx_type: str
    tx_hash: str


@dataclass
class ProofRequest:
    timestamp: datetime
    circuit_type: str


@dataclass
class BottleneckEvent:
    timestamp: datetime
    event_type: str  # 'indexer_sync', 'batch_limit', 'idle'
    details: str


def parse_timestamp(line: str) -> Optional[datetime]:
    """Extract timestamp from log line."""
    m = TS_RE.search(line)
    if m:
        return datetime.fromisoformat(m.group(0).replace('Z', '+00:00'))
    return None


def parse_log(path: Path) -> dict:
    """Parse forester log and extract all performance-relevant events."""
    proof_completions: list[ProofEvent] = []
    tx_events: list[TxEvent] = []
    proof_requests: list[ProofRequest] = []
    bottlenecks: list[BottleneckEvent] = []
    queue_updates: list[tuple[datetime, str, int]] = []  # (ts, tree, items)
    job_types: dict[str, str] = {}  # job_id -> proof_type

    with path.open("r", encoding="utf-8", errors="replace") as f:
        for line in f:
            ts = parse_timestamp(line)
            if not ts:
                continue

            # Submitted proof job (to map job_id -> type)
            if "Submitted proof job" in line:
                m = re.search(r'type=(\w+)\s+job_id=([a-f0-9-]+)', line)
                if m:
                    job_types[m.group(2)] = m.group(1)

            # Proof completions
            if "Proof completed" in line:
                m = ROUND_TRIP_RE.search(line)
                job_m = re.search(r'job_id=([a-f0-9-]+)', line)
                proof_m = PROOF_TIME_RE.search(line)
                if m:
                    job_id = job_m.group(1) if job_m else None
                    proof_type = job_types.get(job_id) if job_id else None
                    proof_ms = int(proof_m.group(1)) if proof_m else None
                    proof_completions.append(ProofEvent(
                        timestamp=ts,
                        round_trip_ms=int(m.group(1)),
                        job_id=job_id,
                        proof_type=proof_type,
                        proof_ms=proof_ms
                    ))

            # TX sent
            elif "tx sent:" in line:
                ixs_m = IXS_RE.search(line)
                type_m = TYPE_RE.search(line)
                if ixs_m and type_m:
                    # Extract tx hash (first base58 string after "tx sent:")
                    hash_start = line.find("tx sent:") + 9
                    hash_end = line.find(" ", hash_start)
                    tx_hash = line[hash_start:hash_end] if hash_end > hash_start else ""
                    tx_events.append(TxEvent(
                        timestamp=ts,
                        ixs=int(ixs_m.group(1)),
                        tx_type=type_m.group(1),
                        tx_hash=tx_hash
                    ))

            # Proof requests
            elif "Submitting async proof request" in line:
                m = CIRCUIT_RE.search(line)
                if m:
                    proof_requests.append(ProofRequest(
                        timestamp=ts,
                        circuit_type=m.group(1)
                    ))

            # Indexer sync waits
            elif "waiting for indexer sync" in line:
                bottlenecks.append(BottleneckEvent(
                    timestamp=ts,
                    event_type="indexer_sync",
                    details=line.strip()
                ))

            # Batch limiting
            elif "would produce" in line and "limiting to" in line:
                m = BATCH_LIMIT_RE.search(line)
                if m:
                    bottlenecks.append(BottleneckEvent(
                        timestamp=ts,
                        event_type="batch_limit",
                        details=f"Queue {m.group(1)} -> {m.group(2)} batches, limited to {m.group(3)}"
                    ))

            # Queue updates
            elif "Routed update to tree" in line:
                m = QUEUE_ITEMS_RE.search(line)
                if m:
                    # Extract tree name
                    tree_start = line.find("tree ") + 5
                    tree_end = line.find(":", tree_start)
                    tree = line[tree_start:tree_end] if tree_end > tree_start else "unknown"
                    queue_updates.append((ts, tree, int(m.group(1))))

    return {
        "proof_completions": proof_completions,
        "tx_events": tx_events,
        "proof_requests": proof_requests,
        "bottlenecks": bottlenecks,
        "queue_updates": queue_updates
    }


def plot_latency_distribution(proof_completions: list[ProofEvent], ax):
    """Plot round-trip latency histogram with percentiles."""
    if not proof_completions:
        ax.text(0.5, 0.5, "No proof data", ha='center', va='center')
        return

    latencies = [p.round_trip_ms for p in proof_completions]

    # Histogram - ensure bins are monotonically increasing
    max_latency = max(latencies) if latencies else 1000
    base_bins = [0, 500, 1000, 2000, 5000, 10000, 20000]
    # Only keep bins smaller than max_latency, then add final bin
    bins = [b for b in base_bins if b < max_latency] + [max_latency + 1000]
    if len(bins) < 2:
        bins = [0, max_latency + 1000]
    ax.hist(latencies, bins=bins, edgecolor='black', alpha=0.7, color='steelblue')

    # Percentile lines
    p50 = np.percentile(latencies, 50)
    p95 = np.percentile(latencies, 95)
    p99 = np.percentile(latencies, 99)

    ax.axvline(p50, color='green', linestyle='--', linewidth=2, label=f'p50: {p50:.0f}ms')
    ax.axvline(p95, color='orange', linestyle='--', linewidth=2, label=f'p95: {p95:.0f}ms')
    ax.axvline(p99, color='red', linestyle='--', linewidth=2, label=f'p99: {p99:.0f}ms')

    ax.set_xlabel('Round-trip Latency (ms)')
    ax.set_ylabel('Count')
    ax.set_title(f'Proof Latency Distribution (n={len(latencies)})')
    ax.legend()
    ax.set_xscale('log')


def plot_latency_timeline(proof_completions: list[ProofEvent], ax):
    """Plot latency over time with color-coded severity."""
    if not proof_completions:
        ax.text(0.5, 0.5, "No proof data", ha='center', va='center')
        return

    timestamps = [p.timestamp for p in proof_completions]
    latencies = [p.round_trip_ms for p in proof_completions]

    # Color by latency bucket
    colors = []
    for lat in latencies:
        if lat < 1000:
            colors.append('green')
        elif lat < 5000:
            colors.append('yellow')
        elif lat < 10000:
            colors.append('orange')
        else:
            colors.append('red')

    ax.scatter(timestamps, latencies, c=colors, alpha=0.6, s=20)
    ax.set_xlabel('Time')
    ax.set_ylabel('Latency (ms)')
    ax.set_title('Proof Latency Over Time')
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%H:%M:%S'))
    ax.tick_params(axis='x', rotation=45)


def plot_throughput_gaps(tx_events: list[TxEvent], ax):
    """Plot transaction throughput with gap highlighting."""
    if not tx_events:
        ax.text(0.5, 0.5, "No TX data", ha='center', va='center')
        return

    timestamps = [t.timestamp for t in tx_events]
    ixs = [t.ixs for t in tx_events]

    # Calculate gaps
    gaps = []
    for i in range(1, len(timestamps)):
        gap = (timestamps[i] - timestamps[i-1]).total_seconds()
        gaps.append((timestamps[i-1], timestamps[i], gap))

    # Plot proofs
    ax.bar(timestamps, ixs, width=0.0001, color='steelblue', alpha=0.8)

    # Highlight large gaps (> 10s)
    for start, end, gap in gaps:
        if gap > 10:
            ax.axvspan(start, end, alpha=0.3, color='red')
            mid = start + (end - start) / 2
            ax.annotate(f'{gap:.0f}s', xy=(mid, max(ixs)*0.9), fontsize=8, ha='center', color='red')

    ax.set_xlabel('Time')
    ax.set_ylabel('Proofs per TX')
    ax.set_title('Transaction Throughput with Gaps (red = >10s gap)')
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%H:%M:%S'))
    ax.tick_params(axis='x', rotation=45)


def plot_pipeline_utilization(proof_requests: list[ProofRequest],
                              proof_completions: list[ProofEvent],
                              ax):
    """Plot proof request vs completion rates to show pipeline depth."""
    if not proof_requests or not proof_completions:
        ax.text(0.5, 0.5, "No pipeline data", ha='center', va='center')
        return

    # Resample to 1-second bins
    req_times = [p.timestamp for p in proof_requests]
    comp_times = [p.timestamp for p in proof_completions]

    all_times = req_times + comp_times
    if not all_times:
        return

    min_t = min(all_times)
    max_t = max(all_times)

    # Create time bins (1 second)
    bins = []
    t = min_t
    while t <= max_t:
        bins.append(t)
        t += timedelta(seconds=1)

    req_counts = np.zeros(len(bins) - 1)
    comp_counts = np.zeros(len(bins) - 1)

    for rt in req_times:
        for i in range(len(bins) - 1):
            if bins[i] <= rt < bins[i + 1]:
                req_counts[i] += 1
                break

    for ct in comp_times:
        for i in range(len(bins) - 1):
            if bins[i] <= ct < bins[i + 1]:
                comp_counts[i] += 1
                break

    bin_centers = [bins[i] + (bins[i+1] - bins[i]) / 2 for i in range(len(bins) - 1)]

    ax.fill_between(bin_centers, req_counts, alpha=0.5, label='Requests', color='blue')
    ax.fill_between(bin_centers, comp_counts, alpha=0.5, label='Completions', color='green')
    ax.plot(bin_centers, np.cumsum(req_counts) - np.cumsum(comp_counts),
            color='red', linewidth=2, label='In-flight (cumulative)')

    ax.set_xlabel('Time')
    ax.set_ylabel('Count per second')
    ax.set_title('Pipeline Utilization (Requests vs Completions)')
    ax.legend()
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%H:%M:%S'))
    ax.tick_params(axis='x', rotation=45)


def plot_bottleneck_timeline(bottlenecks: list[BottleneckEvent], ax):
    """Plot bottleneck events on a timeline."""
    if not bottlenecks:
        ax.text(0.5, 0.5, "No bottleneck events detected", ha='center', va='center')
        return

    event_types = {"indexer_sync": 0, "batch_limit": 1}
    colors = {"indexer_sync": "red", "batch_limit": "orange"}

    for b in bottlenecks:
        y = event_types.get(b.event_type, 2)
        ax.scatter([b.timestamp], [y], c=colors.get(b.event_type, 'gray'), s=50, alpha=0.7)

    ax.set_yticks([0, 1])
    ax.set_yticklabels(['Indexer Sync Wait', 'Batch Limit'])
    ax.set_xlabel('Time')
    ax.set_title(f'Bottleneck Events (n={len(bottlenecks)})')
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%H:%M:%S'))
    ax.tick_params(axis='x', rotation=45)


def plot_tx_type_breakdown(tx_events: list[TxEvent], ax):
    """Pie chart of transaction types."""
    if not tx_events:
        ax.text(0.5, 0.5, "No TX data", ha='center', va='center')
        return

    type_counts = {}
    for t in tx_events:
        type_counts[t.tx_type] = type_counts.get(t.tx_type, 0) + 1

    labels = list(type_counts.keys())
    sizes = list(type_counts.values())

    ax.pie(sizes, labels=labels, autopct='%1.1f%%', startangle=90)
    ax.set_title('Transaction Type Distribution')


def plot_time_breakdown_by_type(proof_completions: list[ProofEvent], ax):
    """Bar chart showing mean round-trip time by proof type."""
    proofs_with_type = [p for p in proof_completions if p.proof_type]

    if not proofs_with_type:
        ax.text(0.5, 0.5, "No timing data by type", ha='center', va='center')
        return

    # Aggregate by type - use round_trip_ms which is the actual end-to-end time
    type_data = {}
    for p in proofs_with_type:
        if p.proof_type not in type_data:
            type_data[p.proof_type] = []
        type_data[p.proof_type].append(p.round_trip_ms)

    types = sorted(type_data.keys())
    means = [np.mean(type_data[t]) for t in types]
    medians = [np.median(type_data[t]) for t in types]
    p95s = [np.percentile(type_data[t], 95) for t in types]
    counts = [len(type_data[t]) for t in types]

    x = np.arange(len(types))
    width = 0.25

    bars1 = ax.bar(x - width, means, width, label='Mean', color='steelblue')
    bars2 = ax.bar(x, medians, width, label='Median', color='green')
    bars3 = ax.bar(x + width, p95s, width, label='p95', color='orange')

    ax.set_ylabel('Round-trip Time (ms)')
    ax.set_title(f'Round-trip Time by Proof Type (n={len(proofs_with_type)})')
    ax.set_xticks(x)
    ax.set_xticklabels([f"{t}\n(n={c})" for t, c in zip(types, counts)])
    ax.legend()

    # Add value labels on bars
    for bars in [bars1, bars2, bars3]:
        for bar in bars:
            height = bar.get_height()
            ax.text(bar.get_x() + bar.get_width()/2., height,
                   f'{height:.0f}',
                   ha='center', va='bottom', fontsize=7)


def plot_latency_timeline_by_type(proof_completions: list[ProofEvent], ax):
    """Plot latency over time with color by proof type."""
    if not proof_completions:
        ax.text(0.5, 0.5, "No proof data", ha='center', va='center')
        return

    # Color map for proof types
    type_colors = {
        'append': 'blue',
        'update': 'red',
        'address_append': 'green',
    }

    for proof_type, color in type_colors.items():
        type_proofs = [p for p in proof_completions if p.proof_type == proof_type]
        if type_proofs:
            timestamps = [p.timestamp for p in type_proofs]
            latencies = [p.round_trip_ms for p in type_proofs]
            ax.scatter(timestamps, latencies, c=color, alpha=0.6, s=20, label=f'{proof_type} (n={len(type_proofs)})')

    # Handle unknown types
    unknown = [p for p in proof_completions if p.proof_type not in type_colors]
    if unknown:
        timestamps = [p.timestamp for p in unknown]
        latencies = [p.round_trip_ms for p in unknown]
        ax.scatter(timestamps, latencies, c='gray', alpha=0.4, s=15, label=f'unknown (n={len(unknown)})')

    ax.set_xlabel('Time')
    ax.set_ylabel('Round-trip Latency (ms)')
    ax.set_title('Latency Over Time by Proof Type')
    ax.legend(loc='upper left', fontsize=8)
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%H:%M:%S'))
    ax.tick_params(axis='x', rotation=45)


def plot_proof_vs_queue_scatter(proof_completions: list[ProofEvent], ax):
    """Scatter plot of proof time vs queue wait time, highlighting cache hits."""
    proofs_with_timing = [p for p in proof_completions if p.proof_ms is not None]

    if not proofs_with_timing:
        ax.text(0.5, 0.5, "No timing data", ha='center', va='center')
        return

    # Separate cache hits (negative queue wait) from fresh proofs
    cached = [p for p in proofs_with_timing if p.queue_wait_ms < 0]
    fresh = [p for p in proofs_with_timing if p.queue_wait_ms >= 0]

    # Plot fresh proofs by type
    type_colors = {
        'append': 'blue',
        'update': 'red',
        'address_append': 'green',
    }

    for proof_type, color in type_colors.items():
        type_proofs = [p for p in fresh if p.proof_type == proof_type]
        if type_proofs:
            proof_times = [p.proof_ms for p in type_proofs]
            queue_times = [p.queue_wait_ms for p in type_proofs]
            ax.scatter(proof_times, queue_times, c=color, alpha=0.6, s=30, label=f'{proof_type} (n={len(type_proofs)})')

    # Unknown types (fresh)
    unknown = [p for p in fresh if p.proof_type not in type_colors]
    if unknown:
        proof_times = [p.proof_ms for p in unknown]
        queue_times = [p.queue_wait_ms for p in unknown]
        ax.scatter(proof_times, queue_times, c='gray', alpha=0.4, s=20, label=f'unknown (n={len(unknown)})')

    # Plot cache hits separately (below zero line)
    if cached:
        proof_times = [p.proof_ms for p in cached]
        queue_times = [p.queue_wait_ms for p in cached]
        ax.scatter(proof_times, queue_times, c='lime', alpha=0.5, s=25, marker='v',
                   label=f'cache hits (n={len(cached)})')

    # Add zero line to show cache hit boundary
    ax.axhline(y=0, color='black', linestyle='-', linewidth=1, alpha=0.5)

    # Add diagonal line where proof == queue (for fresh proofs only)
    if fresh:
        max_val = max(max(p.proof_ms for p in fresh), max(p.queue_wait_ms for p in fresh))
        ax.plot([0, max_val], [0, max_val], 'k--', alpha=0.3)

    ax.set_xlabel('Pure Proof Time (ms)')
    ax.set_ylabel('Queue Wait Time (ms)\n(negative = cache hit)')
    cache_pct = len(cached) / len(proofs_with_timing) * 100 if proofs_with_timing else 0
    ax.set_title(f'Proof vs Queue Wait ({cache_pct:.0f}% cache hits)')
    ax.legend(loc='upper right', fontsize=7)


def print_summary(data: dict):
    """Print summary statistics to console."""
    proof_completions = data["proof_completions"]
    tx_events = data["tx_events"]
    bottlenecks = data["bottlenecks"]

    print("\n" + "="*60)
    print("FORESTER PERFORMANCE SUMMARY")
    print("="*60)

    if proof_completions:
        latencies = [p.round_trip_ms for p in proof_completions]
        print(f"\nProof Latency Statistics (n={len(latencies)}):")
        print(f"  Min:    {min(latencies):,} ms")
        print(f"  Max:    {max(latencies):,} ms")
        print(f"  Mean:   {np.mean(latencies):,.1f} ms")
        print(f"  Median: {np.median(latencies):,.1f} ms")
        print(f"  p95:    {np.percentile(latencies, 95):,.1f} ms")
        print(f"  p99:    {np.percentile(latencies, 99):,.1f} ms")

        # Latency buckets
        print("\n  Distribution:")
        buckets = [(0, 500), (500, 1000), (1000, 2000), (2000, 5000), (5000, 10000), (10000, float('inf'))]
        bucket_names = ["<500ms", "500-1000ms", "1-2s", "2-5s", "5-10s", ">10s"]
        for (lo, hi), name in zip(buckets, bucket_names):
            count = sum(1 for l in latencies if lo <= l < hi)
            pct = count / len(latencies) * 100
            bar = '#' * int(pct / 2)
            print(f"    {name:>12}: {count:4d} ({pct:5.1f}%) {bar}")

        # Time breakdown: proof vs queue wait
        proofs_with_timing = [p for p in proof_completions if p.proof_ms is not None]
        if proofs_with_timing:
            # Separate cache hits (pre-warmed) from fresh proofs
            cached_proofs = [p for p in proofs_with_timing if p.queue_wait_ms < 0]
            fresh_proofs = [p for p in proofs_with_timing if p.queue_wait_ms >= 0]

            cache_hit_rate = len(cached_proofs) / len(proofs_with_timing) * 100
            print(f"\n  Cache Statistics (n={len(proofs_with_timing)} with timing data):")
            print(f"    Cache hits (pre-warmed): {len(cached_proofs):,} ({cache_hit_rate:.1f}%)")
            print(f"    Fresh proofs:            {len(fresh_proofs):,} ({100-cache_hit_rate:.1f}%)")

            if cached_proofs:
                cached_latencies = [p.round_trip_ms for p in cached_proofs]
                print(f"    Cache hit latency:       {np.mean(cached_latencies):.0f}ms mean, {np.median(cached_latencies):.0f}ms median")

            if fresh_proofs:
                proof_times = [p.proof_ms for p in fresh_proofs]
                queue_waits = [p.queue_wait_ms for p in fresh_proofs]

                print(f"\n  Time Breakdown (fresh proofs only, n={len(fresh_proofs)}):")
                print(f"    Pure Proof Time:")
                print(f"      Min:    {min(proof_times):,} ms")
                print(f"      Max:    {max(proof_times):,} ms")
                print(f"      Mean:   {np.mean(proof_times):,.1f} ms")
                print(f"      Median: {np.median(proof_times):,.1f} ms")
                print(f"      p95:    {np.percentile(proof_times, 95):,.1f} ms")

                print(f"    Queue Wait Time (round_trip - proof):")
                print(f"      Min:    {min(queue_waits):,} ms")
                print(f"      Max:    {max(queue_waits):,} ms")
                print(f"      Mean:   {np.mean(queue_waits):,.1f} ms")
                print(f"      Median: {np.median(queue_waits):,.1f} ms")
                print(f"      p95:    {np.percentile(queue_waits, 95):,.1f} ms")

                # Percentage breakdown
                total_time = sum(p.round_trip_ms for p in fresh_proofs)
                total_proof = sum(proof_times)
                total_queue = sum(queue_waits)
                print(f"\n    Time Distribution (fresh proofs):")
                print(f"      Proof generation: {total_proof/total_time*100:5.1f}% of total time")
                print(f"      Queue wait:       {total_queue/total_time*100:5.1f}% of total time")

        # Latency by proof type
        type_latencies = {}
        for p in proof_completions:
            if p.proof_type:
                if p.proof_type not in type_latencies:
                    type_latencies[p.proof_type] = {'round_trip': [], 'proof': [], 'queue': []}
                type_latencies[p.proof_type]['round_trip'].append(p.round_trip_ms)
                if p.proof_ms is not None:
                    type_latencies[p.proof_type]['proof'].append(p.proof_ms)
                    type_latencies[p.proof_type]['queue'].append(p.queue_wait_ms)

        if type_latencies:
            print("\n  Latency by Proof Type (round_trip):")
            print(f"    {'Type':<18} {'Count':>6} {'Min':>8} {'p50':>8} {'Mean':>8} {'p95':>8} {'Max':>8}")
            print("    " + "-"*66)
            for proof_type in sorted(type_latencies.keys()):
                lats = type_latencies[proof_type]['round_trip']
                if lats:
                    print(f"    {proof_type:<18} {len(lats):>6} {min(lats):>7}ms {np.percentile(lats, 50):>7.0f}ms {np.mean(lats):>7.0f}ms {np.percentile(lats, 95):>7.0f}ms {max(lats):>7}ms")

            # Show proof time breakdown by type
            has_proof_timing = any(type_latencies[t]['proof'] for t in type_latencies)
            if has_proof_timing:
                print("\n  Pure Proof Time by Type:")
                print(f"    {'Type':<18} {'Count':>6} {'Min':>8} {'p50':>8} {'Mean':>8} {'p95':>8} {'Max':>8}")
                print("    " + "-"*66)
                for proof_type in sorted(type_latencies.keys()):
                    lats = type_latencies[proof_type]['proof']
                    if lats:
                        print(f"    {proof_type:<18} {len(lats):>6} {min(lats):>7}ms {np.percentile(lats, 50):>7.0f}ms {np.mean(lats):>7.0f}ms {np.percentile(lats, 95):>7.0f}ms {max(lats):>7}ms")

                print("\n  Queue Wait Time by Type:")
                print(f"    {'Type':<18} {'Count':>6} {'Min':>8} {'p50':>8} {'Mean':>8} {'p95':>8} {'Max':>8}")
                print("    " + "-"*66)
                for proof_type in sorted(type_latencies.keys()):
                    lats = type_latencies[proof_type]['queue']
                    if lats:
                        print(f"    {proof_type:<18} {len(lats):>6} {min(lats):>7}ms {np.percentile(lats, 50):>7.0f}ms {np.mean(lats):>7.0f}ms {np.percentile(lats, 95):>7.0f}ms {max(lats):>7}ms")

    if tx_events:
        total_proofs = sum(t.ixs for t in tx_events)
        duration = (tx_events[-1].timestamp - tx_events[0].timestamp).total_seconds()

        print(f"\nTransaction Statistics:")
        print(f"  Total TXs:     {len(tx_events)}")
        print(f"  Total Proofs:  {total_proofs}")
        print(f"  Duration:      {duration:.1f}s ({duration/60:.1f} min)")
        print(f"  Throughput:    {total_proofs/duration*60:.1f} proofs/min" if duration > 0 else "  Throughput: N/A")

        # Gap analysis
        gaps = []
        for i in range(1, len(tx_events)):
            gap = (tx_events[i].timestamp - tx_events[i-1].timestamp).total_seconds()
            gaps.append(gap)

        if gaps:
            large_gaps = [(g, i) for i, g in enumerate(gaps) if g > 10]
            print(f"\n  Inter-TX Gaps:")
            print(f"    Mean:  {np.mean(gaps):.2f}s")
            print(f"    Max:   {max(gaps):.1f}s")
            print(f"    Gaps >10s: {len(large_gaps)}")
            total_gap_time = sum(g for g, _ in large_gaps)
            print(f"    Time lost in gaps: {total_gap_time:.1f}s ({total_gap_time/duration*100:.1f}%)")

    if bottlenecks:
        print(f"\nBottleneck Events:")
        by_type = {}
        for b in bottlenecks:
            by_type[b.event_type] = by_type.get(b.event_type, 0) + 1
        for t, c in by_type.items():
            print(f"  {t}: {c}")

    print("\n" + "="*60)


def main():
    parser = argparse.ArgumentParser(
        description="Enhanced Forester performance analysis and visualization."
    )
    parser.add_argument("logfile", type=Path, help="Path to log file")
    parser.add_argument("--no-show", action="store_true", help="Don't display plots interactively")
    parser.add_argument("--out", type=Path, default=None, help="Base path to save PNGs")
    parser.add_argument("--summary-only", action="store_true", help="Only print summary, no plots")

    args = parser.parse_args()

    print(f"Parsing {args.logfile}...")
    data = parse_log(args.logfile)

    print_summary(data)

    if args.summary_only:
        return

    # Create figure with subplots (3x3 grid for more detailed analysis)
    fig = plt.figure(figsize=(18, 14))

    ax1 = fig.add_subplot(3, 3, 1)
    plot_latency_distribution(data["proof_completions"], ax1)

    ax2 = fig.add_subplot(3, 3, 2)
    plot_latency_timeline_by_type(data["proof_completions"], ax2)

    ax3 = fig.add_subplot(3, 3, 3)
    plot_time_breakdown_by_type(data["proof_completions"], ax3)

    ax4 = fig.add_subplot(3, 3, 4)
    plot_proof_vs_queue_scatter(data["proof_completions"], ax4)

    ax5 = fig.add_subplot(3, 3, 5)
    plot_throughput_gaps(data["tx_events"], ax5)

    ax6 = fig.add_subplot(3, 3, 6)
    plot_tx_type_breakdown(data["tx_events"], ax6)

    ax7 = fig.add_subplot(3, 3, 7)
    plot_pipeline_utilization(data["proof_requests"], data["proof_completions"], ax7)

    ax8 = fig.add_subplot(3, 3, 8)
    plot_bottleneck_timeline(data["bottlenecks"], ax8)

    ax9 = fig.add_subplot(3, 3, 9)
    plot_latency_timeline(data["proof_completions"], ax9)

    plt.tight_layout()

    if args.out:
        out_path = args.out.with_suffix('.png')
        plt.savefig(out_path, dpi=150)
        print(f"\nSaved plot to: {out_path}")

    if not args.no_show:
        plt.show()


if __name__ == "__main__":
    main()
