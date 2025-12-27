export interface MonitoredFolder {
  id: string;
  path: string;
  name: string;
}

export interface RepoStatus {
  path: string;
  branch?: string;
  hasChanges?: boolean;
  hasUnpushed?: boolean;
  hasUnpulled?: boolean;
  hasError: boolean;
  errorMessage?: string;
}

export interface ScanResult {
  scannedPath: string;
  totalRepositories: number;
  withChanges: RepoStatus[];
  withUnpushed: RepoStatus[];
  withUnpulled: RepoStatus[];
  clean: RepoStatus[];
  errors: RepoStatus[];
  executionTime: number;
}

export interface TerminalApp {
  id: string;
  name: string;
  displayName: string;
  path: string;
}

export interface EditorApp {
  id: string;
  name: string;
  displayName: string;
  path: string;
}

export interface AppSettings {
  defaultTerminal?: string;
  defaultEditor?: string;
}
