export type ColorVariant = 'green' | 'yellow' | 'orange' | 'purple' | 'red' | 'gray' | 'blue';

export const colorStyles: Record<ColorVariant, { badge: string; text: string; dot: string; border: string }> = {
  green: {
    badge: 'bg-accent-green/10 text-accent-green',
    text: 'text-accent-green',
    dot: 'bg-accent-green',
    border: 'border-accent-green/40',
  },
  yellow: {
    badge: 'bg-accent-yellow/10 text-accent-yellow',
    text: 'text-accent-yellow',
    dot: 'bg-accent-yellow',
    border: 'border-accent-yellow/40',
  },
  orange: {
    badge: 'bg-accent-orange/10 text-accent-orange',
    text: 'text-accent-orange',
    dot: 'bg-accent-orange',
    border: 'border-accent-orange/40',
  },
  purple: {
    badge: 'bg-purple-500/10 text-purple-400',
    text: 'text-purple-400',
    dot: 'bg-purple-400',
    border: 'border-purple-400/40',
  },
  red: {
    badge: 'bg-accent-red/10 text-accent-red',
    text: 'text-accent-red',
    dot: 'bg-accent-red',
    border: 'border-accent-red/40',
  },
  gray: {
    badge: 'bg-slate-500/10 text-slate-400',
    text: 'text-slate-400',
    dot: 'bg-slate-400',
    border: 'border-slate-400/40',
  },
  blue: {
    badge: 'bg-accent-blue/10 text-accent-blue',
    text: 'text-accent-blue',
    dot: 'bg-accent-blue',
    border: 'border-accent-blue/40',
  },
};
