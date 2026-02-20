export function truncateAddress(addr: string, chars = 4): string {
  if (addr.length <= chars * 2 + 3) return addr;
  return `${addr.slice(0, chars)}...${addr.slice(-chars)}`;
}

export function formatSol(lamports: number | null | undefined): string {
  if (lamports == null) return "-";
  return `${lamports.toFixed(4)} SOL`;
}

export function formatNumber(n: number): string {
  return n.toLocaleString();
}

export function formatPercentage(n: number, decimals = 2): string {
  return `${n.toFixed(decimals)}%`;
}

const DEFAULT_SLOT_DURATION_MS = 400;

export function slotsToTime(slots: number): string {
  const seconds = Math.round((slots * DEFAULT_SLOT_DURATION_MS) / 1000);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  const hours = Math.floor(seconds / 3600);
  const mins = Math.round((seconds % 3600) / 60);
  return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
}

export function formatAgeFromUnixSeconds(unixTs: number | null | undefined): string {
  if (unixTs == null || unixTs <= 0) return "unknown";
  const ageSec = Math.max(0, Math.floor(Date.now() / 1000) - unixTs);
  if (ageSec < 5) return "just now";
  if (ageSec < 60) return `${ageSec}s ago`;
  if (ageSec < 3600) return `${Math.round(ageSec / 60)}m ago`;
  const hours = Math.floor(ageSec / 3600);
  const mins = Math.round((ageSec % 3600) / 60);
  return mins > 0 ? `${hours}h ${mins}m ago` : `${hours}h ago`;
}

export function formatSlotCountdown(
  currentSlot: number | null | undefined,
  nextReadySlot: number | null | undefined
): string {
  if (nextReadySlot == null) return "n/a";
  if (currentSlot == null) return `slot ${nextReadySlot.toLocaleString()}`;
  if (currentSlot > nextReadySlot) return "ready now";
  const remaining = nextReadySlot - currentSlot;
  return `${remaining.toLocaleString()} slots (~${slotsToTime(remaining)})`;
}

export function batchStateLabel(state: number): string {
  switch (state) {
    case 0:
      return "Fill";
    case 1:
      return "Inserted";
    case 2:
      return "Full";
    default:
      return `Unknown(${state})`;
  }
}

export function batchStateColor(state: number): string {
  switch (state) {
    case 0:
      return "bg-blue-100 text-blue-800";
    case 1:
      return "bg-green-100 text-green-800";
    case 2:
      return "bg-amber-100 text-amber-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}

export function treeTypeColor(type: string): string {
  switch (type) {
    case "StateV1":
      return "bg-purple-100 text-purple-800";
    case "StateV2":
      return "bg-indigo-100 text-indigo-800";
    case "AddressV1":
      return "bg-teal-100 text-teal-800";
    case "AddressV2":
      return "bg-cyan-100 text-cyan-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}
