import { RepoStatus } from '../../types';
import { useContextMenu } from '../../hooks';
import { colorStyles, ColorVariant } from './colorStyles';
import { PathText } from './PathText';
import { DotsIcon } from '../icons';

/** Per-repo action callbacks and in-flight state, shared by every section. */
export interface RepoActionHandlers {
  onPull: (repoPath: string) => void;
  onClean: (repoPath: string) => void;
  onOpenInTerminal: (repoPath: string) => void;
  onOpenInEditor: (repoPath: string) => void;
  onOpenInLmsGithub: (repoPath: string) => void;
  defaultTerminalName?: string;
  defaultEditorName?: string;
  pullingRepos: Set<string>;
  cleaningRepos: Set<string>;
}

interface RepoRowProps {
  repo: RepoStatus;
  color: ColorVariant;
  muted?: boolean;
  showError?: boolean;
  showPull?: boolean;
  pullDisabled?: boolean;
  showClean?: boolean;
  handlers: RepoActionHandlers;
}

/**
 * One repository row, mirroring the macOS app: a single line with category
 * dot, repo path (muted directory + emphasized name), branch chip, and a
 * hover kebab menu with the repo actions (FRONTEND.md §5.3/§5.5).
 */
export function RepoRow({
  repo,
  color,
  muted = false,
  showError = false,
  showPull = true,
  pullDisabled = false,
  showClean = false,
  handlers,
}: RepoRowProps) {
  const menu = useContextMenu({ menuWidth: 170 });
  const styles = colorStyles[color];
  const isBusy = handlers.pullingRepos.has(repo.path) || handlers.cleaningRepos.has(repo.path);

  const menuItem = (label: string, action: () => void, disabled = false) => (
    <button
      onClick={(e) => {
        e.stopPropagation();
        menu.close();
        action();
      }}
      disabled={disabled}
      className="w-full text-left px-2.5 py-1 text-xs hover:bg-dark-borderSubtle transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
    >
      {label}
    </button>
  );

  return (
    <li
      className="group flex items-start gap-2.5 px-2 py-1 rounded-md hover:bg-dark-borderSubtle transition-colors"
      onContextMenu={(e) => {
        e.preventDefault();
        menu.open();
      }}
    >
      {/* Softer than the full-strength section-header dot. */}
      <span className={`mt-[6px] w-2 h-2 rounded-full flex-shrink-0 opacity-50 ${styles.dot}`} />

      <div className="flex-1 min-w-0">
        <PathText
          path={repo.path}
          branch={repo.branch ?? undefined}
          muted={muted}
          className="text-xs leading-5"
        />
        {showError && repo.errorMessage && (
          <p className="text-[11px] leading-4 text-accent-red/90">{repo.errorMessage}</p>
        )}
      </div>

      {isBusy ? (
        <span className="mt-[3px] w-3.5 h-3.5 flex-shrink-0 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
      ) : (
        <button
          ref={menu.buttonRef}
          onClick={(e) => {
            e.stopPropagation();
            menu.toggle();
            (e.currentTarget as HTMLButtonElement).blur();
          }}
          className={`p-0.5 rounded flex-shrink-0 hover:bg-dark-border transition-colors ${
            menu.isOpen ? 'bg-dark-border' : 'opacity-0 group-hover:opacity-100'
          }`}
          title="Actions"
        >
          <DotsIcon />
        </button>
      )}

      {menu.isOpen && menu.position && (
        <div
          ref={menu.menuRef}
          className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-0.5 min-w-[170px]"
          style={{ top: menu.position.top, left: menu.position.left }}
        >
          {handlers.defaultEditorName &&
            menuItem(`Open in ${handlers.defaultEditorName}`, () => handlers.onOpenInEditor(repo.path))}
          {handlers.defaultTerminalName &&
            menuItem(`Open in ${handlers.defaultTerminalName}`, () =>
              handlers.onOpenInTerminal(repo.path)
            )}
          {menuItem('Open in LMS Github', () => handlers.onOpenInLmsGithub(repo.path))}
          {(showPull || showClean) && <div className="my-0.5 border-t border-dark-border" />}
          {showPull && menuItem('Fetch & Pull', () => handlers.onPull(repo.path), pullDisabled)}
          {showClean && menuItem('Clean Ignored Files', () => handlers.onClean(repo.path))}
        </div>
      )}
    </li>
  );
}
