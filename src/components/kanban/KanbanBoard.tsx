import { useRef } from 'react';
import { useKanban } from '../../hooks/useKanban';
import { KanbanColumn } from './KanbanColumn';
import { KANBAN_COLUMNS, ColumnId } from '../../config/kanbanColumns';
import { ScanResult, MonitoredFolder } from '../../types';

interface KanbanBoardProps {
  scanResults: Record<string, ScanResult>;
  folders: MonitoredFolder[];
}

export function KanbanBoard({ scanResults, folders }: KanbanBoardProps) {
  const { columns, isLoading, moveCard, updateNotes, removeCard } = useKanban({
    scanResults,
    folders,
  });

  // Use ref to store dragged item - dataTransfer.getData() returns empty in Tauri builds
  const draggingItemRef = useRef<string | null>(null);

  const handleDrop = (toColumn: ColumnId) => {
    const repoPath = draggingItemRef.current;
    draggingItemRef.current = null;
    if (!repoPath) return;

    const currentColumn = Object.entries(columns).find(([, cards]) =>
      cards.some((c) => c.card.repoPath === repoPath)
    )?.[0] as ColumnId | undefined;

    if (currentColumn && currentColumn !== toColumn) {
      moveCard(repoPath, toColumn);
    }
  };

  const hasRepos = Object.values(scanResults).some(
    (result) =>
      result.clean.length > 0 ||
      result.withChanges.length > 0 ||
      result.withUnpushed.length > 0 ||
      result.withUnpulled.length > 0 ||
      result.errors.length > 0
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary">
        <div className="text-sm">Loading...</div>
      </div>
    );
  }

  if (!hasRepos) {
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
              <rect x="3" y="3" width="5" height="18" rx="1" />
              <rect x="10" y="3" width="5" height="12" rx="1" />
              <rect x="17" y="3" width="5" height="15" rx="1" />
            </svg>
          </div>
          <h3 className="text-text-primary text-sm font-medium mb-2">No repositories found</h3>
          <p className="text-text-secondary text-xs">
            Scan your folders to see repositories in the kanban board.
            <br />
            Go to Folders view and click "Scan All".
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex gap-4 p-4 h-full">
      {KANBAN_COLUMNS.map((column) => (
        <KanbanColumn
          key={column.id}
          column={column}
          cards={columns[column.id]}
          onUpdateNotes={updateNotes}
          onRemoveCard={removeCard}
          onDragStart={(path) => (draggingItemRef.current = path)}
          onDragEnd={() => (draggingItemRef.current = null)}
          onDrop={() => handleDrop(column.id)}
        />
      ))}
    </div>
  );
}
