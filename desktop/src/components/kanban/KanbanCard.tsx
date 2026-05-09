import { useState } from 'react';
import { confirm } from '@tauri-apps/plugin-dialog';
import { KanbanCardView } from '../../types';
import { useContextMenu } from '../../hooks';
import { DotsIcon } from '../icons';
import { api } from '../../lib/api';

interface KanbanCardProps {
  cardView: KanbanCardView;
  authedUser: string | null;
  onDragStart: (nameWithOwner: string) => void;
  onDragEnd: () => void;
  onDeleteRepo: (nameWithOwner: string) => void;
}

function formatRelative(iso: string | null): string | null {
  if (!iso) return null;
  const ts = Date.parse(iso);
  if (Number.isNaN(ts)) return null;
  const diff = Date.now() - ts;
  const day = 86_400_000;
  if (diff < day) return 'today';
  if (diff < 7 * day) return `${Math.floor(diff / day)}d ago`;
  if (diff < 30 * day) return `${Math.floor(diff / (7 * day))}w ago`;
  if (diff < 365 * day) return `${Math.floor(diff / (30 * day))}mo ago`;
  return `${Math.floor(diff / (365 * day))}y ago`;
}

export function KanbanCard({
  cardView,
  authedUser,
  onDragStart,
  onDragEnd,
  onDeleteRepo,
}: KanbanCardProps) {
  const { card, repo } = cardView;
  const [isDragging, setIsDragging] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const pushed = formatRelative(repo.pushedAt);
  const menu = useContextMenu({ menuWidth: 180 });
  const showActions = isHovered || menu.isOpen;
  const canDelete =
    authedUser !== null &&
    authedUser.toLowerCase() === repo.owner.login.toLowerCase();

  const handleDragStart = (e: React.DragEvent) => {
    e.dataTransfer.effectAllowed = 'move';
    onDragStart(card.nameWithOwner);
    setIsDragging(true);
  };

  const handleDragEnd = () => {
    setIsDragging(false);
    onDragEnd();
  };

  const handleView = async () => {
    menu.close();
    try {
      await api.openUrl(repo.url);
    } catch {
      /* surfaced upstream via error state if needed */
    }
  };

  const handleDelete = async () => {
    menu.close();
    const ok = await confirm(
      `This will permanently delete the GitHub repository "${repo.nameWithOwner}".\n\nThis cannot be undone.`,
      { title: 'Delete repository?', kind: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }
    );
    if (ok) onDeleteRepo(card.nameWithOwner);
  };

  return (
    <>
      <div
        draggable={!menu.isOpen}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        title={repo.description ?? repo.nameWithOwner}
        className={`
          relative rounded border bg-dark-elevated border-dark-border
          cursor-grab active:cursor-grabbing
          ${isDragging ? 'opacity-50 shadow-lg' : ''}
          transition-opacity duration-150
        `}
      >
        <button
          ref={menu.buttonRef}
          onClick={(e) => {
            e.stopPropagation();
            menu.toggle();
            menu.buttonRef.current?.blur();
          }}
          onMouseDown={(e) => e.stopPropagation()}
          className={`
            absolute top-1.5 right-1.5 p-1 rounded
            text-text-muted hover:text-text-primary hover:bg-dark-borderSubtle
            transition-opacity
            ${showActions ? 'opacity-100' : 'opacity-0 pointer-events-none'}
          `}
          aria-label="Card actions"
        >
          <DotsIcon />
        </button>

        <div className="px-3 py-2 pr-8 flex items-center gap-2">
          <span className="text-sm truncate flex-1 text-text-primary">{repo.name}</span>
          {repo.isArchived && (
            <span className="text-[10px] text-accent-yellow uppercase tracking-wide">archived</span>
          )}
        </div>
        <div className="px-3 pb-2 flex items-center justify-between text-[11px] text-text-muted">
          <span className="flex items-center gap-1.5 truncate">
            <span className="shrink-0" title="GitHub">
              <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
              </svg>
            </span>
            <span className="truncate">{repo.owner.login}</span>
            {repo.isPrivate && (
              <span className="shrink-0" title="Private">
                <svg
                  className="w-3.5 h-3.5"
                  viewBox="0 0 32 32"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeMiterlimit={10}
                >
                  <rect x="7" y="14" width="18" height="14" />
                  <path d="M22,14v-4c0-3.3-2.7-6-6-6h0c-3.3,0-6,2.7-6,6v4" />
                </svg>
              </span>
            )}
          </span>
          {pushed && <span className="ml-2 shrink-0">{pushed}</span>}
        </div>
      </div>

      {menu.isOpen && menu.position && (
        <div
          ref={menu.menuRef}
          className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-1 min-w-[180px]"
          style={{ top: menu.position.top, left: menu.position.left }}
        >
          <button
            onClick={handleView}
            className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors text-text-primary"
          >
            View on GitHub
          </button>
          {canDelete && (
            <button
              onClick={handleDelete}
              className="w-full text-left px-3 py-1.5 text-xs hover:bg-accent-red/15 transition-colors text-accent-red"
            >
              Delete Repository
            </button>
          )}
        </div>
      )}
    </>
  );
}
