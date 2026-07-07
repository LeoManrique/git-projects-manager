import { useState, useRef } from 'react';
import { KanbanCard } from './KanbanCard';
import { ColumnConfig } from '../../config/kanbanColumns';
import { KanbanCardView } from '../../types';
import { colorStyles } from '../scan/colorStyles';

interface KanbanColumnProps {
  column: ColumnConfig;
  cards: KanbanCardView[];
  authedUser: string | null;
  onDragStart: (nameWithOwner: string) => void;
  onDragEnd: () => void;
  onDrop: () => void;
  onDeleteRepo: (nameWithOwner: string) => void;
}

export function KanbanColumn({
  column,
  cards,
  authedUser,
  onDragStart,
  onDragEnd,
  onDrop,
  onDeleteRepo,
}: KanbanColumnProps) {
  const [isOver, setIsOver] = useState(false);
  const dragCounter = useRef(0);
  const styles = colorStyles[column.color];

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    dragCounter.current++;
    if (dragCounter.current === 1) setIsOver(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    dragCounter.current--;
    if (dragCounter.current === 0) setIsOver(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    dragCounter.current = 0;
    setIsOver(false);
    onDrop();
  };

  return (
    <div
      onDragEnter={handleDragEnter}
      onDragOver={(e) => e.preventDefault()}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`
        flex flex-col flex-1 min-w-[220px] rounded-xl border
        transition-all duration-150
        ${
          isOver
            ? `${styles.tint} ${styles.border} ring-2 ${styles.ring}`
            : 'bg-dark-surface/60 border-dark-border'
        }
      `}
    >
      <div className="px-3 pt-2.5 pb-2 flex items-center gap-2">
        <span className={`w-2 h-2 rounded-full shrink-0 ${styles.dot}`} />
        <span className="text-text-primary font-semibold text-sm truncate">{column.label}</span>
        <span className="flex-1" />
        <span className={`rounded-full px-2 py-0.5 font-semibold text-xs ${styles.badge}`}>
          {cards.length}
        </span>
      </div>

      <div className="flex-1 p-2.5 space-y-2 overflow-y-auto">
        {cards.length === 0 ? (
          <div className="text-text-muted text-xs py-6 text-center">No projects</div>
        ) : (
          cards.map((cardView) => (
            <KanbanCard
              key={cardView.card.nameWithOwner}
              cardView={cardView}
              authedUser={authedUser}
              onDragStart={onDragStart}
              onDragEnd={onDragEnd}
              onDeleteRepo={onDeleteRepo}
            />
          ))
        )}
      </div>
    </div>
  );
}
