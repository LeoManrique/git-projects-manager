import { useRef, useState, type ComponentType } from 'react';
import { useKanban } from '../../hooks/useKanban';
import { useAuth } from '../../hooks/useAuth';
import { useContextMenu } from '../../hooks';
import { KanbanColumn } from './KanbanColumn';
import { KANBAN_COLUMNS, ColumnId } from '../../config/kanbanColumns';
import { GhAuthStatus, SyncStatus } from '../../types';
import { CloudIcon, CloudCheckIcon, CloudSlashIcon, CloudAlertIcon } from '../icons';

export function KanbanBoard() {
  const {
    columns,
    auth,
    syncStatus,
    isLoading,
    isRefreshing,
    error,
    refresh,
    recheckAuth,
    moveCard,
    deleteRepo,
  } = useKanban();
  const authedUser = auth?.status === 'ok' ? auth.user : null;

  // dataTransfer.getData() returns empty in Tauri builds — keep the dragged
  // item in a ref so onDrop can read it.
  const draggingItemRef = useRef<string | null>(null);

  const handleDrop = (toColumn: ColumnId) => {
    const nwo = draggingItemRef.current;
    draggingItemRef.current = null;
    if (!nwo) return;

    const currentColumn = (Object.entries(columns).find(([, cards]) =>
      cards.some((c) => c.card.nameWithOwner === nwo)
    )?.[0]) as ColumnId | undefined;

    if (currentColumn && currentColumn !== toColumn) {
      moveCard(nwo, toColumn);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary">
        <div className="text-sm">Loading…</div>
      </div>
    );
  }

  if (auth && auth.status !== 'ok') {
    return <GhEmptyState auth={auth} onRecheck={recheckAuth} />;
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-4 pt-3">
        <div className="flex-1 min-w-0">
          {error && <div className="text-xs text-accent-red truncate">{error}</div>}
        </div>
        <div className="flex items-center gap-2">
          {auth?.status === 'ok' && (
            <span className="text-xs text-text-muted">gh: {auth.user || 'authenticated'}</span>
          )}
          <SyncStatusChip syncStatus={syncStatus} />
          <button
            onClick={refresh}
            disabled={isRefreshing}
            className="text-xs px-2.5 py-1 rounded-md border border-dark-border text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle transition-colors disabled:opacity-50"
          >
            {isRefreshing ? 'Refreshing…' : 'Refresh'}
          </button>
        </div>
      </div>
      <div className="flex gap-3 p-4 flex-1 overflow-hidden">
        {KANBAN_COLUMNS.map((column) => (
          <KanbanColumn
            key={column.id}
            column={column}
            cards={columns[column.id]}
            authedUser={authedUser}
            onDragStart={(nwo) => (draggingItemRef.current = nwo)}
            onDragEnd={() => (draggingItemRef.current = null)}
            onDrop={() => handleDrop(column.id)}
            onDeleteRepo={deleteRepo}
          />
        ))}
      </div>
    </div>
  );
}

const SYNC_CHIP: Record<
  SyncStatus,
  { label: string; classes: string; Icon: ComponentType<{ className?: string }> }
> = {
  disabled: { label: 'Sync off', classes: 'text-text-muted border-dark-border', Icon: CloudSlashIcon },
  synced: {
    label: 'Synced',
    classes: 'text-accent-green bg-accent-green/10 border-accent-green/40',
    Icon: CloudCheckIcon,
  },
  offline: {
    label: 'Offline',
    classes: 'text-accent-yellow bg-accent-yellow/10 border-accent-yellow/40',
    Icon: CloudIcon,
  },
  expired: {
    label: 'Session expired',
    classes: 'text-accent-orange bg-accent-orange/10 border-accent-orange/40',
    Icon: CloudAlertIcon,
  },
};

function SyncStatusChip({ syncStatus }: { syncStatus: SyncStatus }) {
  const { user, status, signIn, signOut } = useAuth();
  const menu = useContextMenu({ menuWidth: 220 });
  const [isSigningIn, setIsSigningIn] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const { label, classes, Icon } = SYNC_CHIP[syncStatus];

  const handleToggle = () => {
    setActionError(null);
    menu.toggle();
    menu.buttonRef.current?.blur();
  };

  const handleSignIn = async () => {
    setActionError(null);
    setIsSigningIn(true);
    try {
      await signIn();
      menu.close();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsSigningIn(false);
    }
  };

  const handleSignOut = async () => {
    setActionError(null);
    try {
      await signOut();
      menu.close();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <>
      <button
        ref={menu.buttonRef}
        onClick={handleToggle}
        className={`flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-full border transition-colors ${classes}`}
        aria-label="Sync status"
      >
        <Icon />
        {label}
      </button>

      {menu.isOpen && menu.position && (
        <div
          ref={menu.menuRef}
          className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-1 min-w-[220px]"
          style={{ top: menu.position.top, left: menu.position.left }}
        >
          {status === 'signed-in' && user ? (
            <>
              <div className="px-3 py-1.5">
                <div className="text-xs text-text-primary truncate">
                  {user.name || user.email || user.sub}
                </div>
                {user.name && user.email && (
                  <div className="text-[11px] text-text-muted truncate">{user.email}</div>
                )}
              </div>
              <div className="border-t border-dark-border my-1" />
              <button
                onClick={handleSignOut}
                className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors text-text-primary"
              >
                Sign out
              </button>
            </>
          ) : (
            <button
              onClick={handleSignIn}
              disabled={isSigningIn}
              className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors text-text-primary disabled:opacity-50 disabled:hover:bg-transparent"
            >
              {isSigningIn ? 'Waiting for browser…' : 'Sign in with Google…'}
            </button>
          )}
          {actionError && (
            <div className="px-3 py-1.5 text-[11px] text-accent-red break-words">{actionError}</div>
          )}
        </div>
      )}
    </>
  );
}

function GhEmptyState({
  auth,
  onRecheck,
}: {
  auth: GhAuthStatus;
  onRecheck: () => void;
}) {
  const headline =
    auth.status === 'notInstalled'
      ? 'GitHub CLI not found'
      : auth.status === 'notAuthenticated'
        ? 'GitHub CLI not authenticated'
        : 'GitHub CLI error';

  const body =
    auth.status === 'notInstalled' ? (
      <>
        Install <code className="text-text-primary">gh</code> from{' '}
        <span className="text-text-primary">cli.github.com</span>, then click Recheck.
      </>
    ) : auth.status === 'notAuthenticated' ? (
      <>
        Run <code className="text-text-primary">gh auth login</code> in a terminal, then click
        Recheck.
      </>
    ) : (
      <span className="text-accent-red break-words">
        {auth.status === 'error' ? auth.message : 'Unknown error'}
      </span>
    );

  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center p-8 max-w-md">
        <div className="text-text-muted mb-3">
          <svg
            className="w-12 h-12 mx-auto"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
          </svg>
        </div>
        <h3 className="text-text-primary text-sm font-medium mb-2">{headline}</h3>
        <p className="text-text-secondary text-xs mb-4">{body}</p>
        <button
          onClick={onRecheck}
          className="text-xs px-3 py-1.5 rounded border border-dark-border text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle transition-colors"
        >
          Recheck
        </button>
      </div>
    </div>
  );
}
