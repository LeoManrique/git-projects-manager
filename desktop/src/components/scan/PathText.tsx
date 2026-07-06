import { useLayoutEffect, useRef, useState } from 'react';
import { truncatePathParts } from '../../lib/pathTruncation';

const TRAILING_PAD = 2;
const CHIP_GAP = 8; // gap-2

interface PathTextProps {
  path: string;
  /** Branch rendered as a trailing accent chip. */
  branch?: string;
  /** Muted rows (clean/uninitialized) dim the name too. */
  muted?: boolean;
  /** Font-size classes; layout/overflow classes are added here. */
  className?: string;
}

/**
 * Single-line repo path: muted directory + emphasized repo name, followed by
 * an optional branch chip (FRONTEND.md §5.3). Truncates directory-first: the
 * rendered width is measured with a hidden span and the longest fitting
 * display is found by binary search, so the repo name stays visible until
 * the directory has shrunk to its "…/" hint.
 */
export function PathText({ path, branch, muted = false, className = '' }: PathTextProps) {
  const containerRef = useRef<HTMLParagraphElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const chipRef = useRef<HTMLSpanElement>(null);
  const [displayLength, setDisplayLength] = useState<number | null>(null);

  useLayoutEffect(() => {
    const container = containerRef.current;
    const measure = measureRef.current;
    if (!container || !measure) return;

    const measureWidth = (text: string): number => {
      measure.textContent = text;
      return measure.getBoundingClientRect().width;
    };
    const displayAt = (length: number): string => {
      const { dir, name } = truncatePathParts(path, length);
      return dir + name;
    };

    const fit = () => {
      const chip = chipRef.current;
      const reserved = chip ? chip.offsetWidth + CHIP_GAP : 0;
      const available = container.clientWidth - reserved - TRAILING_PAD;
      if (available <= 0) return;
      if (measureWidth(path) <= available) {
        setDisplayLength(null);
        return;
      }
      let lo = 1;
      let hi = path.length;
      let best = 1;
      while (lo <= hi) {
        const mid = Math.floor((lo + hi) / 2);
        if (measureWidth(displayAt(mid)) <= available) {
          best = mid;
          lo = mid + 1;
        } else {
          hi = mid - 1;
        }
      }
      setDisplayLength(best);
    };

    const observer = new ResizeObserver(fit);
    observer.observe(container);
    fit();
    return () => observer.disconnect();
  }, [path, branch]);

  const { dir, name } = truncatePathParts(path, displayLength ?? path.length);
  const displayed = dir + name;

  return (
    <p
      ref={containerRef}
      className={`relative flex items-center gap-2 min-w-0 overflow-hidden whitespace-nowrap font-mono ${className}`}
      title={displayed.length < path.length ? path : undefined}
    >
      <span className="flex-shrink-0">
        <span className="text-text-muted">{dir}</span>
        <span className={`font-medium ${muted ? 'text-text-secondary' : 'text-text-primary'}`}>
          {name}
        </span>
      </span>
      {branch && (
        <span
          ref={chipRef}
          className="flex-shrink-0 max-w-40 truncate font-sans px-1.5 rounded-full bg-accent-blue/10 text-accent-blue text-[11px] leading-4"
        >
          {branch}
        </span>
      )}
      <span
        ref={measureRef}
        className="absolute top-0 left-0 invisible whitespace-nowrap"
        aria-hidden="true"
      />
    </p>
  );
}
