"use client";

import { useMetrics } from "@/hooks/useMetrics";
import { ErrorState } from "@/components/ErrorState";
import { MetricsPanel } from "@/components/MetricsPanel";

export default function MetricsPage() {
  const { data: metrics, error, isLoading } = useMetrics();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400 text-sm">Loading metrics...</div>
      </div>
    );
  }

  if (error || !metrics) {
    return <ErrorState error={error} fallbackMessage="Failed to load metrics" />;
  }

  const isEmpty =
    Object.keys(metrics.transactions_processed_total).length === 0 &&
    Object.keys(metrics.forester_balances).length === 0;

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-bold">Metrics</h2>
      {isEmpty ? (
        <div className="bg-white rounded-lg border border-gray-200 p-8 text-center">
          <p className="text-gray-500 text-sm">
            No metrics data available.
          </p>
          <p className="text-gray-400 text-xs mt-2">
            Configure --prometheus-url to query aggregated metrics, or connect
            to a running forester instance.
          </p>
        </div>
      ) : (
        <MetricsPanel metrics={metrics} />
      )}
    </div>
  );
}
