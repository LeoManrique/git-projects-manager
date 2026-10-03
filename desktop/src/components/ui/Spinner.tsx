const SIZES = {
  xs: 'w-3 h-3',
  sm: 'w-3.5 h-3.5',
  md: 'w-4 h-4',
  lg: 'w-5 h-5',
};

interface SpinnerProps {
  size?: keyof typeof SIZES;
  /** For a spinner on a filled accent button. */
  light?: boolean;
  /** Layout only (margins, flex), never size or color. */
  className?: string;
}

export function Spinner({ size = 'xs', light = false, className = '' }: SpinnerProps) {
  const color = light ? 'border-white/70' : 'border-text-muted';
  return (
    <span
      className={`block flex-shrink-0 ${SIZES[size]} border-2 ${color} border-t-transparent rounded-full animate-spin ${className}`}
    />
  );
}
