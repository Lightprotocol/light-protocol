import type { V2QueueInfo } from "@/types/forester";
import { StatusBadge } from "./StatusBadge";
import { ProgressBar } from "./ProgressBar";
import {
  formatNumber,
  batchStateLabel,
  batchStateColor,
} from "@/lib/utils";

interface TreeBatchDetailProps {
  info: V2QueueInfo;
}

export function TreeBatchDetail({ info }: TreeBatchDetailProps) {
  return (
    <div className="bg-gray-50 border-t border-gray-100 px-6 py-4">
      <div className="grid grid-cols-3 gap-4 mb-4 text-xs">
        <div>
          <span className="text-gray-500">ZKP Batch Size</span>
          <p className="font-mono font-medium">{formatNumber(info.zkp_batch_size)}</p>
        </div>
        <div>
          <span className="text-gray-500">Input Pending Batches</span>
          <p className="font-mono font-medium">{info.input_pending_batches}</p>
        </div>
        <div>
          <span className="text-gray-500">Output Pending Batches</span>
          <p className="font-mono font-medium">{info.output_pending_batches}</p>
        </div>
      </div>

      {info.batches.length > 0 && (
        <table className="w-full text-xs">
          <thead>
            <tr className="text-left text-gray-500 border-b border-gray-200">
              <th className="py-1.5 pr-3 font-medium">Batch</th>
              <th className="py-1.5 pr-3 font-medium">State</th>
              <th className="py-1.5 pr-3 font-medium">Inserted</th>
              <th className="py-1.5 pr-3 font-medium">Index</th>
              <th className="py-1.5 pr-3 font-medium">Pending</th>
              <th className="py-1.5 font-medium">ZKP Fill</th>
            </tr>
          </thead>
          <tbody>
            {info.batches.map((batch) => (
              <tr
                key={batch.batch_index}
                className="border-b border-gray-100"
              >
                <td className="py-1.5 pr-3 font-mono">
                  #{batch.batch_index}
                </td>
                <td className="py-1.5 pr-3">
                  <StatusBadge
                    label={batchStateLabel(batch.batch_state)}
                    color={batchStateColor(batch.batch_state)}
                  />
                </td>
                <td className="py-1.5 pr-3 font-mono">
                  {formatNumber(batch.num_inserted)}
                </td>
                <td className="py-1.5 pr-3 font-mono">
                  {formatNumber(batch.current_index)}
                </td>
                <td className="py-1.5 pr-3 font-mono">
                  {formatNumber(batch.pending)}
                </td>
                <td className="py-1.5">
                  <div className="flex items-center gap-2">
                    <ProgressBar
                      value={batch.items_in_current_zkp_batch}
                      max={info.zkp_batch_size}
                      className="w-20"
                      barColor="bg-violet-500"
                    />
                    <span className="font-mono text-gray-600">
                      {batch.items_in_current_zkp_batch}/{info.zkp_batch_size}
                    </span>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
