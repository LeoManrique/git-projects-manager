import { RepoStatus, ScanResult } from '../../types';
import { filterRepos } from '../../lib/repoUtils';
import { ColorVariant } from './colorStyles';

/**
 * The repo categories (FRONTEND.md §5.3), mirroring the macOS app's
 * RepoCategory: display order (the transient Checking first, Clean last),
 * colors, badge labels, and which row / bulk actions each section offers.
 * Unpublished, Remote Not Found and Unknown Remote State are overlays — a repo
 * in any of them also appears in its primary section.
 */
export interface SectionSpec {
  key: string;
  title: string;
  badgeLabel: string;
  color: ColorVariant;
  muted: boolean;
  /** Shown only while the folder scans visibly. */
  onlyWhileScanning: boolean;
  showError: boolean;
  showPull: boolean;
  pullDisabled: boolean;
  showClean: boolean;
  hasBulkPull: boolean;
  hasBulkClean: boolean;
  repos: (result: ScanResult) => RepoStatus[];
}

const defaults = {
  muted: false,
  onlyWhileScanning: false,
  showError: false,
  showPull: true,
  pullDisabled: false,
  showClean: false,
  hasBulkPull: false,
  hasBulkClean: false,
};

const SECTIONS: SectionSpec[] = [
  {
    ...defaults,
    key: 'checking',
    title: 'Checking',
    badgeLabel: 'checking',
    color: 'gray',
    muted: true,
    // Repos a scan found but has no status for yet, so a silent scan keeps
    // them out of sight until their status lands.
    onlyWhileScanning: true,
    showPull: false,
    repos: (r) => r.checking,
  },
  {
    ...defaults,
    key: 'changes',
    title: 'Uncommitted Changes',
    badgeLabel: 'changed',
    color: 'yellow',
    pullDisabled: true,
    repos: (r) => r.withChanges,
  },
  {
    ...defaults,
    key: 'unpushed',
    title: 'Unpushed Commits',
    badgeLabel: 'unpushed',
    color: 'orange',
    repos: (r) => r.withUnpushed,
  },
  {
    ...defaults,
    key: 'unpulled',
    title: 'Unpulled Commits',
    badgeLabel: 'unpulled',
    color: 'purple',
    hasBulkPull: true,
    repos: (r) => r.withUnpulled,
  },
  {
    ...defaults,
    key: 'unpublished',
    title: 'Unpublished',
    badgeLabel: 'unpublished',
    color: 'blue',
    showPull: false,
    repos: (r) => r.unpublished,
  },
  {
    ...defaults,
    key: 'remoteNotFound',
    title: 'Remote Not Found',
    badgeLabel: 'remote gone',
    color: 'pink',
    showPull: false,
    repos: (r) => r.remoteNotFound,
  },
  {
    ...defaults,
    key: 'remoteStateUnknown',
    title: 'Unknown Remote State',
    badgeLabel: 'unknown',
    color: 'gray',
    // Pull is still offered: the comparison failed, but pulling is exactly how
    // a user would resolve it, and `git pull` reports its own errors.
    repos: (r) => r.remoteStateUnknown,
  },
  {
    ...defaults,
    key: 'uninitialized',
    title: 'Uninitialized',
    badgeLabel: 'uninitialized',
    color: 'gray',
    muted: true,
    showPull: false,
    repos: (r) => r.uninitialized,
  },
  {
    ...defaults,
    key: 'errors',
    title: 'Errors',
    badgeLabel: 'errors',
    color: 'red',
    showError: true,
    pullDisabled: true,
    repos: (r) => r.errors,
  },
  {
    ...defaults,
    key: 'clean',
    title: 'Clean',
    badgeLabel: 'clean',
    color: 'green',
    muted: true,
    showClean: true,
    hasBulkClean: true,
    repos: (r) => r.clean,
  },
];

/**
 * A folder's sections with their search-filtered repos, empty ones dropped
 * (FRONTEND.md §5.4: sections filtered to zero disappear), and Checking only
 * while `isScanning` (the folder scans visibly).
 */
export function visibleSections(
  result: ScanResult,
  query: string,
  isScanning: boolean
): [SectionSpec, RepoStatus[]][] {
  return SECTIONS.filter((spec) => isScanning || !spec.onlyWhileScanning)
    .map((spec): [SectionSpec, RepoStatus[]] => [spec, filterRepos(spec.repos(result), query)])
    .filter(([, repos]) => repos.length > 0);
}
