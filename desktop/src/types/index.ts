// Re-export all types from domain-specific files
export type { MonitoredFolder } from './folder';
export type { RepoStatus, ScanResult } from './scan';
export type { TerminalApp, EditorApp, AppSettings, GitCleanSettings, GitCleanResult } from './settings';
export type {
  GhRepo,
  GhAuthStatus,
  KanbanCard,
  KanbanState,
  KanbanRefresh,
  KanbanCardView,
  SyncStatus,
} from './kanban';
export type { SyncUser } from './auth';
