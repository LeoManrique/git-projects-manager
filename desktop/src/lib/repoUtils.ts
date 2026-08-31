import { MonitoredFolder, RepoStatus, ScanResult } from '../types';

/** Last path segment of a repo path, used as its display name. */
export function repoName(path: string): string {
  return path.split('/').filter(Boolean).pop() ?? path;
}

/**
 * Search filter (FRONTEND.md §5.4): case-insensitive substring match of the
 * trimmed query against the repo name or full path.
 */
export function filterRepos(repos: RepoStatus[], query: string): RepoStatus[] {
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) return repos;
  return repos.filter(
    (repo) =>
      repoName(repo.path).toLowerCase().includes(trimmed) ||
      repo.path.toLowerCase().includes(trimmed)
  );
}

/** Repos needing attention (changed + unpushed + unpulled + errors). */
export function attentionCount(result: ScanResult | undefined): number {
  if (!result) return 0;
  return (
    result.withChanges.length +
    result.withUnpushed.length +
    result.withUnpulled.length +
    result.errors.length
  );
}

/**
 * Label for the "only local checks" indicator, or `null` when every folder in
 * view is scanned online (FRONTEND.md §5.4).
 *
 * Without it, a local-only folder reports no unpushed or unpulled commits for
 * every repo simply because it never asked — indistinguishable, on screen, from
 * genuinely being up to date.
 */
export function localOnlyNotice(folders: MonitoredFolder[]): string | null {
  const localOnly = folders.filter((f) => f.onlyLocalChecks).length;
  if (localOnly === 0) return null;
  if (localOnly === folders.length) return 'Local checks only';
  return `${localOnly} of ${folders.length} folders: local checks only`;
}
