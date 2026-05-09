import { useState, useRef } from 'react';
import { KanbanCard } from './KanbanCard';
import { ColumnConfig } from '../../config/kanbanColumns';
import { KanbanCardView } from '../../types';
import { colorStyles } from '../scan/colorStyles';

interface KanbanColumnProps {
  column: ColumnConfig;
  cards: KanbanCardView[];
  onDragStart: (nameWithOwner: string) => void;
  onDragEnd: () => void;
  onDrop: () => void;
}

export function KanbanColumn({
  column,
  cards,
  onDragStart,
  onDragEnd,
  onDrop,
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
        flex flex-col flex-1 min-w-[200px] rounded-lg
        bg-dark-surface border border-dark-border
        transition-all duration-150
        ${isOver ? 'ring-2 ring-accent-blue bg-dark-elevated/50' : ''}
      `}
    >
      <div className={`px-3 py-2 border-b ${styles.border}`}>
        <div className="flex items-center justify-between">
          <span className={`font-medium text-sm ${styles.text}`}>{column.label}</span>
          <span className={`text-xs px-2 py-0.5 rounded ${styles.badge}`}>{cards.length}</span>
        </div>
      </div>

      <div className="flex-1 p-2 space-y-2 overflow-y-auto">
        {cards.length === 0 ? (
          <div className="text-center text-text-muted text-xs py-4">No projects</div>
        ) : (
          cards.map((cardView) => (
            <KanbanCard
              key={cardView.card.nameWithOwner}
              cardView={cardView}
              onDragStart={onDragStart}
              onDragEnd={onDragEnd}
            />
          ))
        )}
      </div>
    </div>
  );
}
