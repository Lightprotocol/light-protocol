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
} from "@/lib/utils";

interface TreeTableProps {
  trees: TreeStatus[];
  foresters: ForesterInfo[];
  currentLightSlot: number | null;
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

      <div className="overflow-x-auto">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-200 text-left text-gray-500">
              <th className="py-2 pr-3 font-medium">Type</th>
              <th className="py-2 pr-3 font-medium">Address</th>
              <th className="py-2 pr-3 font-medium">Fullness</th>
              <th className="py-2 pr-3 font-medium">Index / Cap</th>
              <th className="py-2 pr-3 font-medium">Pending</th>
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
                    <td className="py-2 pr-3 font-mono" title={tree.merkle_tree}>
                      {truncateAddress(tree.merkle_tree, 6)}
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
                        <span className="font-mono">
                          I:{tree.v2_queue_info.input_pending_batches *
                            tree.v2_queue_info.zkp_batch_size}{" "}
                          {tree.tree_type === "StateV2" && (
                            <>
                              O:
                              {tree.v2_queue_info.output_pending_batches *
                                tree.v2_queue_info.zkp_batch_size}
                            </>
                          )}
                        </span>
                      ) : (
                        <span className="font-mono">
                          {tree.queue_length != null
                            ? formatNumber(tree.queue_length)
                            : "-"}
                        </span>
                      )}
                    </td>
                    <td className="py-2 pr-3 font-mono">
                      {tree.assigned_forester
                        ? truncateAddress(tree.assigned_forester, 4)
                        : "-"}
                    </td>
                    <td className="py-2">
                      <ScheduleGrid
                        schedule={tree.schedule}
                        currentSlot={currentLightSlot}
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

function ScheduleGrid({
  schedule,
  currentSlot,
}: {
  schedule: (number | null)[];
  currentSlot: number | null;
}) {
  if (schedule.length === 0)
    return <span className="text-gray-400">-</span>;

  // Show a compact view: only around the current slot
  const start = currentSlot != null ? Math.max(0, currentSlot - 2) : 0;
  const visible = schedule.slice(start, start + 8);

  return (
    <div className="flex gap-px">
      {visible.map((slot, i) => {
        const idx = start + i;
        const isCurrent = idx === currentSlot;
        return (
          <div
            key={idx}
            className={`w-2.5 h-2.5 rounded-sm ${
              slot != null ? "bg-green-400" : "bg-gray-200"
            } ${isCurrent ? "ring-1 ring-gray-900" : ""}`}
            title={`Slot ${idx}: ${slot != null ? `Forester #${slot}` : "unassigned"}`}
          />
        );
      })}
      {schedule.length > 8 && (
        <span className="text-gray-400 ml-1">+{schedule.length - 8}</span>
      )}
    </div>
  );
}
