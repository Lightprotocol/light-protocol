import type { MetricsResponse } from "@/types/forester";
import { formatNumber } from "@/lib/utils";

interface MetricsPanelProps {
  metrics: MetricsResponse;
}

export function MetricsPanel({ metrics }: MetricsPanelProps) {
  const rates = Object.entries(metrics.transaction_rate);

  if (rates.length === 0) return null;

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        {rates.map(([epoch, rate]) => (
          <div key={epoch} className="bg-gray-50 rounded p-3">
            <div className="text-xs text-gray-500">Epoch {epoch}</div>
            <div className="text-sm font-mono font-medium">
              {rate.toFixed(2)} tx/s
            </div>
            <div className="text-xs text-gray-400 font-mono">
              {formatNumber(metrics.transactions_processed_total[epoch] ?? 0)} total
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
