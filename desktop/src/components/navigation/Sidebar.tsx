import { MonitoredFolder, ScanResult } from '../../types';
import { attentionCount } from '../../lib/repoUtils';
import { FolderIcon, GearIcon, GridIcon, KanbanIcon, PlusCircleIcon } from '../icons';

/** What the main content area shows, driven by the sidebar. */
export type Selection =
  | { view: 'all' }
  | { view: 'kanban' }
  | { view: 'folder'; folderId: string };

interface SidebarProps {
  folders: MonitoredFolder[];
  results: Record<string, ScanResult>;
  scanningFolders: Set<string>;
  selection: Selection;
  onSelect: (selection: Selection) => void;
  onAddFolder: () => void;
  onOpenSettings: () => void;
}

/**
 * Navigation sidebar mirroring the macOS app: All Folders and Kanban views on
 * top, the monitored folders with attention badges below, and Add Folder /
 * Settings pinned at the bottom (FRONTEND.md §9).
 */
export function Sidebar({
  folders,
  results,
  scanningFolders,
  selection,
  onSelect,
  onAddFolder,
  onOpenSettings,
}: SidebarProps) {
  return (
    <aside className="w-[220px] flex-shrink-0 h-full flex flex-col bg-dark-surface/60 border-r border-dark-border">
      <nav className="p-2 space-y-0.5">
        <SidebarItem
          icon={<GridIcon />}
          label="All Folders"
          selected={selection.view === 'all'}
          onClick={() => onSelect({ view: 'all' })}
        />
        <SidebarItem
          icon={<KanbanIcon />}
          label="Kanban"
          selected={selection.view === 'kanban'}
          onClick={() => onSelect({ view: 'kanban' })}
        />
      </nav>

      <p className="px-4 pt-2 pb-1 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
        Folders
      </p>
      <nav className="flex-1 overflow-y-auto px-2 space-y-0.5">
        {folders.map((folder) => (
          <SidebarItem
            key={folder.id}
            icon={<FolderIcon className="w-4 h-4" />}
            label={folder.name}
            selected={selection.view === 'folder' && selection.folderId === folder.id}
            onClick={() => onSelect({ view: 'folder', folderId: folder.id })}
            trailing={
              scanningFolders.has(folder.id) ? (
                <span className="w-3 h-3 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
              ) : (
                folderBadge(attentionCount(results[folder.id]))
              )
            }
          />
        ))}
        {folders.length === 0 && (
          <p className="px-2 py-1 text-xs text-text-muted">No folders configured yet.</p>
        )}
      </nav>

      <footer className="flex-shrink-0 flex items-center justify-between p-2 border-t border-dark-border/50">
        <button
          onClick={onAddFolder}
          className="flex items-center gap-1.5 px-2 py-1.5 rounded text-xs text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle transition-colors"
        >
          <PlusCircleIcon />
          Add Folder
        </button>
        <button
          onClick={onOpenSettings}
          className="p-1.5 rounded text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle transition-colors"
          title="Settings"
        >
          <GearIcon />
        </button>
      </footer>
    </aside>
  );
}

function folderBadge(count: number) {
  if (count === 0) return null;
  return (
    <span className="min-w-5 px-1.5 py-px rounded-full bg-dark-elevated text-text-secondary text-[11px] leading-4 font-medium text-center">
      {count}
    </span>
  );
}

interface SidebarItemProps {
  icon: React.ReactNode;
  label: string;
  selected: boolean;
  onClick: () => void;
  trailing?: React.ReactNode;
}

function SidebarItem({ icon, label, selected, onClick, trailing }: SidebarItemProps) {
  return (
    <button
      onClick={onClick}
      className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-left transition-colors ${
        selected
          ? 'bg-accent-blue text-white'
          : 'text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle'
      }`}
    >
      <span className={selected ? 'text-white' : 'text-text-muted'}>{icon}</span>
      <span className="flex-1 min-w-0 truncate text-xs font-medium">{label}</span>
      {trailing}
    </button>
  );
}
