import { colorStyles, ColorVariant } from './colorStyles';

interface StatusBadgeProps {
  count: number;
  label: string;
  color: ColorVariant;
}

export function StatusBadge({ count, label, color }: StatusBadgeProps) {
  return (
    <span className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded-sm ${colorStyles[color].badge}`}>
      <span className="w-1.5 h-1.5 rounded-full bg-current" />
      {count} {label}
    </span>
  );
}
