import type {
  CompressibleResponse,
  CompressibleTypeStats,
} from "@/types/forester";
import {
  formatAgeFromUnixSeconds,
  formatNumber,
  formatSlotCountdown,
  truncateAddress,
} from "@/lib/utils";

interface CompressiblePanelProps {
  data: CompressibleResponse;
}

export function CompressiblePanel({ data }: CompressiblePanelProps) {
  const source = data.source ?? "none";
  const sourceBadge =
    source === "tracker"
      ? "bg-emerald-100 text-emerald-800"
      : source === "forester-apis"
        ? "bg-sky-100 text-sky-800"
        : source === "rpc"
          ? "bg-blue-100 text-blue-800"
          : "bg-gray-100 text-gray-700";

  const sourceLabel =
    source === "tracker"
      ? "In-memory tracker"
      : source === "forester-apis"
        ? "Forester API aggregate"
        : source === "rpc"
          ? "RPC scan"
          : "Unavailable";

  const rows: {
    key: string;
    label: string;
    description: string;
    stats?: CompressibleTypeStats;
    fallbackCount?: number;
    contributesToTotal?: boolean;
  }[] = [
    {
      key: "ctoken",
      label: "Light Token",
      description: "Fungible decompressed token accounts (includes ATA)",
      stats: data.ctoken,
      fallbackCount: data.ctoken_count,
      contributesToTotal: true,
    },
    {
      key: "ata",
      label: "ATA",
      description: "Associated token accounts",
      stats: data.ata,
      fallbackCount: data.ata_count,
      contributesToTotal: false,
    },
    {
      key: "pda",
      label: "PDA",
      description: "Program-derived account records",
      stats: data.pda,
      fallbackCount: data.pda_count,
      contributesToTotal: true,
    },
    {
      key: "mint",
      label: "Mint",
      description: "Non-fungible mint accounts",
      stats: data.mint,
      fallbackCount: data.mint_count,
      contributesToTotal: true,
    },
  ];

  const inferredTracked = rows.reduce((acc, row) => {
    if (row.contributesToTotal === false) {
      return acc;
    }
    const tracked = row.stats?.tracked ?? row.fallbackCount ?? 0;
    return acc + tracked;
  }, 0);

  const totalTracked = data.total_tracked ?? inferredTracked;
  const totalReady = data.total_ready;
  const totalWaiting = data.total_waiting;
  const totalUnknown =
    totalReady != null && totalWaiting != null
      ? Math.max(0, totalTracked - totalReady - totalWaiting)
      : totalTracked;

  const noData =
    !data.enabled &&
    rows.every((row) => (row.stats?.tracked ?? row.fallbackCount ?? 0) === 0);

  const upstreamTotal = data.upstreams?.length ?? 0;
  const upstreamHealthy = data.upstreams?.filter((u) => u.ok).length ?? 0;
  const nextGlobalReadySlot = rows
    .map((row) => row.stats?.next_ready_slot)
    .filter((slot): slot is number => slot != null)
    .reduce<number | null>((min, slot) => (min == null ? slot : Math.min(min, slot)), null);

  const notices: { level: "info" | "warn" | "error"; message: string }[] = [];
  if (totalTracked === 0) {
    notices.push({
      level: "info",
      message: "No compressible accounts are currently tracked.",
    });
  }
  if ((totalReady ?? 0) > 0) {
    notices.push({
      level: "info",
      message:
        "Funding status: some tracked accounts are currently eligible for compression.",
    });
  } else if ((totalWaiting ?? 0) > 0) {
    notices.push({
      level: "info",
      message: `Funding status: tracked accounts are currently funded. Next eligibility: ${formatSlotCountdown(
        data.current_slot,
        nextGlobalReadySlot
      )}.`,
    });
  }
  if (upstreamTotal > 0 && upstreamHealthy < upstreamTotal) {
    notices.push({
      level: "error",
      message: `Only ${upstreamHealthy}/${upstreamTotal} upstream forester API endpoints are healthy.`,
    });
  }

  return (
    <div className="space-y-4">
      <div className="bg-white rounded-lg border border-gray-200 p-4">
        <div className="flex flex-wrap gap-2 items-center">
          <span
            className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${sourceBadge}`}
          >
            Source: {sourceLabel}
          </span>
          <span className="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-700">
            Updated: {formatAgeFromUnixSeconds(data.cached_at)}
          </span>
          <span className="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-700">
            Slot:{" "}
            {data.current_slot != null ? formatNumber(data.current_slot) : "unknown"}
          </span>
          {data.refresh_interval_secs != null && (
            <span className="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-700">
              Refresh: {data.refresh_interval_secs}s
            </span>
          )}
          {upstreamTotal > 0 && (
            <span className="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-700">
              Upstreams: {upstreamHealthy}/{upstreamTotal}
            </span>
          )}
        </div>

        {noData && (
          <p className="text-gray-500 text-sm mt-3">
            No compressible account data available yet.
          </p>
        )}

        {data.error && (
          <div className="mt-3 rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">
            {data.error}
          </div>
        )}

        {data.note && (
          <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
            {data.note}
          </div>
        )}

        {notices.map((notice, idx) => (
          <div
            key={`${notice.level}-${idx}`}
            className={`mt-3 rounded-md border p-3 text-sm ${
              notice.level === "error"
                ? "border-red-200 bg-red-50 text-red-700"
                : notice.level === "warn"
                  ? "border-amber-200 bg-amber-50 text-amber-800"
                  : "border-blue-200 bg-blue-50 text-blue-800"
            }`}
          >
            {notice.message}
          </div>
        ))}

        {data.upstreams && data.upstreams.length > 0 && (
          <div className="mt-3 rounded-md border border-gray-200 bg-gray-50 p-3">
            <h3 className="text-sm font-semibold text-gray-900 mb-2">
              Upstream Forester APIs
            </h3>
            <div className="space-y-1">
              {data.upstreams.map((upstream) => (
                <div
                  key={upstream.base_url}
                  className="flex items-center justify-between gap-3 text-xs"
                >
                  <span className="font-mono text-gray-700">
                    {truncateAddress(upstream.base_url, 12)}
                  </span>
                  <span
                    className={`rounded-full px-2 py-0.5 font-medium ${
                      upstream.ok
                        ? "bg-emerald-100 text-emerald-800"
                        : "bg-red-100 text-red-700"
                    }`}
                  >
                    {upstream.ok ? "ok" : "down"}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {[
          {
            label: "Tracked Accounts",
            value: formatNumber(totalTracked),
            desc: "Total visible across account types",
          },
          {
            label: "Ready Now",
            value: totalReady != null ? formatNumber(totalReady) : "unknown",
            desc: "Eligible for compression at current slot",
          },
          {
            label: "Waiting",
            value: totalWaiting != null ? formatNumber(totalWaiting) : "unknown",
            desc: "Not yet at compressible slot",
          },
          {
            label: "Unknown",
            value: totalUnknown != null ? formatNumber(totalUnknown) : "unknown",
            desc: "Tracked with missing readiness context",
          },
        ].map((card) => (
          <div
            key={card.label}
            className="bg-white rounded-lg border border-gray-200 p-4"
          >
            <div className="text-xs text-gray-500">{card.label}</div>
            <div className="text-2xl font-semibold text-gray-900 mt-1">
              {card.value}
            </div>
            <div className="text-xs text-gray-400 mt-1">{card.desc}</div>
          </div>
        ))}
      </div>
      <p className="text-xs text-gray-500">
        ATA counts are included in Light Token totals.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
        {rows.map((row) => {
          const tracked = row.stats?.tracked ?? row.fallbackCount;
          const ready = row.stats?.ready;
          const waiting = row.stats?.waiting;
          const unknown =
            tracked != null && ready != null && waiting != null
              ? Math.max(0, tracked - ready - waiting)
              : tracked;

          return (
            <div
              key={row.key}
              className="bg-white rounded-lg border border-gray-200 p-4"
            >
              <div className="text-xs text-gray-500">{row.label}</div>
              <div className="text-sm text-gray-400 mt-0.5">{row.description}</div>
              <div className="mt-3 space-y-1 text-sm">
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">Tracked</span>
                  <span className="font-medium text-gray-900">
                    {tracked != null ? formatNumber(tracked) : "unknown"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">Ready now</span>
                  <span className="font-medium text-gray-900">
                    {ready != null ? formatNumber(ready) : "unknown"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">Waiting</span>
                  <span className="font-medium text-gray-900">
                    {waiting != null ? formatNumber(waiting) : "unknown"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">Unknown</span>
                  <span className="font-medium text-gray-900">
                    {unknown != null ? formatNumber(unknown) : "unknown"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">Next ready</span>
                  <span className="font-medium text-gray-900">
                    {formatSlotCountdown(
                      data.current_slot,
                      row.stats?.next_ready_slot
                    )}
                  </span>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {data.pda_programs && data.pda_programs.length > 0 && (
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <h3 className="text-sm font-semibold text-gray-900 mb-3">
            PDA Program Breakdown
          </h3>
          <div className="space-y-2">
            {data.pda_programs.map((program) => (
              <div
                key={program.program_id}
                className="rounded-md border border-gray-100 bg-gray-50 px-3 py-2 text-sm"
              >
                <div className="flex items-center justify-between">
                  <span className="font-mono text-gray-700">
                    {truncateAddress(program.program_id, 6)}
                  </span>
                  <span className="text-gray-500">
                    tracked {formatNumber(program.tracked)}
                  </span>
                </div>
                <div className="mt-1 grid grid-cols-3 gap-2 text-xs text-gray-600">
                  <span>
                    ready{" "}
                    {program.ready != null ? formatNumber(program.ready) : "unknown"}
                  </span>
                  <span>
                    waiting{" "}
                    {program.waiting != null
                      ? formatNumber(program.waiting)
                      : "unknown"}
                  </span>
                  <span>
                    next{" "}
                    {formatSlotCountdown(data.current_slot, program.next_ready_slot)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
