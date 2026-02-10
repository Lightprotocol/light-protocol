interface ProgressBarProps {
  value: number;
  max?: number;
  className?: string;
  barColor?: string;
}

export function ProgressBar({
  value,
  max = 100,
  className = "",
  barColor,
}: ProgressBarProps) {
  const pct = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  const color =
    barColor ?? (pct > 90 ? "bg-red-500" : pct > 70 ? "bg-amber-500" : "bg-blue-500");

  return (
    <div
      className={`w-full bg-gray-200 rounded-full h-2 overflow-hidden ${className}`}
    >
      <div
        className={`h-full rounded-full transition-all ${color}`}
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}
