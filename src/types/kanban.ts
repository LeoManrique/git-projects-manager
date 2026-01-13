import { ColumnId } from '../config/kanbanColumns';

export interface KanbanCard {
  repoPath: string;
  column: ColumnId;
  notes?: string;
  createdAt: number;
  updatedAt: number;
}

export interface KanbanState {
  version: number;
  cards: Record<string, KanbanCard>;
}

export interface KanbanCardView {
  card: KanbanCard;
  repoName: string;
  displayName: string;
  folderName: string;
  folderId: string;
  isStale: boolean;
}
