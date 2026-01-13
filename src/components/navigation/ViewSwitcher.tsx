import { FolderIcon } from '../icons';

export type View = 'folders' | 'kanban';

interface ViewSwitcherProps {
  currentView: View;
  onViewChange: (view: View) => void;
  variant?: 'header' | 'sidebar';
}

function KanbanIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <rect x="3" y="3" width="5" height="18" rx="1" />
      <rect x="10" y="3" width="5" height="12" rx="1" />
      <rect x="17" y="3" width="5" height="15" rx="1" />
    </svg>
  );
}

export function ViewSwitcher({ currentView, onViewChange, variant = 'header' }: ViewSwitcherProps) {
  const baseButtonClass = `
    p-1.5 rounded transition-colors
    hover:bg-dark-elevated
  `;

  const activeClass = 'bg-dark-elevated text-accent-blue';
  const inactiveClass = 'text-text-secondary';

  if (variant === 'header') {
    return (
      <div className="flex items-center gap-0.5 bg-dark-surface rounded-md p-0.5">
        <button
          onClick={() => onViewChange('folders')}
          className={`${baseButtonClass} ${currentView === 'folders' ? activeClass : inactiveClass}`}
          title="Folders"
        >
          <FolderIcon className="w-4 h-4" />
        </button>
        <button
          onClick={() => onViewChange('kanban')}
          className={`${baseButtonClass} ${currentView === 'kanban' ? activeClass : inactiveClass}`}
          title="Kanban"
        >
          <KanbanIcon className="w-4 h-4" />
        </button>
      </div>
    );
  }

  // Sidebar variant (for future use)
  return (
    <div className="flex flex-col gap-1 p-2">
      <button
        onClick={() => onViewChange('folders')}
        className={`${baseButtonClass} ${currentView === 'folders' ? activeClass : inactiveClass}`}
        title="Folders"
      >
        <FolderIcon className="w-5 h-5" />
      </button>
      <button
        onClick={() => onViewChange('kanban')}
        className={`${baseButtonClass} ${currentView === 'kanban' ? activeClass : inactiveClass}`}
        title="Kanban"
      >
        <KanbanIcon className="w-5 h-5" />
      </button>
    </div>
  );
}
