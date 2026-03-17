"use client";

import { useState, useMemo } from "react";
import type { TreeStatus, ForesterInfo } from "@/types/forester";
import { StatusBadge } from "./StatusBadge";
import { ProgressBar } from "./ProgressBar";
import { TreeBatchDetail } from "./TreeBatchDetail";
import {
  truncateAddress,
  formatNumber,
  formatPercentage,
  treeTypeColor,
  explorerUrl,
} from "@/lib/utils";

interface TreeTableProps {
  trees: TreeStatus[];
  foresters: ForesterInfo[];
  currentLightSlot: number | null;
}

function pendingColor(count: number, isBatches: boolean): string {
  if (count === 0) return "text-gray-400";
  if (isBatches) {
    // V2: batches — 1 is normal, 2+ is busy, 3+ is hot
    if (count >= 3) return "text-red-600 font-medium";
    if (count >= 2) return "text-amber-600 font-medium";
    return "text-gray-700";
  }
  // V1: item count
  if (count >= 500) return "text-red-600 font-medium";
  if (count >= 100) return "text-amber-600 font-medium";
  return "text-gray-700";
}

type SortKey = "type" | "fullness" | "pending";
type FilterType = "all" | "StateV1" | "StateV2" | "AddressV1" | "AddressV2";

export function TreeTable({
  trees,
  foresters,
  currentLightSlot,
}: TreeTableProps) {
  const [filter, setFilter] = useState<FilterType>("all");
  const [sortBy, setSortBy] = useState<SortKey>("pending");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [showRolledOver, setShowRolledOver] = useState(false);

  const filtered = useMemo(() => {
    let result = trees;
    if (!showRolledOver) {
      result = result.filter((t) => !t.is_rolledover);
    }
    if (filter !== "all") {
      result = result.filter((t) => t.tree_type === filter);
    }
    return [...result].sort((a, b) => {
      switch (sortBy) {
        case "type":
          return a.tree_type.localeCompare(b.tree_type);
        case "fullness":
          return b.fullness_percentage - a.fullness_percentage;
        case "pending":
          return (b.queue_length ?? 0) - (a.queue_length ?? 0);
        default:
          return 0;
      }
    });
  }, [trees, filter, sortBy, showRolledOver]);

  const filters: FilterType[] = [
    "all",
    "StateV1",
    "StateV2",
    "AddressV1",
    "AddressV2",
  ];

  return (
    <div>
      <div className="flex items-center gap-2 mb-4 flex-wrap">
        {filters.map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
              filter === f
                ? "bg-gray-900 text-white"
                : "bg-gray-100 text-gray-700 hover:bg-gray-200"
            }`}
          >
            {f === "all" ? "All" : f}
          </button>
        ))}
        <div className="ml-auto flex items-center gap-3">
          <label className="flex items-center gap-1.5 text-xs text-gray-600">
            <input
              type="checkbox"
              checked={showRolledOver}
              onChange={(e) => setShowRolledOver(e.target.checked)}
              className="rounded"
            />
            Rolled over
          </label>
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortKey)}
            className="text-xs border border-gray-300 rounded px-2 py-1"
          >
            <option value="pending">Sort: Pending</option>
            <option value="fullness">Sort: Fullness</option>
            <option value="type">Sort: Type</option>
          </select>
        </div>
      </div>

      <ForesterScheduleLegend foresters={foresters} />

      <div className="overflow-x-auto mt-3">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-200 text-left text-gray-500">
              <th className="py-2 pr-3 font-medium">Type</th>
              <th className="py-2 pr-3 font-medium">Address</th>
              <th className="py-2 pr-3 font-medium">Fullness</th>
              <th className="py-2 pr-3 font-medium">Index / Cap</th>
              <th className="py-2 pr-3 font-medium">Queue</th>
              <th className="py-2 pr-3 font-medium">Forester</th>
              <th className="py-2 font-medium">Schedule</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((tree) => {
              const isExpanded = expanded === tree.merkle_tree;
              return (
                <Fragment key={tree.merkle_tree}>
                  <tr
                    className={`border-b border-gray-100 hover:bg-gray-50 cursor-pointer ${
                      tree.is_rolledover ? "opacity-50" : ""
                    }`}
                    onClick={() =>
                      setExpanded(isExpanded ? null : tree.merkle_tree)
                    }
                  >
                    <td className="py-2 pr-3">
                      <StatusBadge
                        label={tree.tree_type}
                        color={treeTypeColor(tree.tree_type)}
                      />
                    </td>
                    <td className="py-2 pr-3 font-mono">
                      <a
                        href={explorerUrl(tree.merkle_tree)}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-gray-700 hover:text-blue-600 hover:underline"
                        title={tree.merkle_tree}
                        onClick={(e) => e.stopPropagation()}
                      >
                        {truncateAddress(tree.merkle_tree, 6)}
                      </a>
                    </td>
                    <td className="py-2 pr-3">
                      <div className="flex items-center gap-2">
                        <ProgressBar
                          value={tree.fullness_percentage}
                          className="w-16"
                        />
                        <span>
                          {formatPercentage(tree.fullness_percentage)}
                        </span>
                      </div>
                    </td>
                    <td className="py-2 pr-3 font-mono">
                      {formatNumber(tree.next_index)} /{" "}
                      {formatNumber(tree.capacity)}
                    </td>
                    <td className="py-2 pr-3">
                      {tree.v2_queue_info ? (
                        <V2QueueCell tree={tree} />
                      ) : (
                        <div>
                          <span className={`font-mono ${pendingColor(tree.queue_length ?? 0, false)}`}>
                            {tree.queue_length != null
                              ? formatNumber(tree.queue_length)
                              : "-"}
                          </span>
                          {tree.queue_length != null && tree.queue_capacity != null && tree.queue_capacity > 0 && (
                            <span className="text-[10px] text-gray-400 ml-1">
                              / {formatNumber(tree.queue_capacity)}
                              {" "}({formatPercentage(tree.queue_length / tree.queue_capacity * 100)})
                            </span>
                          )}
                        </div>
                      )}
                    </td>
                    <td className="py-2 pr-3 font-mono">
                      {tree.assigned_forester ? (
                        <a
                          href={explorerUrl(tree.assigned_forester)}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-gray-700 hover:text-blue-600 hover:underline"
                          title={tree.assigned_forester}
                          onClick={(e) => e.stopPropagation()}
                        >
                          {truncateAddress(tree.assigned_forester, 4)}
                        </a>
                      ) : "-"}
                    </td>
                    <td className="py-2">
                      <ScheduleGrid
                        schedule={tree.schedule}
                        currentSlot={currentLightSlot}
                        foresters={foresters}
                      />
                    </td>
                  </tr>
                  {isExpanded && tree.v2_queue_info && (
                    <tr>
                      <td colSpan={7} className="p-0">
                        <TreeBatchDetail info={tree.v2_queue_info} />
                      </td>
                    </tr>
                  )}
                </Fragment>
              );
            })}
          </tbody>
        </table>
        {filtered.length === 0 && (
          <p className="text-center text-gray-400 py-8 text-sm">
            No trees match the current filter
          </p>
        )}
      </div>
    </div>
  );
}

import { Fragment } from "react";

function V2QueueCell({ tree }: { tree: TreeStatus }) {
  const info = tree.v2_queue_info!;
  const isState = tree.tree_type === "StateV2";

  if (isState) {
    return (
      <div className="font-mono">
        <span className={pendingColor(info.input_pending_batches, true)}>
          I:{info.input_pending_batches}
        </span>
        {info.input_total_zkp_batches > 0 && (
          <span className="text-[10px] text-gray-400">
            /{info.input_total_zkp_batches}
          </span>
        )}
        {" "}
        <span className={pendingColor(info.output_pending_batches, true)}>
          O:{info.output_pending_batches}
        </span>
        {info.output_total_zkp_batches > 0 && (
          <span className="text-[10px] text-gray-400">
            /{info.output_total_zkp_batches}
          </span>
        )}
      </div>
    );
  }

  return (
    <div className="font-mono">
      <span className={pendingColor(info.input_pending_batches, true)}>
        {info.input_pending_batches}
      </span>
      {info.input_total_zkp_batches > 0 && (
        <span className="text-[10px] text-gray-400">
          /{info.input_total_zkp_batches}
        </span>
      )}
    </div>
  );
}

// Distinct colors for up to 8 foresters; cycles if more
const FORESTER_COLORS = [
  "bg-emerald-400",
  "bg-blue-400",
  "bg-amber-400",
  "bg-rose-400",
  "bg-violet-400",
  "bg-cyan-400",
  "bg-orange-400",
  "bg-pink-400",
];

const FORESTER_HEX = [
  "#34d399",
  "#60a5fa",
  "#fbbf24",
  "#fb7185",
  "#a78bfa",
  "#22d3ee",
  "#fb923c",
  "#f472b6",
];

function foresterColor(index: number): string {
  return FORESTER_COLORS[index % FORESTER_COLORS.length];
}

function ScheduleGrid({
  schedule,
  currentSlot,
  foresters,
}: {
  schedule: (number | null)[];
  currentSlot: number | null;
  foresters: ForesterInfo[];
}) {
  if (schedule.length === 0)
    return <span className="text-gray-400">-</span>;

  // Show a compact view: only around the current slot
  const start = currentSlot != null ? Math.max(0, currentSlot - 2) : 0;
  const visible = schedule.slice(start, start + 12);

  return (
    <div className="flex gap-px items-center">
      {visible.map((slot, i) => {
        const idx = start + i;
        const isCurrent = idx === currentSlot;
        const foresterName =
          slot != null && foresters[slot]
            ? truncateAddress(foresters[slot].authority, 3)
            : slot != null
              ? `#${slot}`
              : "unassigned";
        return (
          <div
            key={idx}
            className={`w-2.5 h-2.5 rounded-sm ${
              slot != null ? foresterColor(slot) : "bg-gray-200"
            } ${isCurrent ? "ring-1 ring-offset-1 ring-gray-900" : ""}`}
            title={`Light slot ${idx}: ${foresterName}`}
          />
        );
      })}
      {schedule.length > start + 12 && (
        <span className="text-gray-400 ml-1 text-[10px]">+{schedule.length - start - 12}</span>
      )}
    </div>
  );
}

export function ForesterScheduleLegend({ foresters }: { foresters: ForesterInfo[] }) {
  if (foresters.length === 0) return null;
  return (
    <div className="flex items-center gap-3 flex-wrap text-[10px] text-gray-500">
      <span className="font-medium text-gray-700">Schedule:</span>
      {foresters.map((f, i) => (
        <div key={i} className="flex items-center gap-1">
          <div
            className="w-2.5 h-2.5 rounded-sm"
            style={{ backgroundColor: FORESTER_HEX[i % FORESTER_HEX.length] }}
          />
          <a
            href={explorerUrl(f.authority)}
            target="_blank"
            rel="noopener noreferrer"
            className="font-mono hover:text-blue-600 hover:underline"
          >
            {truncateAddress(f.authority, 4)}
          </a>
        </div>
      ))}
      <div className="flex items-center gap-1">
        <div className="w-2.5 h-2.5 rounded-sm bg-gray-200" />
        <span>unassigned</span>
      </div>
      <div className="flex items-center gap-1">
        <div className="w-2.5 h-2.5 rounded-sm bg-gray-200 ring-1 ring-offset-1 ring-gray-900" />
        <span>current</span>
      </div>
    </div>
  );
}
