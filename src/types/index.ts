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
  hasError: boolean;
  errorMessage?: string;
}

export interface ScanResult {
  scannedPath: string;
  totalRepositories: number;
  withChanges: RepoStatus[];
  withUnpushed: RepoStatus[];
  clean: RepoStatus[];
  errors: RepoStatus[];
  executionTime: number;
}
