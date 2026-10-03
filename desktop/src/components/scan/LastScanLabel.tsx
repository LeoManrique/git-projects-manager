import { useEffect, useState } from 'react';
import { relativeAge } from '../../lib/relativeAge';

// setTimeout fires at once past this (about 24.8 days), so longer waits are
// slept in parts.
const MAX_TIMEOUT_MS = 2 ** 31 - 1;

/**
 * "Last scan: 3 minutes ago" beside the scan control, with the absolute time
 * in its tooltip (FRONTEND.md §5.1).
 *
 * It re-renders only when its text changes: it sleeps until the next change,
 * and catches up at once when the window comes back, in case the WebView held
 * the timer back meanwhile. Render it with `key={startedAtMs}`, so a new scan
 * starts it on a fresh clock.
 */
export function LastScanLabel({ startedAtMs }: { startedAtMs: number }) {
  const [now, setNow] = useState(Date.now);
  const { text, nextChangeMs } = relativeAge(startedAtMs, now);

  // Keyed by `now`, so every wake arms the next sleep, even one that ended a
  // moment early and left the text as it was.
  useEffect(() => {
    const delay = Math.min(Math.max(nextChangeMs - Date.now(), 0), MAX_TIMEOUT_MS);
    const timer = window.setTimeout(() => setNow(Date.now()), delay);
    return () => window.clearTimeout(timer);
  }, [now, nextChangeMs]);

  useEffect(() => {
    const catchUp = () => setNow(Date.now());
    window.addEventListener('focus', catchUp);
    document.addEventListener('visibilitychange', catchUp);
    return () => {
      window.removeEventListener('focus', catchUp);
      document.removeEventListener('visibilitychange', catchUp);
    };
  }, []);

  return (
    <span
      title={new Date(startedAtMs).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}
      className="flex-shrink-0 text-text-muted text-[11px] whitespace-nowrap cursor-default"
    >
      Last scan: {text}
    </span>
  );
}
