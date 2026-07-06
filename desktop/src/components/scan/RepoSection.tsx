import { RepoStatus } from '../../types';
import { useContextMenu } from '../../hooks';
import { colorStyles } from './colorStyles';
import { SectionSpec } from './sections';
import { RepoRow, RepoActionHandlers } from './RepoRow';
import { DotsIcon } from '../icons';

export interface RepoSectionProps {
  spec: SectionSpec;
  /** Already search-filtered (§5.4); bulk actions operate on this list. */
  repos: RepoStatus[];
  handlers: RepoActionHandlers;
  onPullAll: (repos: RepoStatus[]) => void;
  onCleanAll: (repos: RepoStatus[]) => void;
  isBulkPulling: boolean;
  isBulkCleaning: boolean;
}

/**
 * One category section, macOS-style: colored `Title (count)` header with the
 * bulk-action kebab, above the repo rows. Empty sections render nothing.
 */
export function RepoSection({
  spec,
  repos,
  handlers,
  onPullAll,
  onCleanAll,
  isBulkPulling,
  isBulkCleaning,
}: RepoSectionProps) {
  const bulkMenu = useContextMenu({ menuWidth: 170 });

  if (repos.length === 0) return null;

  const styles = colorStyles[spec.color];
  const hasBulk = spec.hasBulkPull || spec.hasBulkClean;
  const bulkInFlight = (spec.hasBulkPull && isBulkPulling) || (spec.hasBulkClean && isBulkCleaning);

  return (
    <section>
      <div className="flex items-center justify-between px-2 mb-0.5">
        <h4 className={`${styles.text} text-xs font-semibold flex items-center gap-1.5`}>
          <span className="w-2 h-2 rounded-full bg-current" />
          {spec.title} ({repos.length})
        </h4>
        {hasBulk && (
          <>
            <button
              ref={bulkMenu.buttonRef}
              onClick={bulkMenu.toggle}
              disabled={bulkInFlight}
              className={`p-1 rounded hover:bg-dark-border transition-colors ${bulkMenu.isOpen ? 'bg-dark-border' : ''}`}
              title="Section actions"
            >
              {bulkInFlight ? (
                <span className="w-3.5 h-3.5 block border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
              ) : (
                <DotsIcon />
              )}
            </button>
            {bulkMenu.isOpen && bulkMenu.position && (
              <div
                ref={bulkMenu.menuRef}
                className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-0.5 min-w-[170px]"
                style={{ top: bulkMenu.position.top, left: bulkMenu.position.left }}
              >
                <button
                  onClick={() => {
                    bulkMenu.close();
                    if (spec.hasBulkPull) onPullAll(repos);
                    else onCleanAll(repos);
                  }}
                  className="w-full text-left px-2.5 py-1 text-xs hover:bg-dark-borderSubtle transition-colors"
                >
                  {spec.hasBulkPull ? `Fetch & Pull All (${repos.length})` : `Clean All (${repos.length})`}
                </button>
              </div>
            )}
          </>
        )}
      </div>
      <ul>
        {repos.map((repo) => (
          <RepoRow
            key={repo.path}
            repo={repo}
            color={spec.color}
            muted={spec.muted}
            showError={spec.showError}
            showPull={spec.showPull}
            pullDisabled={spec.pullDisabled}
            showClean={spec.showClean}
            handlers={handlers}
          />
        ))}
      </ul>
    </section>
  );
}
