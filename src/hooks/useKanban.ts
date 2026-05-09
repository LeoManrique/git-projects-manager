import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { api } from '../lib/api';
import { GhAuthStatus, GhRepo, KanbanCardView, KanbanState } from '../types';
import { KANBAN_COLUMNS, ColumnId } from '../config/kanbanColumns';

const REFRESH_DEBOUNCE_MS = 1500;

interface UseKanbanReturn {
  columns: Record<ColumnId, KanbanCardView[]>;
  auth: GhAuthStatus | null;
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
      lastRefreshRef.current = Date.now();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      inFlightRef.current = false;
      setIsRefreshing(false);
    }
  }, []);

  const recheckAuth = useCallback(async () => {
    setIsLoading(true);
    await doRefresh();
    setIsLoading(false);
  }, [doRefresh]);

  // Initial load
  useEffect(() => {
    (async () => {
      await doRefresh();
      setIsLoading(false);
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

  const columns = useMemo(() => {
    const result: Record<ColumnId, KanbanCardView[]> = {
      backlog: [],
      'active-low': [],
      'active-high': [],
      review: [],
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
    isLoading,
    isRefreshing,
    error,
    refresh: doRefresh,
    recheckAuth,
    moveCard,
    deleteRepo,
  };
}
