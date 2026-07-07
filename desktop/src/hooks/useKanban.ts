import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { api } from '../lib/api';
import { GhAuthStatus, GhRepo, KanbanCardView, KanbanState, SyncStatus } from '../types';
import { KANBAN_COLUMNS, ColumnId } from '../config/kanbanColumns';

const REFRESH_DEBOUNCE_MS = 1500;
const SYNC_USER_EVENT = 'sync-user-changed';

interface UseKanbanReturn {
  columns: Record<ColumnId, KanbanCardView[]>;
  auth: GhAuthStatus | null;
  syncStatus: SyncStatus;
  isLoading: boolean;
  isRefreshing: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  recheckAuth: () => Promise<void>;
  moveCard: (nameWithOwner: string, toColumn: ColumnId) => Promise<void>;
  deleteRepo: (nameWithOwner: string) => Promise<void>;
}

export function useKanban(): UseKanbanReturn {
  const [auth, setAuth] = useState<GhAuthStatus | null>(null);
  const [state, setState] = useState<KanbanState | null>(null);
  const [repos, setRepos] = useState<GhRepo[]>([]);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>('disabled');
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const lastRefreshRef = useRef(0);
  const inFlightRef = useRef(false);

  const doRefresh = useCallback(async () => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    setIsRefreshing(true);
    setError(null);
    try {
      const authStatus = await api.checkGhAuth();
      setAuth(authStatus);
      if (authStatus.status !== 'ok') {
        setRepos([]);
        return;
      }
      const result = await api.refreshKanban();
      setRepos(result.repos);
      setState(result.state);
      setSyncStatus(result.syncStatus);
      lastRefreshRef.current = Date.now();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      inFlightRef.current = false;
      setIsRefreshing(false);
    }
  }, []);

  const recheckAuth = useCallback(async () => {
    await doRefresh();
  }, [doRefresh]);

  // Initial load: try the on-disk cache first so the board paints instantly,
  // then revalidate in the background. Full-screen spinner only fires on a
  // genuine first launch with no cache.
  useEffect(() => {
    (async () => {
      try {
        const local = await api.loadKanbanLocal();
        if (local) {
          setRepos(local.repos);
          setState(local.state);
          setSyncStatus(local.syncStatus);
          setIsLoading(false);
          doRefresh();
        } else {
          await doRefresh();
          setIsLoading(false);
        }
      } catch {
        await doRefresh();
        setIsLoading(false);
      }
    })();
  }, [doRefresh]);

  // Refresh on window focus, debounced
  useEffect(() => {
    const onFocus = () => {
      if (Date.now() - lastRefreshRef.current < REFRESH_DEBOUNCE_MS) return;
      doRefresh();
    };
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [doRefresh]);

  // Refresh whenever sync user changes (signed in / signed out).
  useEffect(() => {
    const onAuthChange = () => { doRefresh(); };
    window.addEventListener(SYNC_USER_EVENT, onAuthChange);
    return () => window.removeEventListener(SYNC_USER_EVENT, onAuthChange);
  }, [doRefresh]);

  const columns = useMemo(() => {
    const result: Record<ColumnId, KanbanCardView[]> = {
      backlog: [],
      'active-low': [],
      'active-high': [],
      done: [],
      closed: [],
    };
    if (!state) return result;

    const repoByKey = new Map(repos.map((r) => [r.nameWithOwner, r]));

    Object.values(state.cards).forEach((card) => {
      const repo = repoByKey.get(card.nameWithOwner);
      if (!repo) return; // store sync drops these, but guard anyway
      const columnId = result[card.column] ? card.column : 'backlog';
      result[columnId].push({ card, repo });
    });

    KANBAN_COLUMNS.forEach((col) => {
      result[col.id].sort((a, b) =>
        a.repo.nameWithOwner.localeCompare(b.repo.nameWithOwner)
      );
    });

    return result;
  }, [state, repos]);

  const deleteRepo = useCallback(async (nameWithOwner: string) => {
    setError(null);
    try {
      const result = await api.deleteGithubRepo(nameWithOwner);
      setRepos(result.repos);
      setState(result.state);
      lastRefreshRef.current = Date.now();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const moveCard = useCallback(async (nameWithOwner: string, toColumn: ColumnId) => {
    // Optimistic local update so dragging feels instant.
    setState((prev) => {
      if (!prev) return prev;
      const existing = prev.cards[nameWithOwner];
      if (!existing) return prev;
      return {
        ...prev,
        cards: {
          ...prev.cards,
          [nameWithOwner]: { ...existing, column: toColumn, updatedAt: Date.now() },
        },
      };
    });
    try {
      const newState = await api.moveKanbanCard(nameWithOwner, toColumn);
      setState(newState);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      // Refetch authoritative state on failure.
      doRefresh();
    }
  }, [doRefresh]);

  return {
    columns,
    auth,
    syncStatus,
    isLoading,
    isRefreshing,
    error,
    refresh: doRefresh,
    recheckAuth,
    moveCard,
    deleteRepo,
  };
}
