import { useEffect, useRef, useCallback } from "react";
import type { ForesterInfo } from "@/types/forester";

interface BalanceSnapshot {
  timestamp: number; // unix ms
  balances: Record<string, number>; // authority -> balance_sol
}

const STORAGE_KEY = "forester_balance_history";
const MAX_HISTORY_HOURS = 24;
const SNAPSHOT_INTERVAL_MS = 60_000; // store at most once per minute

function loadHistory(): BalanceSnapshot[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const data = JSON.parse(raw) as BalanceSnapshot[];
    const cutoff = Date.now() - MAX_HISTORY_HOURS * 3600_000;
    return data.filter((s) => s.timestamp > cutoff);
  } catch {
    return [];
  }
}

function saveHistory(history: BalanceSnapshot[]) {
  try {
    const cutoff = Date.now() - MAX_HISTORY_HOURS * 3600_000;
    const trimmed = history.filter((s) => s.timestamp > cutoff);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch {
    // storage full or unavailable
  }
}

export interface BalanceTrend {
  current: number;
  hourlyRate: number; // SOL per hour (negative = burning)
  hoursTracked: number;
  oldest: number;
}

export function useBalanceHistory(foresters: ForesterInfo[]) {
  const lastSnapshot = useRef(0);

  // Record a snapshot if enough time has passed
  useEffect(() => {
    if (foresters.length === 0) return;
    const now = Date.now();
    if (now - lastSnapshot.current < SNAPSHOT_INTERVAL_MS) return;
    lastSnapshot.current = now;

    const balances: Record<string, number> = {};
    for (const f of foresters) {
      if (f.balance_sol != null) {
        balances[f.authority] = f.balance_sol;
      }
    }
    if (Object.keys(balances).length === 0) return;

    const history = loadHistory();
    history.push({ timestamp: now, balances });
    saveHistory(history);
  }, [foresters]);

  const getTrend = useCallback(
    (authority: string, hours: number = 6): BalanceTrend | null => {
      const forester = foresters.find((f) => f.authority === authority);
      if (!forester || forester.balance_sol == null) return null;

      const history = loadHistory();
      const cutoff = Date.now() - hours * 3600_000;
      const relevant = history
        .filter((s) => s.timestamp > cutoff && s.balances[authority] != null)
        .sort((a, b) => a.timestamp - b.timestamp);

      if (relevant.length < 2) return null;

      const oldest = relevant[0];
      const newest = relevant[relevant.length - 1];
      const timeDiffHours =
        (newest.timestamp - oldest.timestamp) / 3600_000;
      if (timeDiffHours < 0.05) return null; // less than 3 minutes

      const balanceDiff =
        newest.balances[authority] - oldest.balances[authority];
      const hourlyRate = balanceDiff / timeDiffHours;

      return {
        current: forester.balance_sol,
        hourlyRate,
        hoursTracked: timeDiffHours,
        oldest: oldest.balances[authority],
      };
    },
    [foresters]
  );

  return { getTrend };
}
