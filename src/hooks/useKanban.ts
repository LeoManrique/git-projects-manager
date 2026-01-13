import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { api } from '../lib/api';
import { KanbanState, KanbanCardView, ScanResult, MonitoredFolder } from '../types';
import { KANBAN_COLUMNS, ColumnId } from '../config/kanbanColumns';

interface UseKanbanProps {
  scanResults: Record<string, ScanResult>;
  folders: MonitoredFolder[];
}

interface UseKanbanReturn {
  columns: Record<ColumnId, KanbanCardView[]>;
  isLoading: boolean;
  error: string | null;
  moveCard: (repoPath: string, toColumn: ColumnId) => Promise<void>;
  updateNotes: (repoPath: string, notes: string | null) => Promise<void>;
  removeCard: (repoPath: string) => Promise<void>;
  syncWithScans: () => Promise<void>;
}

export function useKanban({ scanResults, folders }: UseKanbanProps): UseKanbanReturn {
  const [kanbanState, setKanbanState] = useState<KanbanState | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const hasSyncedRef = useRef(false);

  // Get all repo paths from scan results
  const allRepoPaths = useMemo(() => {
    const paths: string[] = [];
    Object.values(scanResults).forEach((result) => {
      const allRepos = [
        ...result.clean,
        ...result.withChanges,
        ...result.withUnpushed,
        ...result.withUnpulled,
        ...result.errors,
      ];
      allRepos.forEach((repo) => paths.push(repo.path));
    });
    return paths;
  }, [scanResults]);

  // Load initial state
  useEffect(() => {
    api
      .getKanbanState()
      .then(setKanbanState)
      .catch((err) => setError(err instanceof Error ? err.message : String(err)))
      .finally(() => setIsLoading(false));
  }, []);

  // Sync when repos are available (handles initial load and subsequent changes)
  useEffect(() => {
    // Skip if still loading initial state or no repos
    if (isLoading || allRepoPaths.length === 0) return;

    const doSync = async () => {
      try {
        const newState = await api.syncKanbanWithRepos(allRepoPaths);
        setKanbanState(newState);
        hasSyncedRef.current = true;
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    };

    doSync();
  }, [isLoading, allRepoPaths]);

  // Detect name conflicts
  const nameConflicts = useMemo(() => {
    const nameCounts: Record<string, number> = {};
    allRepoPaths.forEach((path) => {
      const name = path.split('/').pop() || '';
      nameCounts[name] = (nameCounts[name] || 0) + 1;
    });
    return new Set(Object.keys(nameCounts).filter((name) => nameCounts[name] > 1));
  }, [allRepoPaths]);

  // Build card views organized by column
  const columns = useMemo(() => {
    const result: Record<ColumnId, KanbanCardView[]> = {
      backlog: [],
      'active-low': [],
      'active-mid': [],
      'active-high': [],
      review: [],
      done: [],
    };

    if (!kanbanState) return result;

    Object.values(kanbanState.cards).forEach((card) => {
      const repoName = card.repoPath.split('/').pop() || '';
      const folder = folders.find((f) => card.repoPath.startsWith(f.path));
      const isStale = !allRepoPaths.includes(card.repoPath);

      // Determine display name (with folder prefix if conflict)
      const hasConflict = nameConflicts.has(repoName);
      const displayName = hasConflict && folder ? `${folder.name}/${repoName}` : repoName;

      const cardView: KanbanCardView = {
        card,
        repoName,
        displayName,
        folderName: folder?.name || '',
        folderId: folder?.id || '',
        isStale,
      };

      const columnId = card.column as ColumnId;
      // Display in the card's column if it exists, otherwise fall back to backlog
      const displayColumn = result[columnId] ? columnId : 'backlog';
      result[displayColumn].push(cardView);
    });

    // Sort each column alphabetically by displayName
    KANBAN_COLUMNS.forEach((col) => {
      result[col.id].sort((a, b) => a.displayName.localeCompare(b.displayName));
    });

    return result;
  }, [kanbanState, folders, allRepoPaths, nameConflicts]);

  // Actions
  const moveCard = useCallback(async (repoPath: string, toColumn: ColumnId) => {
    try {
      const newState = await api.moveKanbanCard(repoPath, toColumn);
      setKanbanState(newState);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const updateNotes = useCallback(async (repoPath: string, notes: string | null) => {
    try {
      const newState = await api.updateKanbanNotes(repoPath, notes);
      setKanbanState(newState);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const removeCard = useCallback(async (repoPath: string) => {
    try {
      const newState = await api.removeKanbanCard(repoPath);
      setKanbanState(newState);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const syncWithScans = useCallback(async () => {
    if (allRepoPaths.length > 0) {
      try {
        const newState = await api.syncKanbanWithRepos(allRepoPaths);
        setKanbanState(newState);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    }
  }, [allRepoPaths]);

  return {
    columns,
    isLoading,
    error,
    moveCard,
    updateNotes,
    removeCard,
    syncWithScans,
  };
}
