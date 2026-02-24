import type { PhotonStats } from "@/types/forester";
import { formatNumber, formatAgeFromUnixSeconds } from "@/lib/utils";

interface PhotonStatsPanelProps {
  data: PhotonStats;
}

function compactNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return formatNumber(n);
}

export function PhotonStatsPanel({ data }: PhotonStatsPanelProps) {
  if (data.error) {
    return (
      <div className="bg-white rounded-lg border border-red-200 p-4">
        <p className="text-sm text-red-700">{data.error}</p>
      </div>
    );
  }

  const rows = [
    {
      label: "Compressed Accounts",
      total: data.accounts.total,
      active: data.accounts.active,
    },
    {
      label: "Token Accounts",
      total: data.token_accounts.total,
      active: data.token_accounts.active,
    },
    {
      label: "Compressed by Forester",
      total: data.compressed_from_onchain.total,
      active: data.compressed_from_onchain.active,
    },
  ];

  return (
    <div className="space-y-3">
      {/* Summary cards */}
      <div className="grid grid-cols-3 gap-3">
        <MiniStat label="Total Accounts" value={data.accounts.total} />
        <MiniStat label="Token Accounts" value={data.token_accounts.total} />
        <MiniStat
          label="By Forester"
          value={data.compressed_from_onchain.total}
        />
      </div>

      {/* Detail table */}
      <div className="bg-white rounded-lg border border-gray-200 overflow-hidden">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-200 text-left text-gray-500">
              <th className="py-2 px-4 font-medium">Category</th>
              <th className="py-2 px-4 font-medium text-right">Total</th>
              <th className="py-2 px-4 font-medium text-right">Active</th>
              <th className="py-2 px-4 font-medium text-right">Spent</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.label} className="border-b border-gray-100">
                <td className="py-2 px-4 font-medium text-gray-900">
                  {row.label}
                </td>
                <td className="py-2 px-4 text-right font-mono">
                  {formatNumber(row.total)}
                </td>
                <td className="py-2 px-4 text-right font-mono text-green-700">
                  {formatNumber(row.active)}
                </td>
                <td className="py-2 px-4 text-right font-mono text-gray-400">
                  {formatNumber(row.total - row.active)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <p className="text-[10px] text-gray-400">
        Source: photon DB
        {data.timestamp
          ? ` · ${formatAgeFromUnixSeconds(data.timestamp)}`
          : ""}
      </p>
    </div>
  );
}

function MiniStat({ label, value }: { label: string; value: number }) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-3">
      <div className="text-[10px] text-gray-500 uppercase tracking-wide">
        {label}
      </div>
      <div className="text-xl font-semibold text-gray-900 mt-0.5">
        {compactNumber(value)}
      </div>
    </div>
  );
}
