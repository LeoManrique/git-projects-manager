import { ColorVariant } from '../components/scan/colorStyles';

export type ColumnId = 'backlog' | 'active-low' | 'active-high' | 'done' | 'closed';

export interface ColumnConfig {
  id: ColumnId;
  label: string;
  color: ColorVariant;
}

export const KANBAN_COLUMNS: ColumnConfig[] = [
  { id: 'backlog', label: 'Backlog', color: 'gray' },
  { id: 'active-low', label: 'Active · Low', color: 'blue' },
  { id: 'active-high', label: 'Active · High', color: 'red' },
  { id: 'done', label: 'Done', color: 'green' },
  { id: 'closed', label: 'Closed', color: 'yellow' },
];
