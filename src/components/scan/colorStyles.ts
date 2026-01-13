export type ColorVariant = 'green' | 'yellow' | 'orange' | 'purple' | 'red' | 'gray' | 'blue';

export const colorStyles: Record<ColorVariant, { badge: string; text: string; border: string; borderMuted: string }> = {
  green: {
    badge: 'bg-accent-green/10 text-accent-green',
    text: 'text-accent-green',
    border: 'border-accent-green/40',
    borderMuted: 'border-accent-green/30',
  },
  yellow: {
    badge: 'bg-accent-yellow/10 text-accent-yellow',
    text: 'text-accent-yellow',
    border: 'border-accent-yellow/40',
    borderMuted: 'border-accent-yellow/30',
  },
  orange: {
    badge: 'bg-accent-orange/10 text-accent-orange',
    text: 'text-accent-orange',
    border: 'border-accent-orange/40',
    borderMuted: 'border-accent-orange/30',
  },
  purple: {
    badge: 'bg-purple-500/10 text-purple-400',
    text: 'text-purple-400',
    border: 'border-purple-400/40',
    borderMuted: 'border-purple-400/30',
  },
  red: {
    badge: 'bg-accent-red/10 text-accent-red',
    text: 'text-accent-red',
    border: 'border-accent-red/40',
    borderMuted: 'border-accent-red/30',
  },
  gray: {
    badge: 'bg-slate-500/10 text-slate-400',
    text: 'text-slate-400',
    border: 'border-slate-400/40',
    borderMuted: 'border-slate-400/30',
  },
  blue: {
    badge: 'bg-accent-blue/10 text-accent-blue',
    text: 'text-accent-blue',
    border: 'border-accent-blue/40',
    borderMuted: 'border-accent-blue/30',
  },
};
