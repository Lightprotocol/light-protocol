"use client";

import { useForesterStatus } from "@/hooks/useForesterStatus";
import { ErrorState } from "@/components/ErrorState";
import { TreeTable } from "@/components/TreeTable";

export default function TreesPage() {
  const { data: status, error, isLoading } = useForesterStatus();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400 text-sm">Loading trees...</div>
      </div>
    );
  }

  if (error || !status) {
    return <ErrorState error={error} fallbackMessage="Failed to load trees" />;
  }

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-bold">Trees</h2>
      <TreeTable
        trees={status.trees}
        foresters={status.active_epoch_foresters}
        currentLightSlot={status.current_light_slot}
      />
    </div>
  );
}
