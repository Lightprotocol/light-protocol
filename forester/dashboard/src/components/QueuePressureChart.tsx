import type { AggregateQueueStats } from "@/types/forester";
import { formatNumber } from "@/lib/utils";

interface QueuePressureChartProps {
  stats: AggregateQueueStats;
}

const entries: {
  batchKey: keyof AggregateQueueStats;
  itemKey: keyof AggregateQueueStats;
  label: string;
  color: string;
}[] = [
  {
    batchKey: "state_v2_input_pending_batches",
    itemKey: "state_v2_input_pending",
    label: "State V2 Input",
    color: "bg-indigo-500",
  },
  {
    batchKey: "state_v2_output_pending_batches",
    itemKey: "state_v2_output_pending",
    label: "State V2 Output",
    color: "bg-indigo-300",
  },
  {
    batchKey: "address_v2_input_pending_batches",
    itemKey: "address_v2_input_pending",
    label: "Addr V2",
    color: "bg-cyan-500",
  },
];

const v1Entries: {
  key: keyof AggregateQueueStats;
  label: string;
  color: string;
}[] = [
  { key: "state_v1_total_pending", label: "State V1", color: "bg-purple-500" },
  { key: "address_v1_total_pending", label: "Addr V1", color: "bg-teal-500" },
];

export function QueuePressureChart({ stats }: QueuePressureChartProps) {
  const allBatches = entries.map((e) => stats[e.batchKey]);
  const allV1 = v1Entries.map((e) => stats[e.key]);
  const maxVal = Math.max(...allBatches, ...allV1, 1);

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <h3 className="text-sm font-semibold text-gray-900 mb-3">
        Queue Pressure
      </h3>
      <div className="space-y-2">
        {entries.map((e) => {
          const batches = stats[e.batchKey];
          const items = stats[e.itemKey];
          const pct = (batches / maxVal) * 100;
          return (
            <div key={e.batchKey}>
              <div className="flex justify-between text-xs text-gray-600 mb-0.5">
                <span>{e.label}</span>
                <span className="font-mono">
                  {formatNumber(batches)}{batches !== 1 ? " batches" : " batch"}
                  <span className="text-gray-400 ml-1">({formatNumber(items)} items)</span>
                </span>
              </div>
              <div className="w-full bg-gray-100 rounded h-1.5">
                <div
                  className={`h-full rounded ${e.color}`}
                  style={{ width: `${pct}%` }}
                />
              </div>
            </div>
          );
        })}
        {v1Entries.map((e) => {
          const val = stats[e.key];
          if (val === 0) return null;
          const pct = (val / maxVal) * 100;
          return (
            <div key={e.key}>
              <div className="flex justify-between text-xs text-gray-600 mb-0.5">
                <span>{e.label}</span>
                <span className="font-mono">{formatNumber(val)} items</span>
              </div>
              <div className="w-full bg-gray-100 rounded h-1.5">
                <div
                  className={`h-full rounded ${e.color}`}
                  style={{ width: `${pct}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
