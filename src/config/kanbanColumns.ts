import { ColorVariant } from '../components/scan/colorStyles';

export type ColumnId = 'backlog' | 'active-low' | 'active-mid' | 'active-high' | 'review' | 'done';

export interface ColumnConfig {
  id: ColumnId;
  label: string;
  color: ColorVariant;
}

export const KANBAN_COLUMNS: ColumnConfig[] = [
  { id: 'backlog', label: 'Backlog', color: 'gray' },
  { id: 'active-low', label: 'Active - Low Prio.', color: 'blue' },
  { id: 'active-mid', label: 'Active - Mid Prio.', color: 'yellow' },
  { id: 'active-high', label: 'Active - High Prio.', color: 'red' },
  { id: 'review', label: 'Review', color: 'yellow' },
  { id: 'done', label: 'Done', color: 'green' },
];
