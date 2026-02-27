"use client";

import { useState, useEffect, useRef } from "react";
import type { ForesterStatus } from "@/types/forester";
import { ProgressBar } from "./ProgressBar";
import { formatPercentage, formatNumber } from "@/lib/utils";

interface EpochCardProps {
  status: ForesterStatus;
}

const SLOT_MS = 400;

function useCountdown(slots: number): string {
  const [totalMs, setTotalMs] = useState(slots * SLOT_MS);
  const slotsRef = useRef(slots);

  // Reset when slot count changes significantly (new data from API)
  useEffect(() => {
    if (Math.abs(slots - slotsRef.current) > 5) {
      setTotalMs(slots * SLOT_MS);
      slotsRef.current = slots;
    }
  }, [slots]);

  useEffect(() => {
    const interval = setInterval(() => {
      setTotalMs((prev) => Math.max(0, prev - 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const totalSec = Math.max(0, Math.floor(totalMs / 1000));
  const hours = Math.floor(totalSec / 3600);
  const mins = Math.floor((totalSec % 3600) / 60);
  const secs = totalSec % 60;

  if (hours > 0) return `${hours}h ${mins}m ${secs}s`;
  if (mins > 0) return `${mins}m ${secs}s`;
  return `${secs}s`;
}

export function EpochCard({ status }: EpochCardProps) {
  const registrationClosingSoon =
    status.slots_until_next_registration < 5000 &&
    status.registration_epoch_foresters.length === 0;

  const remainingSlots = status.active_phase_length - status.active_epoch_progress;
  const epochCountdown = useCountdown(remainingSlots);
  const regCountdown = useCountdown(status.slots_until_next_registration);

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <h3 className="text-sm font-semibold text-gray-900 mb-3">Epoch Timeline</h3>
      <div className="space-y-4">
        {/* Active epoch */}
        <div>
          <div className="flex justify-between text-xs text-gray-600 mb-1">
            <span className="flex items-center gap-1.5">
              <span className="inline-block w-1.5 h-1.5 rounded-full bg-green-500" />
              Active Epoch {status.current_active_epoch}
            </span>
            <span>{formatPercentage(status.active_epoch_progress_percentage)}</span>
          </div>
          <ProgressBar value={status.active_epoch_progress_percentage} />
          <div className="flex justify-between text-[10px] text-gray-400 mt-0.5">
            <span>{formatNumber(status.active_epoch_progress)} / {formatNumber(status.active_phase_length)} slots</span>
            <span className="font-mono tabular-nums">{epochCountdown}</span>
          </div>
        </div>

        {/* Registration epoch */}
        <div>
          <div className="flex justify-between text-xs text-gray-600 mb-1">
            <span className="flex items-center gap-1.5">
              <span className={`inline-block w-1.5 h-1.5 rounded-full ${registrationClosingSoon ? "bg-amber-500" : "bg-blue-500"}`} />
              Registration Epoch {status.current_registration_epoch}
            </span>
            <span className={`font-mono tabular-nums ${registrationClosingSoon ? "text-amber-600 font-medium" : ""}`}>
              {regCountdown}
            </span>
          </div>
          <div className="text-[10px] text-gray-400">
            {status.registration_epoch_foresters.length} forester{status.registration_epoch_foresters.length !== 1 ? "s" : ""} registered
            {registrationClosingSoon && (
              <span className="text-amber-600 ml-1">— closing soon, no foresters!</span>
            )}
          </div>
        </div>

        {/* Stats grid */}
        <div className="grid grid-cols-3 gap-3 text-xs border-t border-gray-100 pt-3">
          <div>
            <span className="text-gray-500">Next epoch</span>
            <p className="font-medium font-mono tabular-nums">{epochCountdown}</p>
          </div>
          <div>
            <span className="text-gray-500">Light slot</span>
            <p className="font-medium">
              {status.current_light_slot != null
                ? `${status.current_light_slot} / ${status.total_light_slots}`
                : "N/A"}
            </p>
          </div>
          <div>
            <span className="text-gray-500">Slot length</span>
            <p className="font-medium">{formatNumber(status.light_slot_length)} slots</p>
          </div>
        </div>
      </div>
    </div>
  );
}
