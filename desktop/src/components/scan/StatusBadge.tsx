import { colorStyles, ColorVariant } from './colorStyles';

interface StatusBadgeProps {
  count: number;
  label: string;
  color: ColorVariant;
}

/** Count pill in a category color; muted when the count is zero. */
export function StatusBadge({ count, label, color }: StatusBadgeProps) {
  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] leading-4 ${
        count === 0 ? 'bg-dark-elevated text-text-muted' : colorStyles[color].badge
      }`}
    >
      <span className="w-1.5 h-1.5 rounded-full bg-current" />
      {count} {label}
    </span>
  );
}
