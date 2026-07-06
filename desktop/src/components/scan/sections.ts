import { RepoStatus, ScanResult } from '../../types';
import { filterRepos } from '../../lib/repoUtils';
import { ColorVariant } from './colorStyles';

/**
 * The six fixed repo categories (FRONTEND.md §5.3), mirroring the macOS
 * app's RepoCategory: display order (Clean last), colors, badge labels, and
 * which row / bulk actions each section offers.
 */
export interface SectionSpec {
  key: string;
  title: string;
  badgeLabel: string;
  color: ColorVariant;
  muted: boolean;
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
  showError: false,
  showPull: true,
  pullDisabled: false,
  showClean: false,
  hasBulkPull: false,
  hasBulkClean: false,
};

export const SECTIONS: SectionSpec[] = [
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
 * The given sections with their search-filtered repos, empty ones dropped
 * (FRONTEND.md §5.4: sections filtered to zero disappear).
 */
export function visibleSections(
  specs: SectionSpec[],
  result: ScanResult,
  query: string
): [SectionSpec, RepoStatus[]][] {
  return specs
    .map((spec): [SectionSpec, RepoStatus[]] => [spec, filterRepos(spec.repos(result), query)])
    .filter(([, repos]) => repos.length > 0);
}
