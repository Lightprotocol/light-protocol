import type { AggregateQueueStats } from "@/types/forester";
import { formatNumber } from "@/lib/utils";

interface QueuePressureChartProps {
  stats: AggregateQueueStats;
}

const entries: {
  key: keyof AggregateQueueStats;
  label: string;
  color: string;
}[] = [
  {
    key: "state_v1_total_pending",
    label: "State V1",
    color: "bg-purple-500",
  },
  {
    key: "state_v2_input_pending",
    label: "State V2 Input",
    color: "bg-indigo-500",
  },
  {
    key: "state_v2_output_pending",
    label: "State V2 Output",
    color: "bg-indigo-300",
  },
  {
    key: "address_v1_total_pending",
    label: "Addr V1",
    color: "bg-teal-500",
  },
  {
    key: "address_v2_input_pending",
    label: "Addr V2 Input",
    color: "bg-cyan-500",
  },
];

export function QueuePressureChart({ stats }: QueuePressureChartProps) {
  const maxVal = Math.max(
    ...entries.map((e) => stats[e.key]),
    1
  );

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <h3 className="text-sm font-semibold text-gray-900 mb-3">
        Queue Pressure
      </h3>
      <div className="space-y-2">
        {entries.map((e) => {
          const val = stats[e.key];
          const pct = (val / maxVal) * 100;
          return (
            <div key={e.key}>
              <div className="flex justify-between text-xs text-gray-600 mb-0.5">
                <span>{e.label}</span>
                <span className="font-mono">{formatNumber(val)}</span>
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
