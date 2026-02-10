interface StatusBadgeProps {
  label: string;
  color?: string;
}

export function StatusBadge({
  label,
  color = "bg-gray-100 text-gray-800",
}: StatusBadgeProps) {
  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${color}`}
    >
      {label}
    </span>
  );
}
