export interface MonitoredFolder {
  id: string;
  path: string;
  name: string;
  onlyLocalChecks: boolean;
  /**
   * Whether the folder is expected to hold code projects, and so whether
   * sub-folders with files but no git repo are reported as Uninitialized
   * (FRONTEND.md §4). Off for general-purpose folders, where every ordinary
   * directory would otherwise be a finding.
   */
  detectUninitialized: boolean;
}

/** The editable part of a monitored folder: what the add/edit form collects. */
export type FolderFormValues = Omit<MonitoredFolder, 'id'>;

/** A new folder's starting point: a projects folder that checks its remotes. */
export const NEW_FOLDER: FolderFormValues = {
  path: '',
  name: '',
  onlyLocalChecks: false,
  detectUninitialized: true,
};
