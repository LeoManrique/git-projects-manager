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

// Named long form, matching macOS `.relative(presentation: .named)`.
function formatRelative(iso: string | null): string | null {
  if (!iso) return null;
  const ts = Date.parse(iso);
  if (Number.isNaN(ts)) return null;
  const diff = Date.now() - ts;
  const day = 86_400_000;
  const ago = (n: number, unit: string) => `${n} ${unit}${n === 1 ? '' : 's'} ago`;
  if (diff < day) return 'today';
  if (diff < 2 * day) return 'yesterday';
  if (diff < 7 * day) return ago(Math.floor(diff / day), 'day');
  if (diff < 30 * day) return ago(Math.floor(diff / (7 * day)), 'week');
  if (diff < 365 * day) return ago(Math.floor(diff / (30 * day)), 'month');
  return ago(Math.floor(diff / (365 * day)), 'year');
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
          rounded-[10px] bg-dark-elevated border
          cursor-grab active:cursor-grabbing
          transition-all duration-150
          ${
            isDragging
              ? 'opacity-50 shadow-lg border-dark-border'
              : 'border-dark-border hover:border-dark-borderStrong hover:shadow-md'
          }
        `}
      >
        <div className="px-3 pt-2 flex items-center gap-2">
          <span className="text-sm font-medium text-text-primary truncate flex-1">
            {repo.name}
          </span>
          {repo.isArchived && (
            <span className="shrink-0 text-[10px] text-accent-yellow uppercase tracking-wide">
              archived
            </span>
          )}
          <button
            ref={menu.buttonRef}
            onClick={(e) => {
              e.stopPropagation();
              menu.toggle();
              menu.buttonRef.current?.blur();
            }}
            onMouseDown={(e) => e.stopPropagation()}
            className={`
              shrink-0 -mr-1 p-1 rounded
              text-text-muted hover:text-text-primary hover:bg-dark-borderSubtle
              transition-opacity
              ${showActions ? 'opacity-100' : 'opacity-0 pointer-events-none'}
            `}
            aria-label="Card actions"
          >
            <DotsIcon />
          </button>
        </div>
        <div className="px-3 pb-2 flex items-center gap-1.5">
          <span className="text-[11px] text-text-secondary truncate">{repo.owner.login}</span>
          {repo.isPrivate && (
            <span className="shrink-0 text-text-muted" title="Private">
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
          <span className="flex-1" />
          {pushed && (
            <span className="shrink-0 text-[11px] text-text-muted">{pushed}</span>
          )}
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
            <>
              <div className="border-t border-dark-border my-1" />
              <button
                onClick={handleDelete}
                className="w-full text-left px-3 py-1.5 text-xs hover:bg-accent-red/15 transition-colors text-accent-red"
              >
                Delete Repository…
              </button>
            </>
          )}
        </div>
      )}
    </>
  );
}
