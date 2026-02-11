"use client";

import { useForesterStatus } from "@/hooks/useForesterStatus";
import { EpochCard } from "@/components/EpochCard";
import { ErrorState } from "@/components/ErrorState";
import { ForesterList } from "@/components/ForesterList";
import { QueuePressureChart } from "@/components/QueuePressureChart";
import { formatNumber } from "@/lib/utils";

export default function OverviewPage() {
  const { data: status, error, isLoading } = useForesterStatus();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400 text-sm">Loading forester status...</div>
      </div>
    );
  }

  if (error || !status) {
    return <ErrorState error={error} fallbackMessage="Failed to load forester status" />;
  }

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

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold">Overview</h2>
        <span className="text-xs text-gray-400 font-mono">
          Slot {formatNumber(status.slot)}
        </span>
      </div>

      {warnings.map((w, i) => (
        <div
          key={i}
          className="bg-amber-50 border border-amber-200 rounded-lg px-4 py-3 text-sm text-amber-800"
        >
          {w}
        </div>
      ))}

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <StatCard label="Total Trees" value={status.total_trees} />
        <StatCard label="Active Trees" value={status.active_trees} />
        <StatCard label="Rolled Over" value={status.rolled_over_trees} />
        <StatCard
          label="Total Pending"
          value={status.total_pending_items}
          highlight={status.total_pending_items > 0}
        />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <EpochCard status={status} />
        <QueuePressureChart stats={status.aggregate_queue_stats} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <ForesterList
          title="Active Epoch Foresters"
          foresters={status.active_epoch_foresters}
        />
        <ForesterList
          title="Registration Epoch Foresters"
          foresters={status.registration_epoch_foresters}
        />
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  highlight,
}: {
  label: string;
  value: number;
  highlight?: boolean;
}) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <div className="text-xs text-gray-500">{label}</div>
      <div
        className={`text-2xl font-semibold mt-1 ${highlight ? "text-amber-600" : "text-gray-900"}`}
      >
        {formatNumber(value)}
      </div>
    </div>
  );
}
