#!/usr/bin/env python3
import argparse
from pathlib import Path

import pandas as pd
import matplotlib.pyplot as plt
import re


# Match ISO8601 timestamp like 2025-12-04T12:21:58.990364Z anywhere in the line
TS_RE = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z")


def parse_log(path: Path) -> pd.DataFrame:
    """
    Parse a Forester tx_sender log and return a DataFrame with:
      index: timestamp (UTC)
      column: proofs  (ixs value)

    Expected line example:
      2025-12-04T12:21:58.990364Z  INFO forester::processor::v2::tx_sender: \
        tx sent: ... type=AddressAppend ixs=4 root=[...] seq=0..3 epoch=26
    """
    rows: list[tuple[str, int]] = []

    with path.open("r", encoding="utf-8") as f:
        for line in f:
            if "ixs=" not in line:
                continue

            # --- timestamp via regex (robust against ANSI codes, prefixes, etc.) ---
            m = TS_RE.search(line)
            if not m:
                continue
            ts = m.group(0)

            # --- parse integer after "ixs=" ---
            ix_pos = line.find("ixs=")
            if ix_pos == -1:
                continue
            rest = line[ix_pos + len("ixs=") :]  # e.g. "4 root=[...] ..."
            num_str = ""
            for ch in rest:
                if ch.isdigit():
                    num_str += ch
                else:
                    break
            if not num_str:
                continue

            proofs = int(num_str)
            rows.append((ts, proofs))

    if not rows:
        raise RuntimeError(
            "No lines with 'ixs=' could be parsed. "
            "Make sure the log is from forester::processor::v2::tx_sender."
        )

    df = pd.DataFrame(rows, columns=["timestamp", "proofs"])
    df["timestamp"] = pd.to_datetime(df["timestamp"], utc=True, errors="raise")
    df = df.sort_values("timestamp").set_index("timestamp")

    return df


def make_plots(
    df: pd.DataFrame,
    rolling_window: int = 3,
    show: bool = True,
    out: Path | None = None,
):
    """
    Create two plots:
      1) proofs per tx over time
      2) proofs per minute + rolling average

    rolling_window is in minutes.
    """
    # --- per-tx plot ---------------------------------------------------------
    plt.figure(figsize=(10, 4))
    plt.plot(df.index, df["proofs"], marker="o")
    plt.title("Proofs per Transaction Over Time")
    plt.xlabel("Time")
    plt.ylabel("ixs (proofs per tx)")
    plt.xticks(rotation=45)
    plt.tight_layout()

    if out is not None:
        per_tx_path = out.with_suffix(".per_tx.png")
        plt.savefig(per_tx_path, dpi=150)
        print(f"Saved per-tx plot to: {per_tx_path}")
    if show:
        plt.show()

    # --- per-minute aggregation + rolling average ---------------------------
    # Sum proofs per minute
    per_min = df["proofs"].resample("1T").sum()

    # Rolling average over N minutes (trailing window)
    rolling = per_min.rolling(window=rolling_window, min_periods=1).mean()

    duration_min = (df.index.max() - df.index.min()).total_seconds() / 60.0
    total_proofs = df["proofs"].sum()
    avg_per_min = total_proofs / duration_min if duration_min > 0 else float("nan")

    print(f"Total proofs: {total_proofs}")
    print(f"Duration: {duration_min:.2f} minutes")
    print(f"Average throughput: {avg_per_min:.2f} proofs/min")
    print(f"Rolling window: {rolling_window} minutes")

    plt.figure(figsize=(10, 4))
    plt.plot(per_min.index, per_min.values, marker="o", label="Proofs per minute")
    plt.plot(
        rolling.index,
        rolling.values,
        linestyle="--",
        marker="x",
        label=f"Rolling avg ({rolling_window}-min)",
    )
    plt.title("Proof Throughput and Rolling Average")
    plt.xlabel("Time")
    plt.ylabel("Proofs per minute")
    plt.xticks(rotation=45)
    plt.legend()
    plt.tight_layout()

    if out is not None:
        per_min_path = out.with_suffix(".per_min.png")
        plt.savefig(per_min_path, dpi=150)
        print(f"Saved per-minute plot to: {per_min_path}")
    if show:
        plt.show()


def main():
    parser = argparse.ArgumentParser(
        description="Parse Forester tx_sender logs and plot proof throughput."
    )
    parser.add_argument("logfile", type=Path, help="Path to log file, e.g. tx.log")
    parser.add_argument(
        "--rolling-window",
        type=int,
        default=3,
        help="Rolling average window in minutes (default: 3).",
    )
    parser.add_argument(
        "--no-show",
        action="store_true",
        help="Do not display plots interactively, only save to files (if --out is given).",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Base path to save PNGs (without extension), e.g. ./proofs",
    )

    args = parser.parse_args()

    df = parse_log(args.logfile)
    # Debug: uncomment if you want to quickly see parsed data
    # print(df.head(), df.tail(), df.shape)

    make_plots(
        df,
        rolling_window=args.rolling_window,
        show=not args.no_show,
        out=args.out,
    )


if __name__ == "__main__":
    main()

