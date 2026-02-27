import type { CompressibleResponse } from "@/types/forester";
import { formatAgeFromUnixSeconds, formatNumber, formatSlotCountdown } from "@/lib/utils";

interface CompressiblePanelProps {
  data: CompressibleResponse;
}

export function CompressiblePanel({ data }: CompressiblePanelProps) {
  const rows = [
    { label: "Token", stats: data.ctoken, fallback: data.ctoken_count },
    { label: "Mint", stats: data.mint, fallback: data.mint_count },
    { label: "PDA", stats: data.pda, fallback: data.pda_count },
  ];

  const hasReadiness = rows.some((r) => r.stats?.ready != null);
  const hasCompressed = rows.some((r) => r.stats?.compressed != null);

  if (data.error) {
    return (
      <div className="bg-white rounded-lg border border-red-200 p-4">
        <p className="text-sm text-red-700">{data.error}</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="bg-white rounded-lg border border-gray-200 overflow-hidden">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-200 text-left text-gray-500">
              <th className="py-2 px-4 font-medium">Type</th>
              {hasCompressed && (
                <th className="py-2 px-4 font-medium text-right">Compressed</th>
              )}
              <th className="py-2 px-4 font-medium text-right">Pending</th>
              {hasReadiness && (
                <>
                  <th className="py-2 px-4 font-medium text-right">Compressible</th>
                  <th className="py-2 px-4 font-medium text-right">Cooling down</th>
                  <th className="py-2 px-4 font-medium text-right">Next ready</th>
                </>
              )}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const tracked = row.stats?.tracked ?? row.fallback;
              const compressed = row.stats?.compressed;
              if ((tracked == null || tracked === 0) && (compressed == null || compressed === 0)) return null;
              return (
                <tr key={row.label} className="border-b border-gray-100">
                  <td className="py-2 px-4 font-medium text-gray-900">{row.label}</td>
                  {hasCompressed && (
                    <td className="py-2 px-4 text-right font-mono text-green-700">
                      {compressed != null ? formatNumber(compressed) : "-"}
                    </td>
                  )}
                  <td className="py-2 px-4 text-right font-mono">{formatNumber(tracked ?? 0)}</td>
                  {hasReadiness && (
                    <>
                      <td className="py-2 px-4 text-right font-mono">
                        {row.stats?.ready != null ? formatNumber(row.stats.ready) : "-"}
                      </td>
                      <td className="py-2 px-4 text-right font-mono">
                        {row.stats?.waiting != null ? formatNumber(row.stats.waiting) : "-"}
                      </td>
                      <td className="py-2 px-4 text-right font-mono text-gray-500">
                        {formatSlotCountdown(data.current_slot, row.stats?.next_ready_slot, row.stats?.ready, row.stats?.waiting)}
                      </td>
                    </>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {data.pda_programs && data.pda_programs.length > 0 && (
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <h3 className="text-sm font-semibold text-gray-900 mb-2">PDA Programs</h3>
          <div className="space-y-1">
            {data.pda_programs.map((p) => (
              <div key={p.program_id} className="flex justify-between text-xs">
                <span className="font-mono text-gray-700">
                  {p.program_id.slice(0, 8)}...{p.program_id.slice(-4)}
                </span>
                <span className="text-gray-500">{formatNumber(p.tracked)} pending</span>
              </div>
            ))}
          </div>
        </div>
      )}

      <p className="text-[10px] text-gray-400">
        Source: {data.source ?? "unknown"}
        {data.cached_at ? ` \u00b7 ${formatAgeFromUnixSeconds(data.cached_at)}` : ""}
        {data.note ? ` \u00b7 ${data.note}` : ""}
      </p>
    </div>
  );
}
