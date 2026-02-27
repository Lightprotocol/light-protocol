"use client";

import { useState, useEffect, useRef } from "react";
import { useForesterStatus } from "@/hooks/useForesterStatus";
import { useMetrics } from "@/hooks/useMetrics";
import { useCompressible } from "@/hooks/useCompressible";
import { usePhotonStats } from "@/hooks/usePhotonStats";
import { useBalanceHistory } from "@/hooks/useBalanceHistory";
import { ErrorState } from "@/components/ErrorState";
import { EpochCard } from "@/components/EpochCard";
import { QueuePressureChart } from "@/components/QueuePressureChart";
import { ForesterList } from "@/components/ForesterList";
import { TreeTable } from "@/components/TreeTable";
import { CompressiblePanel } from "@/components/CompressiblePanel";
import { PhotonStatsPanel } from "@/components/PhotonStatsPanel";
import { MetricsPanel } from "@/components/MetricsPanel";
import { formatNumber } from "@/lib/utils";

export default function Dashboard() {
  const { data: status, error: statusError, isLoading: statusLoading } = useForesterStatus();
  const { data: metrics } = useMetrics();
  const { data: compressible } = useCompressible();
  const { data: photonStats } = usePhotonStats();

  // Track balance history for all foresters
  const allForesters = [
    ...(status?.active_epoch_foresters ?? []),
    ...(status?.registration_epoch_foresters ?? []),
  ];
  const { getTrend } = useBalanceHistory(allForesters);

  // Auto-refresh indicator
  const [lastUpdated, setLastUpdated] = useState<number>(Date.now());
  const [secondsAgo, setSecondsAgo] = useState(0);
  const prevSlot = useRef<number | null>(null);

  useEffect(() => {
    if (status && status.slot !== prevSlot.current) {
      prevSlot.current = status.slot;
      setLastUpdated(Date.now());
    }
  }, [status]);

  useEffect(() => {
    const interval = setInterval(() => {
      setSecondsAgo(Math.floor((Date.now() - lastUpdated) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, [lastUpdated]);

  if (statusLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400 text-sm">Loading...</div>
      </div>
    );
  }

  if (statusError || !status) {
    return <ErrorState error={statusError} fallbackMessage="Failed to load forester status" />;
  }

  // Alerts
  const warnings: string[] = [];
  if (status.active_epoch_foresters.length === 0) {
    warnings.push("No foresters registered for the active epoch");
  }
  if (
    status.slots_until_next_registration < 1000 &&
    status.registration_epoch_foresters.length === 0
  ) {
    warnings.push("Registration closing soon with no foresters registered");
  }
  // Check for foresters not re-registered
  const notReRegistered = status.active_epoch_foresters.filter(
    (f) => !status.registration_epoch_foresters.some((r) => r.authority === f.authority)
  );
  if (notReRegistered.length > 0 && status.active_epoch_progress_percentage > 50) {
    warnings.push(
      `${notReRegistered.length} active forester${notReRegistered.length !== 1 ? "s" : ""} not registered for next epoch`
    );
  }

  const hasMetrics = metrics &&
    Object.keys(metrics.transaction_rate).length > 0;

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold tracking-tight">Forester</h1>
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-400 font-mono">
            Slot {formatNumber(status.slot)}
          </span>
          <RefreshIndicator secondsAgo={secondsAgo} />
        </div>
      </div>

      {/* Warnings */}
      {warnings.map((w, i) => (
        <div
          key={i}
          className="bg-amber-50 border border-amber-200 rounded-lg px-4 py-3 text-sm text-amber-800"
        >
          {w}
        </div>
      ))}

      {/* Stats row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <StatCard
          label="Trees"
          value={status.active_trees}
          sub={status.rolled_over_trees > 0 ? `${status.rolled_over_trees} rolled over` : undefined}
        />
        <StatCard
          label="Pending Batches"
          value={status.total_pending_batches}
          highlight={status.total_pending_batches > 0}
          sub={`${formatNumber(status.total_pending_items)} items`}
        />
        {compressible && compressible.total_tracked != null && (
          <StatCard
            label="Compressible"
            value={compressible.total_tracked}
            sub={compressible.total_ready != null && compressible.total_ready > 0
              ? `${formatNumber(compressible.total_ready)} ready`
              : undefined}
          />
        )}
        {hasMetrics && (
          <StatCard
            label="Transactions"
            value={Object.values(metrics!.transactions_processed_total).reduce((a, b) => a + b, 0)}
          />
        )}
      </div>

      {/* Epoch + Queue Pressure */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <EpochCard status={status} />
        <QueuePressureChart stats={status.aggregate_queue_stats} />
      </div>

      {/* Foresters */}
      <ForesterList
        active={status.active_epoch_foresters}
        registering={status.registration_epoch_foresters}
        activeEpoch={status.current_active_epoch}
        registrationEpoch={status.current_registration_epoch}
        getTrend={getTrend}
      />

      {/* Compressible */}
      {compressible && compressible.enabled && (
        <Section title="Compressible">
          <CompressiblePanel data={compressible} />
        </Section>
      )}

      {/* Photon Stats */}
      {photonStats && !photonStats.error && (
        <Section title="Compression Stats">
          <PhotonStatsPanel data={photonStats} />
        </Section>
      )}

      {/* Trees */}
      <Section title="Trees">
        <TreeTable
          trees={status.trees}
          foresters={status.active_epoch_foresters}
          currentLightSlot={status.current_light_slot}
        />
      </Section>

      {/* Metrics */}
      {hasMetrics && (
        <Section title="Metrics">
          <MetricsPanel metrics={metrics!} />
        </Section>
      )}
    </div>
  );
}

function RefreshIndicator({ secondsAgo }: { secondsAgo: number }) {
  const stale = secondsAgo > 15;
  return (
    <div className="flex items-center gap-1.5">
      <div
        className={`w-1.5 h-1.5 rounded-full ${
          stale ? "bg-amber-400" : "bg-green-400 animate-pulse"
        }`}
      />
      <span className={`text-[10px] ${stale ? "text-amber-500" : "text-gray-400"}`}>
        {secondsAgo < 2 ? "live" : `${secondsAgo}s ago`}
      </span>
    </div>
  );
}

function StatCard({
  label,
  value,
  highlight,
  sub,
}: {
  label: string;
  value: number;
  highlight?: boolean;
  sub?: string;
}) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <div className="text-xs text-gray-500">{label}</div>
      <div
        className={`text-2xl font-semibold mt-1 ${highlight ? "text-amber-600" : "text-gray-900"}`}
      >
        {formatNumber(value)}
      </div>
      {sub && <div className="text-xs text-gray-400 mt-0.5">{sub}</div>}
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h2 className="text-lg font-semibold mb-3">{title}</h2>
      {children}
    </div>
  );
}
