import { ColumnId } from '../config/kanbanColumns';

export interface GhRepo {
  nameWithOwner: string;
  name: string;
  owner: { login: string };
  description: string | null;
  url: string;
  isPrivate: boolean;
  isArchived: boolean;
  pushedAt: string | null;
}

export interface KanbanCard {
  nameWithOwner: string;
  column: ColumnId;
  createdAt: number;
  updatedAt: number;
}

export interface KanbanState {
  version: number;
  cards: Record<string, KanbanCard>;
}

export type SyncStatus = 'disabled' | 'synced' | 'offline' | 'expired';

export interface KanbanRefresh {
  repos: GhRepo[];
  state: KanbanState;
  syncStatus: SyncStatus;
}

export type GhAuthStatus =
  | { status: 'ok'; user: string }
  | { status: 'notInstalled' }
  | { status: 'notAuthenticated' }
  | { status: 'error'; message: string };

export interface KanbanCardView {
  card: KanbanCard;
  repo: GhRepo;
}
