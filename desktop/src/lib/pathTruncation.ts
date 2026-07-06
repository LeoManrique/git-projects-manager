/** Directory and filename halves of a displayed path. */
export interface PathParts {
  dir: string;
  name: string;
}

/** Middle-truncate: keep both ends, collapse the middle to a single "…". */
function truncateMid(value: string, length: number): string {
  if (value.length <= length) return value;
  if (length <= 0) return '';
  if (length === 1) return '…';
  const mid = (length - 1) / 2;
  const pre = value.substring(0, Math.floor(mid));
  const post = value.substring(value.length - Math.ceil(mid));
  return `${pre}…${post}`;
}

/**
 * Truncate a path into its directory and filename parts, preferring the
 * filename. The directory shrinks first, down to a trailing "…/" bridge —
 * but never below a first-letter "/…/" hint, so a nested path can't be
 * mistaken for a root one. Only once the hint plus the full filename can't
 * fit does the filename itself middle-truncate. Building the parts here
 * (instead of splitting an already-truncated string) keeps directory
 * characters from ever being classified as filename or vice versa.
 */
export function truncatePathParts(p: string, length: number): PathParts {
  const lastSep = p.lastIndexOf('/');
  const dir = lastSep === -1 ? '' : p.substring(0, lastSep + 1);
  const name = p.substring(lastSep + 1);
  if (p.length <= length) return { dir, name };
  if (length <= 0) return { dir: '', name: '' };
  if (dir) {
    if (name.length + 3 <= length) {
      const keep = length - name.length - 2;
      return { dir: `${dir.substring(0, keep)}…/`, name };
    }
    // A dir short enough to fit within the hint's footprint shows whole.
    const hint = dir.length <= 3 ? dir : `${dir[0]}…/`;
    if (length > hint.length) {
      return { dir: hint, name: truncateMid(name, length - hint.length) };
    }
    // Trailing-slash paths have no filename to keep.
    if (!name) return { dir: '…', name: '' };
  }
  return { dir: '', name: truncateMid(name, length) };
}
