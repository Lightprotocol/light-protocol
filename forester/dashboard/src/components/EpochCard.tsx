import type { ForesterStatus } from "@/types/forester";
import { ProgressBar } from "./ProgressBar";
import { formatPercentage, slotsToTime } from "@/lib/utils";

interface EpochCardProps {
  status: ForesterStatus;
}

export function EpochCard({ status }: EpochCardProps) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <h3 className="text-sm font-semibold text-gray-900 mb-3">
        Epoch Status
      </h3>
      <div className="space-y-3">
        <div>
          <div className="flex justify-between text-xs text-gray-600 mb-1">
            <span>Active Epoch {status.current_active_epoch}</span>
            <span>
              {formatPercentage(status.active_epoch_progress_percentage)}
            </span>
          </div>
          <ProgressBar value={status.active_epoch_progress_percentage} />
        </div>
        <div className="grid grid-cols-2 gap-3 text-xs">
          <div>
            <span className="text-gray-500">Next Epoch</span>
            <p className="font-medium">{status.hours_until_next_epoch}h</p>
          </div>
          <div>
            <span className="text-gray-500">Registration Epoch</span>
            <p className="font-medium">{status.current_registration_epoch}</p>
          </div>
          <div>
            <span className="text-gray-500">Next Registration</span>
            <p className="font-medium">
              {slotsToTime(status.slots_until_next_registration)}
            </p>
          </div>
          <div>
            <span className="text-gray-500">Light Slot</span>
            <p className="font-medium">
              {status.current_light_slot != null
                ? `${status.current_light_slot} / ${status.total_light_slots}`
                : "N/A"}
            </p>
          </div>
        </div>
        {status.current_light_slot != null &&
          status.slots_until_next_light_slot != null && (
            <div className="text-xs text-gray-500">
              Next light slot in{" "}
              {slotsToTime(status.slots_until_next_light_slot)} (
              {status.slots_until_next_light_slot} slots)
            </div>
          )}
      </div>
    </div>
  );
}
