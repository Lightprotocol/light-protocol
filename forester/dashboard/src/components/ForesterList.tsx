import type { ForesterInfo } from "@/types/forester";
import { truncateAddress, formatSol } from "@/lib/utils";

interface ForesterListProps {
  title: string;
  foresters: ForesterInfo[];
}

export function ForesterList({ title, foresters }: ForesterListProps) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <h3 className="text-sm font-semibold text-gray-900 mb-3">
        {title}{" "}
        <span className="text-gray-400 font-normal">({foresters.length})</span>
      </h3>
      {foresters.length === 0 ? (
        <p className="text-xs text-amber-600">No foresters registered</p>
      ) : (
        <div className="space-y-2">
          {foresters.map((f, i) => {
            const low = f.balance_sol != null && f.balance_sol < 0.1;
            return (
              <div
                key={i}
                className="flex items-center justify-between text-xs"
              >
                <span className="font-mono text-gray-700" title={f.authority}>
                  {truncateAddress(f.authority, 6)}
                </span>
                <span className={low ? "text-red-600 font-medium" : "text-gray-600"}>
                  {formatSol(f.balance_sol)}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
