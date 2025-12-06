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
    pure_proof_ms: Optional[float] = None  # Time from prover_client (actual prover wait)


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
                if m:
                    job_id = job_m.group(1) if job_m else None
                    proof_type = job_types.get(job_id) if job_id else None
                    proof_completions.append(ProofEvent(
                        timestamp=ts,
                        round_trip_ms=int(m.group(1)),
                        job_id=job_id,
                        proof_type=proof_type
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

    # Histogram
    bins = [0, 500, 1000, 2000, 5000, 10000, 20000, max(latencies) + 1000]
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

        # Latency by proof type
        type_latencies = {}
        for p in proof_completions:
            if p.proof_type:
                if p.proof_type not in type_latencies:
                    type_latencies[p.proof_type] = []
                type_latencies[p.proof_type].append(p.round_trip_ms)

        if type_latencies:
            print("\n  Latency by Proof Type:")
            print(f"    {'Type':<18} {'Count':>6} {'Min':>8} {'p50':>8} {'Mean':>8} {'p95':>8} {'Max':>8}")
            print("    " + "-"*66)
            for proof_type in sorted(type_latencies.keys()):
                lats = type_latencies[proof_type]
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

    # Create figure with subplots
    fig = plt.figure(figsize=(16, 12))

    ax1 = fig.add_subplot(2, 3, 1)
    plot_latency_distribution(data["proof_completions"], ax1)

    ax2 = fig.add_subplot(2, 3, 2)
    plot_latency_timeline(data["proof_completions"], ax2)

    ax3 = fig.add_subplot(2, 3, 3)
    plot_tx_type_breakdown(data["tx_events"], ax3)

    ax4 = fig.add_subplot(2, 3, 4)
    plot_throughput_gaps(data["tx_events"], ax4)

    ax5 = fig.add_subplot(2, 3, 5)
    plot_pipeline_utilization(data["proof_requests"], data["proof_completions"], ax5)

    ax6 = fig.add_subplot(2, 3, 6)
    plot_bottleneck_timeline(data["bottlenecks"], ax6)

    plt.tight_layout()

    if args.out:
        out_path = args.out.with_suffix('.png')
        plt.savefig(out_path, dpi=150)
        print(f"\nSaved plot to: {out_path}")

    if not args.no_show:
        plt.show()


if __name__ == "__main__":
    main()
