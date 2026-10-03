const DAY_MS = 86_400_000;

/**
 * Smallest first. A 30-day month and a 365-day year: the label answers
 * "roughly how long ago", and the exact time is in its tooltip.
 */
const UNITS: { name: string; ms: number }[] = [
  { name: 'minute', ms: 60_000 },
  { name: 'hour', ms: 3_600_000 },
  { name: 'day', ms: DAY_MS },
  { name: 'month', ms: 30 * DAY_MS },
  { name: 'year', ms: 365 * DAY_MS },
];

export interface RelativeAge {
  text: string;
  /** When `text` next changes, unix ms. */
  nextChangeMs: number;
}

/**
 * How long ago `dateMs` was, in leogit's vocabulary ("just now", "5 minutes
 * ago", "2 years ago"), and the instant that text next changes, so a label can
 * sleep until then instead of polling (FRONTEND.md §5.1).
 */
export function relativeAge(dateMs: number, nowMs: number): RelativeAge {
  // A date in the future (a clock set back) reads as "just now".
  const elapsed = Math.max(nowMs - dateMs, 0);
  let index = UNITS.length - 1;
  while (index >= 0 && elapsed < UNITS[index].ms) index--;
  if (index < 0) return { text: 'just now', nextChangeMs: dateMs + UNITS[0].ms };
  const unit = UNITS[index];
  const count = Math.floor(elapsed / unit.ms);
  // The next count of this unit, or the next unit if it comes first (12 months
  // ago turns into 1 year ago at 365 days, not 390).
  let untilNext = (count + 1) * unit.ms;
  if (index + 1 < UNITS.length) untilNext = Math.min(untilNext, UNITS[index + 1].ms);
  return {
    text: `${count} ${unit.name}${count === 1 ? '' : 's'} ago`,
    nextChangeMs: dateMs + untilNext,
  };
}
