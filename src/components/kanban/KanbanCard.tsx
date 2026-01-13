import { useState } from 'react';
import { KanbanCardView } from '../../types';

interface KanbanCardProps {
  cardView: KanbanCardView;
  onUpdateNotes: (repoPath: string, notes: string | null) => void;
  onRemoveCard: (repoPath: string) => void;
  onDragStart: (repoPath: string) => void;
  onDragEnd: () => void;
}

export function KanbanCard({
  cardView,
  onUpdateNotes,
  onRemoveCard,
  onDragStart,
  onDragEnd,
}: KanbanCardProps) {
  const { card, displayName, isStale } = cardView;
  const [isEditingNotes, setIsEditingNotes] = useState(false);
  const [notesValue, setNotesValue] = useState(card.notes || '');
  const [isDragging, setIsDragging] = useState(false);

  const handleDragStart = (e: React.DragEvent) => {
    e.dataTransfer.effectAllowed = 'move';
    onDragStart(card.repoPath);
    setIsDragging(true);
  };

  const handleDragEnd = () => {
    setIsDragging(false);
    onDragEnd();
  };

  const handleNotesBlur = () => {
    setIsEditingNotes(false);
    const newNotes = notesValue.trim() || null;
    if (newNotes !== card.notes) {
      onUpdateNotes(card.repoPath, newNotes);
    }
  };

  const handleNotesKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      handleNotesBlur();
    } else if (e.key === 'Escape') {
      setNotesValue(card.notes || '');
      setIsEditingNotes(false);
    }
  };

  return (
    <div
      draggable={!isStale && !isEditingNotes}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      className={`
        rounded border bg-dark-elevated
        ${!isStale && !isEditingNotes ? 'cursor-grab active:cursor-grabbing' : ''}
        ${isStale ? 'border-accent-red/50 opacity-60' : 'border-dark-border'}
        ${isDragging ? 'opacity-50 shadow-lg' : ''}
        transition-opacity duration-150
      `}
    >
      {/* Header */}
      <div className="px-3 py-2 flex items-center gap-2">
        <span className="text-text-muted text-sm">
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
        </span>
        <span
          className={`text-sm truncate flex-1 ${isStale ? 'text-text-muted line-through' : 'text-text-primary'}`}
          title={card.repoPath}
        >
          {displayName}
        </span>
      </div>

      {/* Stale indicator */}
      {isStale && (
        <div className="px-3 pb-2 flex items-center justify-between">
          <span className="text-xs text-accent-red">Not found</span>
          <button
            onClick={() => onRemoveCard(card.repoPath)}
            className="text-xs text-accent-red hover:text-accent-red/80 transition-colors"
          >
            Remove
          </button>
        </div>
      )}

      {/* Notes */}
      {!isStale && (
        <div className="px-3 pb-2">
          {isEditingNotes ? (
            <textarea
              value={notesValue}
              onChange={(e) => setNotesValue(e.target.value)}
              onBlur={handleNotesBlur}
              onKeyDown={handleNotesKeyDown}
              autoFocus
              className="w-full text-[11px] bg-transparent text-text-secondary focus:outline-none resize-none overflow-y-auto max-h-[72px] caret-accent-blue"
              style={{ fieldSizing: 'content' } as React.CSSProperties}
              placeholder="Add notes..."
            />
          ) : (
            <button
              onClick={() => setIsEditingNotes(true)}
              className="w-full text-left text-[11px] text-text-muted hover:text-text-secondary transition-colors line-clamp-4 break-words whitespace-pre-wrap"
            >
              {card.notes || 'Add notes...'}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
