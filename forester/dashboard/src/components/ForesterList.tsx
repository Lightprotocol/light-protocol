import type { ForesterInfo } from "@/types/forester";
import { truncateAddress, formatSol, explorerUrl } from "@/lib/utils";
import type { BalanceTrend } from "@/hooks/useBalanceHistory";

interface ForesterListProps {
  active: ForesterInfo[];
  registering: ForesterInfo[];
  activeEpoch: number;
  registrationEpoch: number;
  getTrend?: (authority: string, hours?: number) => BalanceTrend | null;
}

function BalanceTrendBadge({ trend }: { trend: BalanceTrend }) {
  const ratePerHour = trend.hourlyRate;
  const burning = ratePerHour < -0.005; // more than 0.005 SOL/hr
  const fast = ratePerHour < -0.05; // more than 0.05 SOL/hr

  if (!burning) return null;

  const hoursLeft =
    ratePerHour < 0 ? Math.abs(trend.current / ratePerHour) : Infinity;
  const rateStr = Math.abs(ratePerHour).toFixed(3);

  return (
    <span
      className={`text-[10px] ml-1 ${fast ? "text-red-600 font-medium" : "text-amber-600"}`}
      title={`${rateStr} SOL/hr burn over ${trend.hoursTracked.toFixed(1)}h. ~${hoursLeft.toFixed(0)}h until empty.`}
    >
      -{rateStr}/hr
      {hoursLeft < 48 && (
        <span className="text-red-600"> (~{hoursLeft.toFixed(0)}h left)</span>
      )}
    </span>
  );
}

function ForesterRow({
  info,
  trend,
}: {
  info: ForesterInfo;
  trend: BalanceTrend | null;
}) {
  const low = info.balance_sol != null && info.balance_sol < 0.1;
  return (
    <div className="flex items-center justify-between text-xs py-1">
      <a
        href={explorerUrl(info.authority)}
        target="_blank"
        rel="noopener noreferrer"
        className="font-mono text-gray-700 hover:text-blue-600 hover:underline"
        title={info.authority}
      >
        {truncateAddress(info.authority, 6)}
      </a>
      <div className="flex items-center">
        <span className={low ? "text-red-600 font-medium" : "text-gray-500"}>
          {formatSol(info.balance_sol)}
        </span>
        {trend && <BalanceTrendBadge trend={trend} />}
      </div>
    </div>
  );
}

function ForesterSection({
  title,
  epoch,
  foresters,
  badgeClass,
  emptyMessage,
  getTrend,
}: {
  title: string;
  epoch: number;
  foresters: ForesterInfo[];
  badgeClass: string;
  emptyMessage: string;
  getTrend?: (authority: string, hours?: number) => BalanceTrend | null;
}) {
  return (
    <div>
      <div className="flex items-center gap-2 mb-2">
        <h4 className="text-xs font-semibold text-gray-700">{title}</h4>
        <span className={`rounded-full px-1.5 py-0.5 text-[10px] font-medium ${badgeClass}`}>
          epoch {epoch}
        </span>
        <span className="text-[10px] text-gray-400 ml-auto">
          {foresters.length} forester{foresters.length !== 1 ? "s" : ""}
        </span>
      </div>
      {foresters.length === 0 ? (
        <p className="text-xs text-amber-600 pl-1">{emptyMessage}</p>
      ) : (
        <div className="pl-1">
          {foresters.map((f, i) => (
            <ForesterRow
              key={i}
              info={f}
              trend={getTrend ? getTrend(f.authority, 6) : null}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function ForesterList({
  active,
  registering,
  activeEpoch,
  registrationEpoch,
  getTrend,
}: ForesterListProps) {
  // Foresters registered for next epoch but not in current
  const newNextEpoch = registering.filter(
    (f) => !active.some((a) => a.authority === f.authority)
  );
  // Foresters in current epoch that haven't re-registered for next
  const notReRegistered = active.filter(
    (f) => !registering.some((r) => r.authority === f.authority)
  );

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <h3 className="text-sm font-semibold text-gray-900 mb-4">Foresters</h3>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <ForesterSection
          title="Currently Active"
          epoch={activeEpoch}
          foresters={active}
          badgeClass="bg-green-100 text-green-700"
          emptyMessage="No foresters active — queues are not being processed"
          getTrend={getTrend}
        />
        <ForesterSection
          title="Registered for Next Epoch"
          epoch={registrationEpoch}
          foresters={registering}
          badgeClass="bg-blue-100 text-blue-700"
          emptyMessage="No foresters registered yet"
          getTrend={getTrend}
        />
      </div>
      {/* Continuity warnings */}
      {(notReRegistered.length > 0 || newNextEpoch.length > 0) && (
        <div className="mt-4 pt-3 border-t border-gray-100 space-y-1">
          {notReRegistered.length > 0 && (
            <p className="text-[11px] text-amber-600">
              {notReRegistered.length} active forester{notReRegistered.length !== 1 ? "s" : ""} not yet registered for next epoch:{" "}
              {notReRegistered.map((f) => truncateAddress(f.authority, 4)).join(", ")}
            </p>
          )}
          {newNextEpoch.length > 0 && (
            <p className="text-[11px] text-blue-600">
              {newNextEpoch.length} new forester{newNextEpoch.length !== 1 ? "s" : ""} joining next epoch:{" "}
              {newNextEpoch.map((f) => truncateAddress(f.authority, 4)).join(", ")}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
