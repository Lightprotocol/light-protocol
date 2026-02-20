"use client";

import { useCompressible } from "@/hooks/useCompressible";
import { ErrorState } from "@/components/ErrorState";
import { CompressiblePanel } from "@/components/CompressiblePanel";

export default function CompressiblePage() {
  const { data, error, isLoading } = useCompressible();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400 text-sm">
          Loading compressible status...
        </div>
      </div>
    );
  }

  if (error || !data) {
    return <ErrorState error={error} fallbackMessage="Failed to load compressible data" />;
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-xl font-bold">Compressible Accounts</h2>
        <p className="text-sm text-gray-500 mt-1">
          Track what is currently compressible, what is waiting on rent
          windows, and how fresh this view is.
        </p>
      </div>
      <CompressiblePanel data={data} />
    </div>
  );
}
